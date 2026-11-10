//! Reads the unit's current mix from `ffado-dbus-server` (read-only).

use ffado_dbus::FfadoReader;
use mixer_core::{CHANNELS, Crosspoints};

pub struct LoadedDevice {
    pub guid: String,
    pub crosspoints: Crosspoints,
}

pub fn load_first_device() -> Result<LoadedDevice, String> {
    let reader =
        FfadoReader::connect().map_err(|e| format!("cannot reach ffado-dbus-server ({e})"))?;
    let guid = reader
        .device_guids()
        .map_err(|e| format!("cannot list devices ({e})"))?
        .into_iter()
        .next()
        .ok_or_else(|| "ffado-dbus-server reports no devices".to_owned())?;

    let matrix = |name: &str| {
        let snapshot = reader
            .matrix(&guid, name)
            .map_err(|e| format!("cannot read {name} ({e})"))?;
        Ok::<_, String>(snapshot.values)
    };
    let inputs = matrix("Mixer/InputFaders")?;
    let playbacks = matrix("Mixer/PlaybackFaders")?;
    let outputs = matrix("Mixer/OutputFaders")?
        .into_iter()
        .next()
        .unwrap_or_default();

    let square = |m: &Vec<Vec<f64>>| m.len() == CHANNELS && m.iter().all(|r| r.len() == CHANNELS);
    if !square(&inputs) || !square(&playbacks) || outputs.len() != CHANNELS {
        return Err(format!(
            "unexpected mixer size from FFADO; expected {CHANNELS}×{CHANNELS} (is this a Fireface 800?)"
        ));
    }

    Ok(LoadedDevice {
        guid,
        crosspoints: Crosspoints {
            inputs,
            playbacks,
            outputs,
        },
    })
}
