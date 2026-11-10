//! The app's connection to `ffado-dbus-server`: reading the unit's mix,
//! and the write link used while live.

use ffado_dbus::writer::{FaderMatrix, FfadoWriter, GainWrite};
use ffado_dbus::{FfadoReader, Result as DbusResult};
use mixer_core::writes::{Matrix, Write};
use mixer_core::{CHANNELS, Crosspoints};

use crate::session::GainSink;

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
    Ok(LoadedDevice { guid, crosspoints })
}

/// Fresh read of the unit's mix, e.g. for "Reload from unit".
pub fn read_mix(guid: &str) -> Result<Crosspoints, String> {
    read_crosspoints(&connect_reader()?, guid)
}

/// Binds a writer to the running server and reads the unit's mix through
/// it, after checking FFADO's flags are clear.
pub fn open_link(guid: &str) -> Result<(FfadoLink, Crosspoints), String> {
    let writer =
        FfadoWriter::connect(guid).map_err(|e| format!("cannot reach ffado-dbus-server ({e})"))?;
    let reader = connect_reader()?;

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
    let link = FfadoLink { writer };
    if !link.server_unchanged()? {
        return Err("ffado-dbus-server restarted while connecting; try again".to_owned());
    }
    Ok((link, crosspoints))
}

pub struct FfadoLink {
    writer: FfadoWriter,
}

impl GainSink for FfadoLink {
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
        let (link, unit) = open_link(&device.guid).unwrap();
        assert!(link.server_unchanged().unwrap());
        assert_eq!(unit, device.crosspoints);
    }
}
