//! Gain units, pan law and balance law.
//!
//! Gains in the model are linear amplitudes: 1.0 is 0 dB, 0.0 is −∞ and
//! [`GAIN_MAX`] is +6 dB. Conversion to FFADO's D-Bus units happens only
//! in [`to_dbus`] / [`from_dbus`].

use std::f64::consts::FRAC_PI_4;

/// Largest linear gain the Fireface accepts (+6.02 dB).
pub const GAIN_MAX: f64 = 2.0;

/// FFADO D-Bus value for 0 dB (see `ffado-dbus`).
pub const DBUS_UNITY: f64 = 16384.0;

/// Smallest dB value treated as audible; anything below is −∞.
pub const DB_FLOOR: f64 = -144.0;

pub fn db_to_gain(db: Option<f64>) -> f64 {
    match db {
        Some(db) if db > DB_FLOOR => 10f64.powf(db / 20.0).min(GAIN_MAX),
        _ => 0.0,
    }
}

pub fn gain_to_db(gain: f64) -> Option<f64> {
    if gain <= 0.0 {
        None
    } else {
        Some(20.0 * gain.log10()).filter(|db| *db > DB_FLOOR)
    }
}

/// Linear gain → FFADO D-Bus value, rounded and clamped to the valid range.
pub fn to_dbus(gain: f64) -> f64 {
    (gain.clamp(0.0, GAIN_MAX) * DBUS_UNITY).round()
}

/// FFADO D-Bus value → linear gain. The sign (phase) is dropped.
pub fn from_dbus(raw: f64) -> f64 {
    (raw.abs() / DBUS_UNITY).min(GAIN_MAX)
}

/// Mono source into a stereo destination: constant power, −3 dB at
/// centre (manual §25.3). `pan` runs from −1 (left) to +1 (right).
pub fn mono_pan(pan: f64) -> (f64, f64) {
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (angle.cos(), angle.sin())
}

/// Inverse of [`mono_pan`]: per-side gains → (gain, pan).
pub fn mono_unpan(
    left: f64,
    right: f64,
) -> (f64, f64) {
    let gain = left.hypot(right);
    if gain == 0.0 {
        return (0.0, 0.0);
    }
    let angle = right.atan2(left);
    (gain, angle / FRAC_PI_4 - 1.0)
}

/// Stereo source or output: balance. Centre leaves both sides at full
/// level; moving away attenuates the opposite side with the same curve
/// as [`mono_pan`], normalised so the near side stays at unity.
pub fn balance(pan: f64) -> (f64, f64) {
    let (left, right) = mono_pan(pan);
    let scale = std::f64::consts::SQRT_2;
    ((left * scale).min(1.0), (right * scale).min(1.0))
}

/// Inverse of [`balance`]: per-side gains → (gain, pan).
pub fn unbalance(
    left: f64,
    right: f64,
) -> (f64, f64) {
    let gain = left.max(right);
    if gain == 0.0 {
        return (0.0, 0.0);
    }
    let quieter = (left.min(right) / gain).clamp(0.0, 1.0);
    // Near side is unity, so the quieter side is √2·sin(angle) for the
    // angle measured from the hard-panned position.
    let angle = (quieter / std::f64::consts::SQRT_2).asin();
    let offset = 1.0 - angle / FRAC_PI_4;
    let pan = if right >= left { offset } else { -offset };
    (gain, pan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(
        a: f64,
        b: f64,
    ) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn mono_centre_is_minus_three_db() {
        let (left, right) = mono_pan(0.0);
        assert!(close(left, right));
        assert!((gain_to_db(left).unwrap() + 3.01).abs() < 0.01);
    }

    #[test]
    fn mono_extremes() {
        let (left, right) = mono_pan(-1.0);
        assert!(close(left, 1.0) && right.abs() < 1e-12);
        let (left, right) = mono_pan(1.0);
        assert!(left.abs() < 1e-12 && close(right, 1.0));
    }

    #[test]
    fn mono_round_trip() {
        for pan in [-1.0, -0.6, -0.1, 0.0, 0.3, 1.0] {
            let (left, right) = mono_pan(pan);
            let (gain, back) = mono_unpan(left * 0.7, right * 0.7);
            assert!(close(gain, 0.7), "{pan}: {gain}");
            assert!(close(back, pan), "{pan}: {back}");
        }
    }

    #[test]
    fn balance_centre_is_unity() {
        assert_eq!(balance(0.0), (1.0, 1.0));
    }

    #[test]
    fn balance_round_trip() {
        for pan in [-1.0, -0.5, -0.05, 0.0, 0.2, 0.9, 1.0] {
            let (left, right) = balance(pan);
            let (gain, back) = unbalance(left * 1.5, right * 1.5);
            assert!(close(gain, 1.5), "{pan}: {gain}");
            assert!((back - pan).abs() < 1e-9, "{pan}: {back}");
        }
    }

    #[test]
    fn dbus_scale() {
        assert_eq!(to_dbus(1.0), 16384.0);
        assert_eq!(to_dbus(5.0), 32768.0);
        assert_eq!(from_dbus(-16384.0), 1.0);
        assert_eq!(db_to_gain(Some(0.0)), 1.0);
        assert_eq!(db_to_gain(None), 0.0);
        assert_eq!(gain_to_db(0.0), None);
    }
}
