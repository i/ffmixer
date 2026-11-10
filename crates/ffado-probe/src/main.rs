//! Read-only probe: prints what `ffado-dbus-server` reports for each
//! device, then samples the Fireface 800 hardware meters.
//!
//! Usage: `ffado-probe [METER_SECONDS]` (default 5; 0 skips meters).
//! Nothing here writes to the device.

use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use ffado_dbus::{FfadoReader, MatrixSnapshot, gain_to_db};
use fw_meter::{Ff800Meters, MeterFrame, level_to_dbfs};

const DISCRETE_CONTROLS: &[&str] = &[
    "Control/Phantom",
    "Control/Input_level",
    "Control/Output_level",
    "Control/Phones_level",
    "Control/Chan1_source",
    "Control/Chan7_source",
    "Control/Chan8_source",
    "Control/Clock_mode",
    "Control/sysclock_mode",
    "Control/sysclock_freq",
];

const MATRICES: &[&str] = &[
    "Mixer/InputFaders",
    "Mixer/PlaybackFaders",
    "Mixer/OutputFaders",
];

const METER_INTERVAL: Duration = Duration::from_millis(100);
/// Channels quieter than this are left out of the meter printout.
const METER_FLOOR_DBFS: f64 = -90.0;

fn main() -> ExitCode {
    let meter_seconds = match parse_meter_seconds() {
        Ok(seconds) => seconds,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    let reader = match FfadoReader::connect() {
        Ok(reader) => reader,
        Err(e) => {
            eprintln!("cannot connect to ffado-dbus-server: {e}");
            return ExitCode::FAILURE;
        }
    };
    let guids = match reader.device_guids() {
        Ok(guids) if !guids.is_empty() => guids,
        Ok(_) => {
            eprintln!("ffado-dbus-server reports no devices");
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("cannot list devices: {e}");
            return ExitCode::FAILURE;
        }
    };

    for guid in &guids {
        println!("== Device {guid}");
        print_discrete_controls(&reader, guid);
        for matrix in MATRICES {
            print_matrix(&reader, guid, matrix);
        }
        if meter_seconds > 0 {
            print_meters(guid, meter_seconds);
        }
    }
    ExitCode::SUCCESS
}

fn parse_meter_seconds() -> Result<u64, String> {
    match std::env::args().nth(1) {
        None => Ok(5),
        Some(text) => text
            .parse::<u64>()
            .ok()
            .filter(|seconds| *seconds <= 600)
            .ok_or_else(|| format!("METER_SECONDS must be 0-600, got `{text}`")),
    }
}

fn print_discrete_controls(
    reader: &FfadoReader,
    guid: &str,
) {
    println!("-- Discrete controls");
    for control in DISCRETE_CONTROLS {
        match reader.discrete(guid, control) {
            Ok(value) => println!("{control:<24} {value:>8}  (0x{value:08x})"),
            Err(e) => println!("{control:<24} unavailable: {e}"),
        }
    }
}

fn print_matrix(
    reader: &FfadoReader,
    guid: &str,
    matrix: &str,
) {
    let snapshot = match reader.matrix(guid, matrix) {
        Ok(snapshot) => snapshot,
        Err(e) => {
            println!("-- {matrix}: unavailable: {e}");
            return;
        }
    };
    let rows = snapshot.row_names.len();
    let cols = snapshot.col_names.len();
    println!("-- {matrix} ({rows} rows x {cols} cols), non-silent crosspoints:");
    for (row, col, raw) in audible_crosspoints(&snapshot) {
        let db = gain_to_db(raw).map_or("-inf".to_owned(), |db| format!("{db:+.1} dB"));
        let phase = if raw < 0.0 { "  [phase inverted]" } else { "" };
        println!(
            "  {:<16} -> {:<16} raw {raw:>7}  {db}{phase}",
            label(&snapshot.col_names, col, "in"),
            label(&snapshot.row_names, row, "out"),
        );
    }
}

fn audible_crosspoints(snapshot: &MatrixSnapshot) -> Vec<(usize, usize, f64)> {
    let mut found = Vec::new();
    for (row, values) in snapshot.values.iter().enumerate() {
        for (col, raw) in values.iter().enumerate() {
            if *raw != 0.0 {
                found.push((row, col, *raw));
            }
        }
    }
    found
}

fn label(
    names: &[String],
    index: usize,
    fallback: &str,
) -> String {
    match names.get(index) {
        Some(name) if !name.is_empty() => name.clone(),
        _ => format!("{fallback} {}", index + 1),
    }
}

fn print_meters(
    guid: &str,
    seconds: u64,
) {
    println!("-- Meters for {seconds} s (peak dBFS per 100 ms; silent channels hidden)");
    let mut meters = match Ff800Meters::open_by_guid(guid) {
        Ok(meters) => meters,
        Err(e) => {
            println!("meters unavailable: {e}");
            return;
        }
    };

    let end = Instant::now() + Duration::from_secs(seconds);
    let mut failures = 0;
    while Instant::now() < end {
        match meters.read() {
            Ok(frame) => println!("{}", format_frame(&frame)),
            Err(e) => {
                failures += 1;
                println!("read failed: {e}");
                if failures >= 5 {
                    println!("giving up after {failures} failed reads");
                    return;
                }
            }
        }
        thread::sleep(METER_INTERVAL);
    }
}

fn format_frame(frame: &MeterFrame) -> String {
    let mut parts = Vec::new();
    append_levels(&mut parts, "in", &frame.inputs);
    append_levels(&mut parts, "pb", &frame.playbacks);
    append_levels(&mut parts, "out", &frame.outputs);
    if parts.is_empty() {
        "(silence)".to_owned()
    } else {
        parts.join("  ")
    }
}

fn append_levels(
    parts: &mut Vec<String>,
    prefix: &str,
    levels: &[u32],
) {
    for (index, level) in levels.iter().enumerate() {
        if let Some(db) = level_to_dbfs(*level).filter(|db| *db > METER_FLOOR_DBFS) {
            parts.push(format!("{prefix}{}:{db:.1}", index + 1));
        }
    }
}
