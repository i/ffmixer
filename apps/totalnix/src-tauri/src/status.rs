//! Status thread: once a second, reads the unit's clock and input lock
//! status (read-only) and sends it to the UI as a `device-status` event.
//!
//! It is bound to the `ffado-dbus-server` that is running, never the
//! service name: D-Bus would start a new server if that one exited, and
//! a new server reloads the flash mix onto the unit.

use std::thread;
use std::time::Duration;

use ffado_dbus::FfadoReader;
use ffado_dbus::settings::ClockStatus;
use tauri::{AppHandle, Emitter};

use crate::SharedSession;
use crate::session::Mode;

pub const EVENT: &str = "device-status";
const POLL_EVERY: Duration = Duration::from_secs(1);

pub fn spawn(
    app: AppHandle,
    session: SharedSession,
) {
    thread::Builder::new()
        .name("status".to_owned())
        .spawn(move || {
            let mut reader: Option<FfadoReader> = None;
            loop {
                thread::sleep(POLL_EVERY);
                let guid = {
                    let session = session.lock().expect("session lock");
                    (session.mode == Mode::Hardware).then(|| session.device.clone())
                };
                let status = guid.and_then(|guid| read(&mut reader, &guid));
                // Sent every time, so a UI that attaches late still gets it.
                let _ = app.emit(EVENT, status);
            }
        })
        .expect("spawn status thread");
}

/// `None` if the server is not running. Reconnects on the next call.
fn read(
    reader: &mut Option<FfadoReader>,
    guid: &str,
) -> Option<ClockStatus> {
    if reader.is_none() {
        *reader = FfadoReader::connect_running().ok();
    }
    match reader.as_ref()?.clock_status(guid) {
        Ok(status) => Some(status),
        Err(_) => {
            *reader = None;
            None
        }
    }
}
