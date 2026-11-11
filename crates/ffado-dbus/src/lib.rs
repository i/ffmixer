//! Client for `ffado-dbus-server`.
//!
//! [`FfadoReader`] exposes **only getters**: the only D-Bus methods it
//! calls are the read methods listed in [`READ_METHODS`]. All writing
//! lives in [`writer`], which can change fader gains and the typed
//! [`settings::Setting`]s (analog and digital options, clock source) and
//! nothing else: no flash, sample rate or bandwidth controls.

pub mod settings;
pub mod writer;

use serde::Serialize;
use serde::de::DeserializeOwned;
use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::names::{BusName, OwnedUniqueName};
use zbus::zvariant::{DynamicType, OwnedObjectPath, Type};

pub(crate) const SERVICE: &str = "org.ffado.Control";
const DEVICE_MANAGER_PATH: &str = "/org/ffado/Control/DeviceManager";

const IFACE_CONTAINER: &str = "org.ffado.Control.Element.Container";
const IFACE_DISCRETE: &str = "org.ffado.Control.Element.Discrete";
pub(crate) const IFACE_MATRIX: &str = "org.ffado.Control.Element.MatrixMixer";

/// Every D-Bus method this crate is allowed to call.
///
/// [`FfadoReader::call`] refuses anything else, so a typo or a future
/// edit cannot accidentally send a setter to the hardware.
pub const READ_METHODS: &[&str] = &[
    "getNbElements",
    "getElementName",
    "getValue",
    "getRowCount",
    "getColCount",
    "getRowName",
    "getColName",
];

/// D-Bus gain value that corresponds to 0 dB on RME Fireface mixers.
///
/// FFADO's D-Bus mixer values are half the hardware register values: it
/// doubles them before writing (`RmeSettingsMatrixCtrl::setValue`). On
/// D-Bus the scale is linear: 0 is −∞, 0x4000 is 0 dB and 0x8000
/// ([`GAIN_MAX`]) is +6 dB. Negative values mean the crosspoint is
/// phase-inverted. Confirmed on a FF800 with a test tone.
pub const GAIN_UNITY: f64 = 16384.0;

/// Largest D-Bus gain (+6 dB; hardware register 0x10000).
pub const GAIN_MAX: f64 = 32768.0;

#[derive(Debug)]
pub enum Error {
    DBus(zbus::Error),
    /// A method outside [`READ_METHODS`] was requested.
    NotReadOnly(String),
    InvalidPath(String),
    /// A write outside the hardware limits, or refused by the server.
    Rejected(String),
    /// The `ffado-dbus-server` the writer bound to is no longer running.
    ServerGone,
    /// FFADO reported a value this crate does not understand.
    Unexpected(String),
}

impl std::fmt::Display for Error {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Error::DBus(e) => write!(f, "D-Bus error: {e}"),
            Error::NotReadOnly(m) => write!(f, "refusing non-read method `{m}`"),
            Error::InvalidPath(p) => write!(f, "invalid object path `{p}`"),
            Error::Rejected(why) => write!(f, "write rejected: {why}"),
            Error::ServerGone => write!(f, "ffado-dbus-server is not running"),
            Error::Unexpected(what) => write!(f, "unexpected value from FFADO: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<zbus::Error> for Error {
    fn from(e: zbus::Error) -> Self {
        Error::DBus(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A mixer matrix as published by FFADO: `rows` are destinations
/// (hardware outputs), `cols` are sources.
#[derive(Debug, Clone, PartialEq)]
pub struct MatrixSnapshot {
    pub row_names: Vec<String>,
    pub col_names: Vec<String>,
    /// Raw hardware gains, indexed `[row][col]`.
    pub values: Vec<Vec<f64>>,
}

pub struct FfadoReader {
    connection: Connection,
    /// Bus name calls go to: the service name (which D-Bus will start a
    /// server for) or the unique name of one running server.
    destination: String,
}

impl FfadoReader {
    /// Connects to `ffado-dbus-server` on the session bus. D-Bus starts
    /// a server if none is running.
    pub fn connect() -> Result<Self> {
        Ok(Self {
            connection: Connection::session()?,
            destination: SERVICE.to_owned(),
        })
    }

    /// Binds to the server that is running now and never starts one. If
    /// that server exits, reads fail. Use this for anything repeated: a
    /// new server would reload the flash mix onto the unit.
    pub fn connect_running() -> Result<Self> {
        let connection = Connection::session()?;
        let owner = current_owner(&connection)?.ok_or(Error::ServerGone)?;
        Ok(Self {
            connection,
            destination: owner.to_string(),
        })
    }

    /// GUIDs of all devices FFADO has discovered, e.g. `000a35004aa780e6`.
    pub fn device_guids(&self) -> Result<Vec<String>> {
        let count: i32 = self.call(DEVICE_MANAGER_PATH, IFACE_CONTAINER, "getNbElements", &())?;
        (0..count)
            .map(|index| {
                self.call(
                    DEVICE_MANAGER_PATH,
                    IFACE_CONTAINER,
                    "getElementName",
                    &(index,),
                )
            })
            .collect()
    }

    /// Reads a discrete control, e.g. `Control/Phantom`.
    pub fn discrete(
        &self,
        guid: &str,
        control: &str,
    ) -> Result<i32> {
        self.call(&device_path(guid, control), IFACE_DISCRETE, "getValue", &())
    }

    /// Reads a whole matrix mixer, e.g. `Mixer/InputFaders`.
    pub fn matrix(
        &self,
        guid: &str,
        control: &str,
    ) -> Result<MatrixSnapshot> {
        let path = device_path(guid, control);
        let rows: i32 = self.call(&path, IFACE_MATRIX, "getRowCount", &())?;
        let cols: i32 = self.call(&path, IFACE_MATRIX, "getColCount", &())?;

        let row_names = (0..rows)
            .map(|row| self.call(&path, IFACE_MATRIX, "getRowName", &(row,)))
            .collect::<Result<Vec<String>>>()?;
        let col_names = (0..cols)
            .map(|col| self.call(&path, IFACE_MATRIX, "getColName", &(col,)))
            .collect::<Result<Vec<String>>>()?;

        let mut values = Vec::with_capacity(rows as usize);
        for row in 0..rows {
            let row_values = (0..cols)
                .map(|col| self.call(&path, IFACE_MATRIX, "getValue", &(row, col)))
                .collect::<Result<Vec<f64>>>()?;
            values.push(row_values);
        }

        Ok(MatrixSnapshot {
            row_names,
            col_names,
            values,
        })
    }

    /// Reads all of the unit's settings (see [`settings::DeviceSettings`]).
    pub fn device_settings(
        &self,
        guid: &str,
    ) -> Result<settings::DeviceSettings> {
        settings::DeviceSettings::read_with(|control| self.discrete(guid, control))
    }

    /// Reads the clock and input lock status (see [`settings::ClockStatus`]).
    pub fn clock_status(
        &self,
        guid: &str,
    ) -> Result<settings::ClockStatus> {
        settings::ClockStatus::read_with(|control| self.discrete(guid, control))
    }

    fn call<B, R>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        body: &B,
    ) -> Result<R>
    where
        B: Serialize + DynamicType,
        R: DeserializeOwned + Type,
    {
        if !READ_METHODS.contains(&method) {
            return Err(Error::NotReadOnly(method.to_owned()));
        }
        let object_path =
            OwnedObjectPath::try_from(path).map_err(|_| Error::InvalidPath(path.to_owned()))?;
        let reply = self.connection.call_method(
            Some(self.destination.as_str()),
            object_path,
            Some(interface),
            method,
            body,
        )?;
        Ok(reply.body().deserialize()?)
    }
}

/// The unique bus name of the running server, if any.
pub(crate) fn current_owner(connection: &Connection) -> Result<Option<OwnedUniqueName>> {
    let proxy = DBusProxy::new(connection)?;
    let name = BusName::try_from(SERVICE).map_err(zbus::Error::from)?;
    match proxy.get_name_owner(name) {
        Ok(owner) => Ok(Some(owner)),
        Err(zbus::fdo::Error::NameHasNoOwner(_)) => Ok(None),
        Err(e) => Err(zbus::Error::from(e).into()),
    }
}

/// Converts a raw FFADO/RME gain to dB. Returns `None` for −∞.
/// Phase inversion (negative values) is ignored for the magnitude.
pub fn gain_to_db(raw: f64) -> Option<f64> {
    let magnitude = raw.abs();
    if magnitude < 1.0 {
        None
    } else {
        Some(20.0 * (magnitude / GAIN_UNITY).log10())
    }
}

pub(crate) fn device_path(
    guid: &str,
    control: &str,
) -> String {
    format!("{DEVICE_MANAGER_PATH}/{guid}/{control}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_is_zero_db() {
        assert_eq!(gain_to_db(GAIN_UNITY), Some(0.0));
    }

    #[test]
    fn half_is_minus_six_db() {
        let db = gain_to_db(GAIN_UNITY / 2.0).unwrap();
        assert!((db + 6.02).abs() < 0.01, "{db}");
    }

    #[test]
    fn zero_is_minus_infinity() {
        assert_eq!(gain_to_db(0.0), None);
    }

    #[test]
    fn inverted_gain_keeps_magnitude() {
        assert_eq!(gain_to_db(-GAIN_UNITY), Some(0.0));
    }

    #[test]
    fn max_is_plus_six_db() {
        let db = gain_to_db(GAIN_MAX).unwrap();
        assert!((db - 6.02).abs() < 0.01, "{db}");
    }

    #[test]
    fn measured_output_fader_matches() {
        // Line out 1 at 1060 measured 23.8 dB below its source.
        let db = gain_to_db(1060.0).unwrap();
        assert!((db + 23.8).abs() < 0.05, "{db}");
    }

    #[test]
    fn setters_are_not_whitelisted() {
        for forbidden in ["setValue", "setValueIdx", "connectRowTo", "setRowName"] {
            assert!(!READ_METHODS.contains(&forbidden), "{forbidden}");
        }
    }
}
