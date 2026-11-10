//! The TotalMix submix model.
//!
//! There is no channel fader. Every source strip (hardware input or
//! software playback, mono or stereo) has one send — gain plus pan — to
//! each of the 14 stereo hardware outputs. Mute, solo, pan and stereo
//! linking exist only here; [`Mixer::render`] turns them into the plain
//! per-crosspoint gains the hardware stores.

use serde::{Deserialize, Serialize};

use crate::law::{self, GAIN_MAX};
use crate::{CHANNELS, OUTPUT_PAIRS, PAIRS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Input,
    Playback,
}

/// One routing: how loud a source is in one output's submix, and where.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Send {
    /// Linear gain, 0.0 to [`GAIN_MAX`].
    pub gain: f64,
    /// −1 (left) to +1 (right). Pan for mono sources, balance for stereo.
    pub pan: f64,
}

/// State of one source channel. For a stereo pair the left (even)
/// channel holds mute, solo and sends for the whole strip; each side
/// keeps its own phase flag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Source {
    pub muted: bool,
    pub soloed: bool,
    pub phase_inverted: bool,
    pub sends: [Send; OUTPUT_PAIRS],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceBank {
    /// Whether channel pair `p` (channels `2p`, `2p+1`) is one stereo strip.
    pub stereo: [bool; PAIRS],
    pub channels: [Source; CHANNELS],
}

impl Default for SourceBank {
    fn default() -> Self {
        Self {
            stereo: [true; PAIRS],
            channels: std::array::from_fn(|_| Source::default()),
        }
    }
}

/// A hardware output pair (outputs are always stereo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Output {
    /// Linear gain, 0.0 to [`GAIN_MAX`].
    pub volume: f64,
    pub balance: f64,
    pub muted: bool,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            volume: 1.0,
            balance: 0.0,
            muted: false,
        }
    }
}

/// A strip as shown in the mixer: its first channel and whether it
/// covers a stereo pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StripRef {
    pub channel: usize,
    pub stereo: bool,
}

/// Raw per-crosspoint gains in FFADO D-Bus units (negative = phase
/// inverted), laid out like FFADO's matrices: `[output][source]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crosspoints {
    pub inputs: Vec<Vec<f64>>,
    pub playbacks: Vec<Vec<f64>>,
    pub outputs: Vec<f64>,
}

impl Crosspoints {
    pub fn silent() -> Self {
        Self {
            inputs: vec![vec![0.0; CHANNELS]; CHANNELS],
            playbacks: vec![vec![0.0; CHANNELS]; CHANNELS],
            outputs: vec![0.0; CHANNELS],
        }
    }

    /// Number of crosspoints that differ by more than one D-Bus step.
    pub fn differences(
        &self,
        other: &Crosspoints,
    ) -> usize {
        let differ = |a: &f64, b: &f64| (a - b).abs() > 1.0;
        let matrix = |a: &[Vec<f64>], b: &[Vec<f64>]| {
            a.iter()
                .flatten()
                .zip(b.iter().flatten())
                .filter(|(x, y)| differ(x, y))
                .count()
        };
        matrix(&self.inputs, &other.inputs)
            + matrix(&self.playbacks, &other.playbacks)
            + self
                .outputs
                .iter()
                .zip(&other.outputs)
                .filter(|(x, y)| differ(x, y))
                .count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MixerError {
    ChannelOutOfRange(usize),
    OutputOutOfRange(usize),
    /// The right channel of a stereo pair was addressed directly.
    NotAStripLeader(usize),
}

impl std::fmt::Display for MixerError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            MixerError::ChannelOutOfRange(c) => write!(f, "channel {c} out of range"),
            MixerError::OutputOutOfRange(o) => write!(f, "output pair {o} out of range"),
            MixerError::NotAStripLeader(c) => {
                write!(f, "channel {c} is the right side of a stereo strip")
            }
        }
    }
}

impl std::error::Error for MixerError {}

pub type Result<T, E = MixerError> = std::result::Result<T, E>;

#[derive(Clone, Debug, PartialEq)]
pub struct Mixer {
    pub inputs: SourceBank,
    pub playbacks: SourceBank,
    pub outputs: [Output; OUTPUT_PAIRS],
    /// The submix being edited (Submix view). Solo applies only here.
    pub selected_output: usize,
}

impl Default for Mixer {
    /// TotalMix "Straight playback": each playback pair to the matching
    /// output pair at 0 dB, inputs silent, outputs at 0 dB.
    fn default() -> Self {
        let mut mixer = Self {
            inputs: SourceBank::default(),
            playbacks: SourceBank::default(),
            outputs: [Output::default(); OUTPUT_PAIRS],
            selected_output: 0,
        };
        for pair in 0..PAIRS {
            mixer.playbacks.channels[2 * pair].sends[pair].gain = 1.0;
        }
        mixer
    }
}

impl Mixer {
    pub fn bank(
        &self,
        kind: SourceKind,
    ) -> &SourceBank {
        match kind {
            SourceKind::Input => &self.inputs,
            SourceKind::Playback => &self.playbacks,
        }
    }

    fn bank_mut(
        &mut self,
        kind: SourceKind,
    ) -> &mut SourceBank {
        match kind {
            SourceKind::Input => &mut self.inputs,
            SourceKind::Playback => &mut self.playbacks,
        }
    }

    /// Strips in display order for one row.
    pub fn strips(
        &self,
        kind: SourceKind,
    ) -> Vec<StripRef> {
        let bank = self.bank(kind);
        let mut strips = Vec::with_capacity(CHANNELS);
        for pair in 0..PAIRS {
            if bank.stereo[pair] {
                strips.push(StripRef {
                    channel: 2 * pair,
                    stereo: true,
                });
            } else {
                for channel in [2 * pair, 2 * pair + 1] {
                    strips.push(StripRef {
                        channel,
                        stereo: false,
                    });
                }
            }
        }
        strips
    }

    pub fn source(
        &self,
        kind: SourceKind,
        channel: usize,
    ) -> Result<&Source> {
        self.check_leader(kind, channel)?;
        Ok(&self.bank(kind).channels[channel])
    }

    pub fn is_stereo(
        &self,
        kind: SourceKind,
        channel: usize,
    ) -> bool {
        self.bank(kind).stereo[channel / 2]
    }

    pub fn select_output(
        &mut self,
        output: usize,
    ) -> Result<()> {
        check_output(output)?;
        self.selected_output = output;
        Ok(())
    }

    pub fn set_send_gain(
        &mut self,
        kind: SourceKind,
        channel: usize,
        output: usize,
        gain: f64,
    ) -> Result<()> {
        check_output(output)?;
        self.source_mut(kind, channel)?.sends[output].gain = clamp_gain(gain);
        Ok(())
    }

    pub fn set_send_pan(
        &mut self,
        kind: SourceKind,
        channel: usize,
        output: usize,
        pan: f64,
    ) -> Result<()> {
        check_output(output)?;
        self.source_mut(kind, channel)?.sends[output].pan = clamp_pan(pan);
        Ok(())
    }

    pub fn set_muted(
        &mut self,
        kind: SourceKind,
        channel: usize,
        muted: bool,
    ) -> Result<()> {
        self.source_mut(kind, channel)?.muted = muted;
        Ok(())
    }

    pub fn set_soloed(
        &mut self,
        kind: SourceKind,
        channel: usize,
        soloed: bool,
    ) -> Result<()> {
        self.source_mut(kind, channel)?.soloed = soloed;
        Ok(())
    }

    /// Switches a channel pair between one stereo strip and two mono
    /// strips, keeping the sound as close as possible (manual §25.3/§25.6):
    /// splitting pans the two halves hard left and right; joining keeps
    /// the louder gain and centres the balance.
    pub fn set_stereo(
        &mut self,
        kind: SourceKind,
        pair: usize,
        stereo: bool,
    ) -> Result<()> {
        if pair >= PAIRS {
            return Err(MixerError::ChannelOutOfRange(2 * pair));
        }
        let bank = self.bank_mut(kind);
        if bank.stereo[pair] == stereo {
            return Ok(());
        }
        let (left_index, right_index) = (2 * pair, 2 * pair + 1);
        if stereo {
            let right = bank.channels[right_index].clone();
            let left = &mut bank.channels[left_index];
            for (send, other) in left.sends.iter_mut().zip(right.sends.iter()) {
                send.gain = send.gain.max(other.gain);
                send.pan = 0.0;
            }
            left.muted = left.muted && right.muted;
            left.soloed = left.soloed || right.soloed;
        } else {
            let mut left = bank.channels[left_index].clone();
            for send in left.sends.iter_mut() {
                send.pan = -1.0;
            }
            let mut right = left.clone();
            right.phase_inverted = bank.channels[right_index].phase_inverted;
            for send in right.sends.iter_mut() {
                send.pan = 1.0;
            }
            bank.channels[left_index] = left;
            bank.channels[right_index] = right;
        }
        bank.stereo[pair] = stereo;
        Ok(())
    }

    pub fn set_output_volume(
        &mut self,
        output: usize,
        volume: f64,
    ) -> Result<()> {
        check_output(output)?;
        self.outputs[output].volume = clamp_gain(volume);
        Ok(())
    }

    pub fn set_output_balance(
        &mut self,
        output: usize,
        balance: f64,
    ) -> Result<()> {
        check_output(output)?;
        self.outputs[output].balance = clamp_pan(balance);
        Ok(())
    }

    pub fn set_output_muted(
        &mut self,
        output: usize,
        muted: bool,
    ) -> Result<()> {
        check_output(output)?;
        self.outputs[output].muted = muted;
        Ok(())
    }

    pub fn any_muted(&self) -> bool {
        self.leaders().any(|(_, source)| source.muted)
    }

    pub fn any_soloed(&self) -> bool {
        self.leaders().any(|(_, source)| source.soloed)
    }

    pub fn clear_mutes(&mut self) {
        for bank in [&mut self.inputs, &mut self.playbacks] {
            bank.channels
                .iter_mut()
                .for_each(|source| source.muted = false);
        }
    }

    pub fn clear_solos(&mut self) {
        for bank in [&mut self.inputs, &mut self.playbacks] {
            bank.channels
                .iter_mut()
                .for_each(|source| source.soloed = false);
        }
    }

    /// Whether a strip sends anything to any output.
    pub fn is_routed(
        &self,
        kind: SourceKind,
        channel: usize,
    ) -> bool {
        self.bank(kind).channels[channel]
            .sends
            .iter()
            .any(|send| send.gain > 0.0)
    }

    /// Turns the model into hardware crosspoint gains.
    pub fn render(&self) -> Crosspoints {
        let any_solo = self.any_soloed();
        let mut crosspoints = Crosspoints::silent();
        for (kind, matrix) in [
            (SourceKind::Input, &mut crosspoints.inputs),
            (SourceKind::Playback, &mut crosspoints.playbacks),
        ] {
            let bank = self.bank(kind);
            for strip in self.strips(kind) {
                let source = &bank.channels[strip.channel];
                if source.muted {
                    continue;
                }
                for (output, send) in source.sends.iter().enumerate() {
                    let solo_silenced =
                        any_solo && output == self.selected_output && !source.soloed;
                    if send.gain == 0.0 || solo_silenced {
                        continue;
                    }
                    let (out_left, out_right) = (2 * output, 2 * output + 1);
                    if strip.stereo {
                        let (left, right) = law::balance(send.pan);
                        let right_channel = strip.channel + 1;
                        matrix[out_left][strip.channel] =
                            signed(send.gain * left, source.phase_inverted);
                        matrix[out_right][right_channel] = signed(
                            send.gain * right,
                            bank.channels[right_channel].phase_inverted,
                        );
                    } else {
                        let (left, right) = law::mono_pan(send.pan);
                        let phase = source.phase_inverted;
                        matrix[out_left][strip.channel] = signed(send.gain * left, phase);
                        matrix[out_right][strip.channel] = signed(send.gain * right, phase);
                    }
                }
            }
        }
        for (pair, output) in self.outputs.iter().enumerate() {
            if output.muted {
                continue;
            }
            let (left, right) = law::balance(output.balance);
            crosspoints.outputs[2 * pair] = law::to_dbus(output.volume * left);
            crosspoints.outputs[2 * pair + 1] = law::to_dbus(output.volume * right);
        }
        crosspoints
    }

    fn leaders(&self) -> impl Iterator<Item = (SourceKind, &Source)> {
        [SourceKind::Input, SourceKind::Playback]
            .into_iter()
            .flat_map(move |kind| {
                self.strips(kind)
                    .into_iter()
                    .map(move |strip| (kind, &self.bank(kind).channels[strip.channel]))
            })
    }

    fn check_leader(
        &self,
        kind: SourceKind,
        channel: usize,
    ) -> Result<()> {
        if channel >= CHANNELS {
            return Err(MixerError::ChannelOutOfRange(channel));
        }
        if channel % 2 == 1 && self.is_stereo(kind, channel) {
            return Err(MixerError::NotAStripLeader(channel));
        }
        Ok(())
    }

    fn source_mut(
        &mut self,
        kind: SourceKind,
        channel: usize,
    ) -> Result<&mut Source> {
        self.check_leader(kind, channel)?;
        Ok(&mut self.bank_mut(kind).channels[channel])
    }
}

fn check_output(output: usize) -> Result<()> {
    if output >= OUTPUT_PAIRS {
        Err(MixerError::OutputOutOfRange(output))
    } else {
        Ok(())
    }
}

fn clamp_gain(gain: f64) -> f64 {
    if gain.is_finite() {
        gain.clamp(0.0, GAIN_MAX)
    } else {
        0.0
    }
}

fn clamp_pan(pan: f64) -> f64 {
    if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn signed(
    gain: f64,
    inverted: bool,
) -> f64 {
    let raw = law::to_dbus(gain);
    if inverted && raw != 0.0 { -raw } else { raw }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_straight_playback() {
        let crosspoints = Mixer::default().render();
        for channel in 0..CHANNELS {
            assert_eq!(crosspoints.playbacks[channel][channel], 16384.0);
            assert_eq!(crosspoints.outputs[channel], 16384.0);
        }
        assert!(crosspoints.inputs.iter().flatten().all(|v| *v == 0.0));
    }

    #[test]
    fn mono_send_centre_is_minus_three_db_each_side() {
        let mut mixer = Mixer::default();
        mixer.set_stereo(SourceKind::Input, 3, false).unwrap();
        mixer.set_send_gain(SourceKind::Input, 6, 4, 1.0).unwrap();
        mixer.set_send_pan(SourceKind::Input, 6, 4, 0.0).unwrap();
        let crosspoints = mixer.render();
        assert_eq!(crosspoints.inputs[8][6], 11585.0);
        assert_eq!(crosspoints.inputs[9][6], 11585.0);
    }

    #[test]
    fn stereo_send_is_left_to_left_right_to_right() {
        let mut mixer = Mixer::default();
        mixer.set_send_gain(SourceKind::Input, 0, 6, 1.0).unwrap();
        let crosspoints = mixer.render();
        assert_eq!(crosspoints.inputs[12][0], 16384.0);
        assert_eq!(crosspoints.inputs[13][1], 16384.0);
        assert_eq!(crosspoints.inputs[13][0], 0.0);
        assert_eq!(crosspoints.inputs[12][1], 0.0);
    }

    #[test]
    fn mute_silences_every_routing() {
        let mut mixer = Mixer::default();
        mixer
            .set_send_gain(SourceKind::Playback, 0, 3, 1.0)
            .unwrap();
        mixer.set_muted(SourceKind::Playback, 0, true).unwrap();
        let crosspoints = mixer.render();
        assert_eq!(crosspoints.playbacks[0][0], 0.0);
        assert_eq!(crosspoints.playbacks[6][0], 0.0);
    }

    #[test]
    fn solo_only_affects_the_selected_submix() {
        let mut mixer = Mixer::default();
        // Playback 3/4 also feeds output 1/2, so output 1/2 has two sources.
        mixer
            .set_send_gain(SourceKind::Playback, 2, 0, 1.0)
            .unwrap();
        mixer.set_soloed(SourceKind::Playback, 2, true).unwrap();
        mixer.select_output(0).unwrap();
        let crosspoints = mixer.render();
        assert_eq!(
            crosspoints.playbacks[0][0], 0.0,
            "non-soloed silenced in submix"
        );
        assert_eq!(crosspoints.playbacks[0][2], 16384.0, "soloed kept");
        assert_eq!(
            crosspoints.playbacks[2][2], 16384.0,
            "other submixes untouched"
        );
    }

    #[test]
    fn right_channel_of_stereo_strip_is_rejected() {
        let mut mixer = Mixer::default();
        assert_eq!(
            mixer.set_send_gain(SourceKind::Input, 1, 0, 1.0),
            Err(MixerError::NotAStripLeader(1))
        );
    }

    #[test]
    fn gains_are_clamped() {
        let mut mixer = Mixer::default();
        mixer.set_output_volume(0, 10.0).unwrap();
        mixer
            .set_send_gain(SourceKind::Input, 0, 0, f64::NAN)
            .unwrap();
        let crosspoints = mixer.render();
        assert_eq!(crosspoints.outputs[0], 32768.0);
        assert_eq!(crosspoints.inputs[0][0], 0.0);
    }

    #[test]
    fn splitting_stereo_pans_hard_and_keeps_level() {
        let mut mixer = Mixer::default();
        mixer.set_send_gain(SourceKind::Input, 0, 0, 1.0).unwrap();
        let before = mixer.render();
        mixer.set_stereo(SourceKind::Input, 0, false).unwrap();
        assert_eq!(mixer.render(), before);
        assert_eq!(mixer.strips(SourceKind::Input)[1].channel, 1);
    }

    #[test]
    fn phase_inversion_negates() {
        let mut mixer = Mixer::default();
        mixer.playbacks.channels[0].phase_inverted = true;
        assert_eq!(mixer.render().playbacks[0][0], -16384.0);
    }
}
