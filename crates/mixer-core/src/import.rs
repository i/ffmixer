//! Rebuilds the submix model from raw hardware crosspoints, e.g. the
//! mix `ffado-dbus-server` loaded from flash.
//!
//! A channel pair becomes one stereo strip when its left channel only
//! ever feeds left outputs and its right channel only right outputs;
//! otherwise it becomes two mono strips. Any raw matrix is representable
//! this way except mixed phase signs on one channel and gains above
//! +6 dB, which are reported.

use crate::law::{self, GAIN_MAX};
use crate::mixer::{Crosspoints, Mixer, Output, SourceBank, SourceKind};
use crate::{CHANNELS, OUTPUT_PAIRS, PAIRS};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    /// Human-readable notes about things the model could not represent
    /// exactly. Empty means the import is lossless.
    pub warnings: Vec<String>,
}

pub fn import(crosspoints: &Crosspoints) -> (Mixer, ImportReport) {
    let mut report = ImportReport::default();
    let mut mixer = Mixer {
        inputs: import_bank(&crosspoints.inputs, SourceKind::Input, &mut report),
        playbacks: import_bank(&crosspoints.playbacks, SourceKind::Playback, &mut report),
        ..Mixer::default()
    };
    for pair in 0..OUTPUT_PAIRS {
        let (volume, balance) = law::unbalance(
            law::from_dbus(crosspoints.outputs[2 * pair]),
            law::from_dbus(crosspoints.outputs[2 * pair + 1]),
        );
        mixer.outputs[pair] = Output {
            volume,
            balance,
            muted: false,
        };
    }
    (mixer, report)
}

fn import_bank(
    matrix: &[Vec<f64>],
    kind: SourceKind,
    report: &mut ImportReport,
) -> SourceBank {
    let mut bank = SourceBank::default();
    let value = |output: usize, source: usize| matrix[output][source];

    for (channel, source) in bank.channels.iter_mut().enumerate() {
        source.phase_inverted = phase_of(
            (0..CHANNELS).map(|output| value(output, channel)),
            kind,
            channel,
            report,
        );
    }

    for pair in 0..PAIRS {
        let (left, right) = (2 * pair, 2 * pair + 1);
        let has_cross_terms = (0..OUTPUT_PAIRS)
            .any(|out| value(2 * out, right) != 0.0 || value(2 * out + 1, left) != 0.0);
        bank.stereo[pair] = !has_cross_terms;

        for out in 0..OUTPUT_PAIRS {
            let (out_left, out_right) = (2 * out, 2 * out + 1);
            if bank.stereo[pair] {
                let (gain, pan) = law::unbalance(
                    law::from_dbus(value(out_left, left)),
                    law::from_dbus(value(out_right, right)),
                );
                let send = &mut bank.channels[left].sends[out];
                send.gain = gain;
                send.pan = pan;
            } else {
                for channel in [left, right] {
                    let (gain, pan) = law::mono_unpan(
                        law::from_dbus(value(out_left, channel)),
                        law::from_dbus(value(out_right, channel)),
                    );
                    if gain > GAIN_MAX {
                        report.warnings.push(format!(
                            "{kind:?} {} → output pair {}: {:+.1} dB clipped to +6 dB",
                            channel + 1,
                            out + 1,
                            law::gain_to_db(gain).unwrap_or(0.0),
                        ));
                    }
                    let send = &mut bank.channels[channel].sends[out];
                    send.gain = gain.min(GAIN_MAX);
                    send.pan = pan;
                }
            }
        }
    }
    bank
}

fn phase_of(
    values: impl Iterator<Item = f64>,
    kind: SourceKind,
    channel: usize,
    report: &mut ImportReport,
) -> bool {
    let (mut negative, mut positive) = (false, false);
    for value in values {
        negative |= value < 0.0;
        positive |= value > 0.0;
    }
    if negative && positive {
        report.warnings.push(format!(
            "{kind:?} {}: mixed phase across routings; treated as normal phase",
            channel + 1,
        ));
        return false;
    }
    negative
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crosspoints read from this FF800 by `ffado-probe`.
    fn measured_unit() -> Crosspoints {
        let mut c = Crosspoints::silent();
        c.inputs[6][4] = 5.0;
        c.inputs[7][4] = 5.0;
        c.inputs[8][4] = 10444.0;
        c.inputs[9][4] = 10444.0;
        c.inputs[9][5] = 10.0;
        for channel in 0..10 {
            c.inputs[12 + channel][channel] = 16384.0;
        }
        for channel in 0..6 {
            c.playbacks[channel][channel] = 16384.0;
        }
        for (out, source) in [
            (6, 0),
            (6, 6),
            (7, 1),
            (7, 7),
            (8, 0),
            (8, 6),
            (9, 1),
            (9, 7),
        ] {
            c.playbacks[out][source] = 16384.0;
        }
        c.playbacks[10][0] = 16384.0;
        c.playbacks[11][1] = 16384.0;
        for channel in 18..28 {
            c.playbacks[channel][channel] = 16384.0;
        }
        c.outputs = vec![16384.0; CHANNELS];
        c.outputs[0] = 1060.0;
        c.outputs[1] = 1011.0;
        c.outputs[18] = 0.0;
        c.outputs[19] = 0.0;
        c
    }

    fn assert_within_one_step(
        a: &Crosspoints,
        b: &Crosspoints,
    ) {
        assert_eq!(a.differences(b), 0, "\n{a:?}\n!=\n{b:?}");
    }

    #[test]
    fn measured_unit_round_trips() {
        let original = measured_unit();
        let (mixer, report) = import(&original);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_within_one_step(&mixer.render(), &original);
    }

    #[test]
    fn measured_unit_layout() {
        let (mixer, _) = import(&measured_unit());
        // AN 5/6 feed Mon 9/10 with cross terms, so they split to mono.
        assert!(!mixer.inputs.stereo[2]);
        assert!(mixer.inputs.stereo[0]);
        // AN 5 is centred into PH 9/10.
        let send = mixer.inputs.channels[4].sends[4];
        assert!(send.pan.abs() < 1e-9);
        // Line out 1/2 sits about 24 dB down with a slight balance.
        let output = mixer.outputs[0];
        assert!((law::gain_to_db(output.volume).unwrap() + 23.8).abs() < 0.1);
        assert!(output.balance < 0.0);
    }

    #[test]
    fn default_mixer_round_trips() {
        let mixer = Mixer::default();
        let (imported, report) = import(&mixer.render());
        assert!(report.warnings.is_empty());
        assert_within_one_step(&imported.render(), &mixer.render());
    }

    #[test]
    fn phase_is_recovered() {
        let mut c = Crosspoints::silent();
        c.playbacks[0][0] = -16384.0;
        c.playbacks[1][1] = 16384.0;
        let (mixer, report) = import(&c);
        assert!(report.warnings.is_empty());
        assert!(mixer.playbacks.channels[0].phase_inverted);
        assert!(!mixer.playbacks.channels[1].phase_inverted);
        assert_within_one_step(&mixer.render(), &c);
    }

    #[test]
    fn mixed_phase_is_reported() {
        let mut c = Crosspoints::silent();
        c.inputs[0][0] = -16384.0;
        c.inputs[2][0] = 16384.0;
        let (_, report) = import(&c);
        assert_eq!(report.warnings.len(), 1);
    }
}
