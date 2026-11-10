//! The only code in totalnix that changes anything on the unit.
//!
//! It can write exactly one thing: a gain in one of the three fader
//! matrices ([`FaderMatrix`]). Flash, phantom power, levels, clock and
//! FFADO's mute/invert flags are unreachable: the target is an enum, not
//! a path, and every value is range-checked before it leaves the process.
//!
//! Calls go to the server's *unique* bus name, captured on connect. If
//! `ffado-dbus-server` exits, writes fail instead of D-Bus starting a new
//! server, which would reload the flash mix onto the unit behind our back.

use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::names::{BusName, OwnedUniqueName};
use zbus::zvariant::OwnedObjectPath;

use crate::{Error, GAIN_MAX, IFACE_MATRIX, Result, SERVICE, device_path};

/// FF800 mixer size: 28 sources into 28 output channels.
pub const CHANNELS: usize = 28;

/// The single D-Bus method the writer calls.
pub const WRITE_METHOD: &str = "setValue";

/// A fader matrix the writer may change. Nothing else is writable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaderMatrix {
    /// Hardware input → output channel.
    Inputs,
    /// Software playback → output channel.
    Playbacks,
    /// Output channel volume (one row).
    Outputs,
}

impl FaderMatrix {
    pub const fn control(self) -> &'static str {
        match self {
            FaderMatrix::Inputs => "Mixer/InputFaders",
            FaderMatrix::Playbacks => "Mixer/PlaybackFaders",
            FaderMatrix::Outputs => "Mixer/OutputFaders",
        }
    }

    const fn rows(self) -> usize {
        match self {
            FaderMatrix::Outputs => 1,
            FaderMatrix::Inputs | FaderMatrix::Playbacks => CHANNELS,
        }
    }
}

/// One gain write. `row` is the output channel (0 for [`FaderMatrix::Outputs`]),
/// `col` the source channel or, for outputs, the output channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GainWrite {
    pub matrix: FaderMatrix,
    pub row: usize,
    pub col: usize,
    /// D-Bus scale: 0x4000 is 0 dB, negative is phase-inverted.
    pub value: f64,
}

/// Checks a write against the hardware limits. FFADO's own bounds check
/// is off by one and it accepts any finite value, so this is the guard.
pub fn validate(write: &GainWrite) -> Result<(i32, i32, f64)> {
    let reject = |why: String| Err(Error::Rejected(why));
    if write.row >= write.matrix.rows() || write.col >= CHANNELS {
        return reject(format!(
            "{} has no crosspoint ({}, {})",
            write.matrix.control(),
            write.row,
            write.col
        ));
    }
    if !write.value.is_finite() || write.value.abs() > GAIN_MAX {
        return reject(format!(
            "gain {} is outside ±{GAIN_MAX} (+6 dB)",
            write.value
        ));
    }
    if write.value.fract() != 0.0 {
        return reject(format!("gain {} is not a whole number", write.value));
    }
    Ok((write.row as i32, write.col as i32, write.value))
}

pub struct FfadoWriter {
    connection: Connection,
    owner: OwnedUniqueName,
    guid: String,
}

impl FfadoWriter {
    /// Binds to the `ffado-dbus-server` that is running now. Fails if none
    /// is running; it does not start one.
    pub fn connect(guid: &str) -> Result<Self> {
        let connection = Connection::session()?;
        let owner = current_owner(&connection)?.ok_or(Error::ServerGone)?;
        Ok(Self {
            connection,
            owner,
            guid: guid.to_owned(),
        })
    }

    /// True while the server we bound to is still the one running.
    pub fn server_unchanged(&self) -> Result<bool> {
        Ok(current_owner(&self.connection)?.as_ref() == Some(&self.owner))
    }

    pub fn set_gain(
        &self,
        write: &GainWrite,
    ) -> Result<()> {
        let body = validate(write)?;
        let path = device_path(&self.guid, write.matrix.control());
        let object_path =
            OwnedObjectPath::try_from(path.as_str()).map_err(|_| Error::InvalidPath(path))?;
        let reply = self.connection.call_method(
            Some(self.owner.as_str()),
            object_path,
            Some(IFACE_MATRIX),
            WRITE_METHOD,
            &body,
        )?;
        let status: f64 = reply.body().deserialize()?;
        if status < 0.0 {
            return Err(Error::Rejected(format!(
                "ffado-dbus-server refused {write:?}"
            )));
        }
        Ok(())
    }
}

fn current_owner(connection: &Connection) -> Result<Option<OwnedUniqueName>> {
    let proxy = DBusProxy::new(connection)?;
    let name = BusName::try_from(SERVICE).map_err(zbus::Error::from)?;
    match proxy.get_name_owner(name) {
        Ok(owner) => Ok(Some(owner)),
        Err(zbus::fdo::Error::NameHasNoOwner(_)) => Ok(None),
        Err(e) => Err(zbus::Error::from(e).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(
        matrix: FaderMatrix,
        row: usize,
        col: usize,
        value: f64,
    ) -> GainWrite {
        GainWrite {
            matrix,
            row,
            col,
            value,
        }
    }

    #[test]
    fn only_fader_matrices_are_writable() {
        let controls = [
            FaderMatrix::Inputs,
            FaderMatrix::Playbacks,
            FaderMatrix::Outputs,
        ]
        .map(FaderMatrix::control);
        assert_eq!(
            controls,
            [
                "Mixer/InputFaders",
                "Mixer/PlaybackFaders",
                "Mixer/OutputFaders"
            ]
        );
    }

    #[test]
    fn accepts_full_range_and_inversion() {
        for value in [0.0, 1.0, GAIN_MAX, -GAIN_MAX, -1.0] {
            assert!(validate(&write(FaderMatrix::Inputs, 27, 27, value)).is_ok());
        }
    }

    #[test]
    fn rejects_gain_above_plus_six_db() {
        for value in [GAIN_MAX + 1.0, -GAIN_MAX - 1.0, f64::NAN, f64::INFINITY] {
            assert!(validate(&write(FaderMatrix::Playbacks, 0, 0, value)).is_err());
        }
    }

    #[test]
    fn rejects_fractional_gain() {
        assert!(validate(&write(FaderMatrix::Inputs, 0, 0, 100.5)).is_err());
    }

    #[test]
    fn rejects_out_of_range_crosspoints() {
        // FFADO itself would accept channel 28 (its check is `>`).
        assert!(validate(&write(FaderMatrix::Inputs, 28, 0, 1.0)).is_err());
        assert!(validate(&write(FaderMatrix::Inputs, 0, 28, 1.0)).is_err());
        assert!(validate(&write(FaderMatrix::Outputs, 1, 0, 1.0)).is_err());
        assert!(validate(&write(FaderMatrix::Outputs, 0, 27, 1.0)).is_ok());
    }
}
