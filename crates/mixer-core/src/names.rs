//! TotalMix-style channel names for the Fireface 800.
//!
//! Channel order (inputs, playbacks and outputs alike): analog 1–10,
//! SPDIF L/R, ADAT 1–16.

use crate::CHANNELS;

/// Name of a single hardware input channel.
pub fn input_name(channel: usize) -> String {
    match channel {
        0..=5 => format!("AN {}", channel + 1),
        6..=9 => format!("Mic {}", channel + 1),
        10 => "SPDIF L".to_owned(),
        11 => "SPDIF R".to_owned(),
        _ => format!("ADAT {}", channel - 11),
    }
}

/// Name of a single output (and playback) channel. Outputs 9/10 are the
/// front headphone jack, shown as `PH` like TotalMix.
pub fn output_name(channel: usize) -> String {
    match channel {
        0..=7 => format!("AN {}", channel + 1),
        8 | 9 => format!("PH {}", channel + 1),
        10 => "SPDIF L".to_owned(),
        11 => "SPDIF R".to_owned(),
        _ => format!("ADAT {}", channel - 11),
    }
}

/// Name for a stereo pair starting at `first`, e.g. `AN 1/2`, `SPDIF`.
pub fn pair_name(
    first: usize,
    single: fn(usize) -> String,
) -> String {
    debug_assert!(first.is_multiple_of(2) && first + 1 < CHANNELS);
    if first == 10 {
        return "SPDIF".to_owned();
    }
    let left = single(first);
    let right = single(first + 1);
    match left.rsplit_once(' ') {
        Some((prefix, number)) => {
            let right_number = right.rsplit_once(' ').map_or("", |(_, n)| n);
            format!("{prefix} {number}/{right_number}")
        }
        None => format!("{left}/{right}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_names() {
        assert_eq!(input_name(0), "AN 1");
        assert_eq!(input_name(6), "Mic 7");
        assert_eq!(input_name(11), "SPDIF R");
        assert_eq!(input_name(12), "ADAT 1");
        assert_eq!(input_name(27), "ADAT 16");
    }

    #[test]
    fn pair_names() {
        assert_eq!(pair_name(0, input_name), "AN 1/2");
        assert_eq!(pair_name(6, input_name), "Mic 7/8");
        assert_eq!(pair_name(8, output_name), "PH 9/10");
        assert_eq!(pair_name(10, output_name), "SPDIF");
        assert_eq!(pair_name(26, output_name), "ADAT 15/16");
    }
}
