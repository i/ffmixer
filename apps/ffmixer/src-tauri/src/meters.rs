//! Meter thread: about 30 readings a second, sent to the UI as one
//! `meters` event per reading.
//!
//! With a unit, levels come from the hardware (read-only block reads via
//! `fw-meter`). In demo mode they are simulated from the current mix so
//! faders visibly affect the output meters.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fw_meter::{Ff800Meters, MeterFrame, level_to_dbfs};
use mixer_core::{CHANNELS, law};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::session::{Mode, Session};

pub const EVENT: &str = "meters";
const INTERVAL: Duration = Duration::from_millis(33);
const RETRY_DELAY: Duration = Duration::from_secs(1);
/// Sent for silence so the UI never has to handle −∞.
const SILENCE_DBFS: f32 = -150.0;

/// dBFS per channel for each row.
#[derive(Clone, Serialize)]
pub struct MeterEvent {
    pub inputs: Vec<f32>,
    pub playbacks: Vec<f32>,
    pub outputs: Vec<f32>,
}

pub fn spawn(
    app: AppHandle,
    session: Arc<Mutex<Session>>,
) {
    thread::Builder::new()
        .name("meters".to_owned())
        .spawn(move || run(&app, &session))
        .expect("spawn meter thread");
}

fn run(
    app: &AppHandle,
    session: &Mutex<Session>,
) {
    let started = Instant::now();
    let mut hardware: Option<Ff800Meters> = None;
    loop {
        let (mode, guid) = {
            let session = session.lock().expect("session lock");
            (session.mode, session.device.clone())
        };
        let event = match mode {
            Mode::Loading => None,
            Mode::Demo => Some(simulate(session, started.elapsed())),
            Mode::Hardware => read_hardware(&mut hardware, &guid),
        };
        match event {
            Some(event) => {
                // The window may be closing; a failed emit is harmless.
                let _ = app.emit(EVENT, event);
                thread::sleep(INTERVAL);
            }
            None => thread::sleep(if mode == Mode::Hardware {
                RETRY_DELAY
            } else {
                INTERVAL
            }),
        }
    }
}

fn read_hardware(
    meters: &mut Option<Ff800Meters>,
    guid: &str,
) -> Option<MeterEvent> {
    if meters.is_none() {
        match Ff800Meters::open_by_guid(guid) {
            Ok(opened) => *meters = Some(opened),
            Err(e) => {
                eprintln!("meters: cannot open device {guid}: {e}");
                return None;
            }
        }
    }
    match meters.as_mut()?.read() {
        Ok(frame) => Some(from_frame(&frame)),
        Err(e) => {
            eprintln!("meters: read failed, reopening: {e}");
            *meters = None;
            None
        }
    }
}

fn from_frame(frame: &MeterFrame) -> MeterEvent {
    let convert = |levels: &[u32]| {
        levels
            .iter()
            .map(|level| level_to_dbfs(*level).map_or(SILENCE_DBFS, |db| db as f32))
            .collect()
    };
    MeterEvent {
        inputs: convert(&frame.inputs),
        playbacks: convert(&frame.playbacks),
        outputs: convert(&frame.outputs),
    }
}

/// Demo levels: a few inputs and playbacks carry moving signals; outputs
/// are the mix of those through the current crosspoints.
fn simulate(
    session: &Mutex<Session>,
    elapsed: Duration,
) -> MeterEvent {
    let time = elapsed.as_secs_f64();
    let wobble = |channel: usize, base_db: f64| {
        let phase = channel as f64 * 1.7;
        base_db + 6.0 * (time * 0.9 + phase).sin() + 3.0 * (time * 5.3 + phase).sin()
    };
    let inputs: Vec<f64> = (0..CHANNELS)
        .map(|c| match c {
            0..=1 => law::db_to_gain(Some(wobble(c, -14.0))),
            6..=7 => law::db_to_gain(Some(wobble(c, -20.0))),
            _ => 0.0,
        })
        .collect();
    let playbacks: Vec<f64> = (0..CHANNELS)
        .map(|c| {
            if c < 4 {
                law::db_to_gain(Some(wobble(c + 3, -12.0)))
            } else {
                0.0
            }
        })
        .collect();

    let crosspoints = session.lock().expect("session lock").mixer.render();
    let outputs: Vec<f64> = (0..CHANNELS)
        .map(|out| {
            let sum: f64 = (0..CHANNELS)
                .map(|src| {
                    inputs[src] * law::from_dbus(crosspoints.inputs[out][src])
                        + playbacks[src] * law::from_dbus(crosspoints.playbacks[out][src])
                })
                .sum();
            (sum * law::from_dbus(crosspoints.outputs[out])).min(1.0)
        })
        .collect();

    let to_db = |levels: &[f64]| {
        levels
            .iter()
            .map(|gain| law::gain_to_db(*gain).map_or(SILENCE_DBFS, |db| db.min(0.0) as f32))
            .collect()
    };
    MeterEvent {
        inputs: to_db(&inputs),
        playbacks: to_db(&playbacks),
        outputs: to_db(&outputs),
    }
}
