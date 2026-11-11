//! The app's connection to `ffado-dbus-server`: reading the unit's mix,
//! and the write link used while live.

use ffado_dbus::settings::{DeviceSettings, Setting, confirm_applied};
use ffado_dbus::writer::{FaderMatrix, FfadoWriter, GainWrite};
use ffado_dbus::{FfadoReader, Result as DbusResult};
use mixer_core::writes::{Matrix, Write};
use mixer_core::{CHANNELS, Crosspoints};

use crate::session::UnitLink;

/// FFADO's own per-crosspoint flags. It applies them to every value we
/// write (mute forces 0, invert flips the sign), so they must all be
/// clear before going live.
const FLAG_MATRICES: [&str; 5] = [
    "Mixer/InputMutes",
    "Mixer/PlaybackMutes",
    "Mixer/OutputMutes",
    "Mixer/InputInverts",
    "Mixer/PlaybackInverts",
];

pub struct LoadedDevice {
    pub guid: String,
    pub crosspoints: Crosspoints,
    /// `None` if FFADO's settings could not be read; the mix still loads.
    pub settings: Option<DeviceSettings>,
}

pub fn load_first_device() -> Result<LoadedDevice, String> {
    let reader = connect_reader()?;
    let guid = reader
        .device_guids()
        .map_err(|e| format!("cannot list devices ({e})"))?
        .into_iter()
        .next()
        .ok_or_else(|| "ffado-dbus-server reports no devices".to_owned())?;
    let crosspoints = read_crosspoints(&reader, &guid)?;
    let settings = reader.device_settings(&guid).ok();
    Ok(LoadedDevice {
        guid,
        crosspoints,
        settings,
    })
}

/// Fresh read of the unit's mix, e.g. for "Reload from unit".
pub fn read_mix(guid: &str) -> Result<Crosspoints, String> {
    read_crosspoints(&connect_reader()?, guid)
}

/// Fresh read of the unit's device settings, if FFADO reports them.
pub fn read_device_settings(guid: &str) -> Option<DeviceSettings> {
    connect_reader().ok()?.device_settings(guid).ok()
}

/// What going live starts from: the link, plus a fresh read of the unit.
pub struct OpenedLink {
    pub link: FfadoLink,
    pub crosspoints: Crosspoints,
    pub settings: Option<DeviceSettings>,
}

/// Binds a writer to the running server and reads the unit's mix through
/// it, after checking FFADO's flags are clear.
pub fn open_link(guid: &str) -> Result<OpenedLink, String> {
    let writer =
        FfadoWriter::connect(guid).map_err(|e| format!("cannot reach ffado-dbus-server ({e})"))?;
    // Bound to the running server, so a restarted one cannot answer.
    let reader = FfadoReader::connect_running()
        .map_err(|e| format!("cannot reach ffado-dbus-server ({e})"))?;

    let mut flagged = 0;
    for control in FLAG_MATRICES {
        let snapshot = reader
            .matrix(guid, control)
            .map_err(|e| format!("cannot read {control} ({e})"))?;
        flagged += snapshot
            .values
            .iter()
            .flatten()
            .filter(|v| **v != 0.0)
            .count();
    }
    if flagged > 0 {
        return Err(format!(
            "FFADO has {flagged} mute/invert flags set (perhaps from ffado-mixer). They would \
             change what totalnix sends. Clear them in ffado-mixer, or restart ffado-dbus-server."
        ));
    }

    let crosspoints = read_crosspoints(&reader, guid)?;
    let settings = reader.device_settings(guid).ok();
    let link = FfadoLink {
        writer,
        reader,
        guid: guid.to_owned(),
    };
    if !link.server_unchanged()? {
        return Err("ffado-dbus-server restarted while connecting; try again".to_owned());
    }
    Ok(OpenedLink {
        link,
        crosspoints,
        settings,
    })
}

pub struct FfadoLink {
    writer: FfadoWriter,
    /// Reads settings back after each write to confirm it.
    reader: FfadoReader,
    guid: String,
}

impl UnitLink for FfadoLink {
    fn apply_setting(
        &self,
        setting: &Setting,
        current: &DeviceSettings,
    ) -> Result<DeviceSettings, String> {
        self.writer
            .set_setting(setting, current)
            .map_err(|e| e.to_string())?;
        let after = self
            .reader
            .device_settings(&self.guid)
            .map_err(|e| format!("cannot read the settings back ({e})"))?;
        confirm_applied(setting, current, after).map_err(|e| e.to_string())
    }

    fn set_gain(
        &self,
        write: &Write,
    ) -> Result<(), String> {
        let matrix = match write.matrix {
            Matrix::Inputs => FaderMatrix::Inputs,
            Matrix::Playbacks => FaderMatrix::Playbacks,
            Matrix::Outputs => FaderMatrix::Outputs,
        };
        self.writer
            .set_gain(&GainWrite {
                matrix,
                row: write.row,
                col: write.col,
                value: write.value,
            })
            .map_err(|e| e.to_string())
    }

    fn server_unchanged(&self) -> Result<bool, String> {
        self.writer.server_unchanged().map_err(|e| e.to_string())
    }
}

fn connect_reader() -> Result<FfadoReader, String> {
    FfadoReader::connect().map_err(|e| format!("cannot reach ffado-dbus-server ({e})"))
}

fn read_crosspoints(
    reader: &FfadoReader,
    guid: &str,
) -> Result<Crosspoints, String> {
    let matrix =
        |name: &str| -> DbusResult<Vec<Vec<f64>>> { Ok(reader.matrix(guid, name)?.values) };
    let read = |name: &str| matrix(name).map_err(|e| format!("cannot read {name} ({e})"));
    let inputs = read(FaderMatrix::Inputs.control())?;
    let playbacks = read(FaderMatrix::Playbacks.control())?;
    let outputs = read(FaderMatrix::Outputs.control())?
        .into_iter()
        .next()
        .unwrap_or_default();

    let square = |m: &Vec<Vec<f64>>| m.len() == CHANNELS && m.iter().all(|r| r.len() == CHANNELS);
    if !square(&inputs) || !square(&playbacks) || outputs.len() != CHANNELS {
        return Err(format!(
            "unexpected mixer size from FFADO; expected {CHANNELS}×{CHANNELS} (is this a Fireface 800?)"
        ));
    }
    Ok(Crosspoints {
        inputs,
        playbacks,
        outputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a running `ffado-dbus-server` and unit. Reads only:
    /// `cargo test -p totalnix -- --ignored`.
    #[test]
    #[ignore]
    fn open_link_against_running_server() {
        let device = load_first_device().unwrap();
        let opened = open_link(&device.guid).unwrap();
        assert!(opened.link.server_unchanged().unwrap());
        assert_eq!(opened.crosspoints, device.crosspoints);
        assert_eq!(opened.settings, device.settings);
    }

    /// Reads the settings and clock status through a reader bound to the
    /// running server. Reads only.
    #[test]
    #[ignore]
    fn reads_settings_and_status_from_running_server() {
        let device = load_first_device().unwrap();
        let reader = FfadoReader::connect_running().unwrap();
        let settings = reader.device_settings(&device.guid).unwrap();
        let status = reader.clock_status(&device.guid).unwrap();
        println!("{settings:#?}\n{status:#?}");
        assert_eq!(settings.phantom[3], None);
        assert!(status.sample_rate >= 32000);
    }

    /// **Writes to the unit**, but changes nothing: sends the setting's
    /// current value (S/PDIF emphasis) back, so FFADO rewrites the same
    /// configuration registers. Checks the write is read back and
    /// confirmed, and that the clock is undisturbed.
    /// `cargo test -p totalnix -- --ignored identical_value_write`.
    #[test]
    #[ignore]
    fn identical_value_write_round_trips() {
        let device = load_first_device().unwrap();
        let before = device.settings.unwrap();
        let reader = FfadoReader::connect_running().unwrap();
        let clock_before = reader.clock_status(&device.guid).unwrap();

        let opened = open_link(&device.guid).unwrap();
        let same = Setting::Emphasis {
            on: before.emphasis,
        };
        assert_eq!(same.risk(&before), None);
        let after = opened.link.apply_setting(&same, &before).unwrap();

        assert_eq!(after, before);
        let clock_after = reader.clock_status(&device.guid).unwrap();
        assert_eq!(clock_after.sample_rate, clock_before.sample_rate);
        assert_eq!(clock_after.master, clock_before.master);
    }
}
