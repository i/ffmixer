//! The TotalMix submix model.
//!
//! There is no channel fader. Every source strip (hardware input or
//! software playback, mono or stereo) has one send — gain plus pan — to
//! each of the 14 stereo hardware outputs. Mute, solo, pan, stereo
//! linking, width, M/S processing and phase exist only here;
//! [`Mixer::render`] turns them into the plain per-crosspoint gains the
//! hardware stores.

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
    /// Stereo width per pair (manual §25.3): 1 = normal, 0 = mono,
    /// −1 = left and right swapped. Only used while the pair is stereo.
    pub width: [f64; PAIRS],
    /// M/S processing per pair (manual §27.6). Only used while stereo.
    pub ms_proc: [bool; PAIRS],
    pub channels: [Source; CHANNELS],
}

impl Default for SourceBank {
    fn default() -> Self {
        Self {
            stereo: [true; PAIRS],
            width: [1.0; PAIRS],
            ms_proc: [false; PAIRS],
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
    /// Phase L / Phase R. The FF800 drivers expose no output inverters,
    /// so this inverts every crosspoint feeding that output channel,
    /// which sounds identical.
    pub phase_inverted: [bool; 2],
}

impl Default for Output {
    fn default() -> Self {
        Self {
            volume: 1.0,
            balance: 0.0,
            muted: false,
            phase_inverted: [false; 2],
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
    /// the louder gain and centres the balance. Width and M/S processing
    /// reset, since mono strips have neither.
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
        bank.width[pair] = 1.0;
        bank.ms_proc[pair] = false;
        Ok(())
    }

    /// Phase L / Phase R (or Phase, for a mono strip). Any channel may be
    /// addressed, including the right side of a stereo strip.
    pub fn set_phase(
        &mut self,
        kind: SourceKind,
        channel: usize,
        inverted: bool,
    ) -> Result<()> {
        if channel >= CHANNELS {
            return Err(MixerError::ChannelOutOfRange(channel));
        }
        self.bank_mut(kind).channels[channel].phase_inverted = inverted;
        Ok(())
    }

    pub fn set_width(
        &mut self,
        kind: SourceKind,
        pair: usize,
        width: f64,
    ) -> Result<()> {
        check_pair(pair)?;
        self.bank_mut(kind).width[pair] = clamp_pan(width);
        Ok(())
    }

    pub fn set_ms_proc(
        &mut self,
        kind: SourceKind,
        pair: usize,
        enabled: bool,
    ) -> Result<()> {
        check_pair(pair)?;
        self.bank_mut(kind).ms_proc[pair] = enabled;
        Ok(())
    }

    /// Phase of one output channel (0–27).
    pub fn set_output_phase(
        &mut self,
        channel: usize,
        inverted: bool,
    ) -> Result<()> {
        if channel >= CHANNELS {
            return Err(MixerError::ChannelOutOfRange(channel));
        }
        self.outputs[channel / 2].phase_inverted[channel % 2] = inverted;
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
                    let out_channels = [2 * output, 2 * output + 1];
                    if strip.stereo {
                        let pair = strip.channel / 2;
                        let mix = stereo_mix(bank.width[pair], bank.ms_proc[pair]);
                        let sides = law::balance(send.pan);
                        let side_gains = [sides.0, sides.1];
                        let source_channels = [strip.channel, strip.channel + 1];
                        for (row, out_channel) in out_channels.into_iter().enumerate() {
                            for (col, source_channel) in source_channels.into_iter().enumerate() {
                                let phase =
                                    phase_sign(bank.channels[source_channel].phase_inverted);
                                matrix[out_channel][source_channel] = to_crosspoint(
                                    send.gain * side_gains[row] * mix[row][col] * phase,
                                );
                            }
                        }
                    } else {
                        let (left, right) = law::mono_pan(send.pan);
                        let phase = phase_sign(source.phase_inverted);
                        matrix[out_channels[0]][strip.channel] =
                            to_crosspoint(send.gain * left * phase);
                        matrix[out_channels[1]][strip.channel] =
                            to_crosspoint(send.gain * right * phase);
                    }
                }
            }
        }
        for channel in 0..CHANNELS {
            if self.outputs[channel / 2].phase_inverted[channel % 2] {
                for matrix in [&mut crosspoints.inputs, &mut crosspoints.playbacks] {
                    for value in matrix[channel].iter_mut().filter(|value| **value != 0.0) {
                        *value = -*value;
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

fn check_pair(pair: usize) -> Result<()> {
    if pair >= PAIRS {
        Err(MixerError::ChannelOutOfRange(2 * pair))
    } else {
        Ok(())
    }
}

fn phase_sign(inverted: bool) -> f64 {
    if inverted { -1.0 } else { 1.0 }
}

/// Signed linear gain to a D-Bus crosspoint value: the sign carries the
/// phase, the magnitude is capped at [`GAIN_MAX`].
fn to_crosspoint(gain: f64) -> f64 {
    let raw = law::to_dbus(gain.abs().min(GAIN_MAX));
    if raw == 0.0 { 0.0 } else { raw.copysign(gain) }
}

/// How a stereo strip's two channels (columns: left, right) reach the
/// two sides of an output (rows: left, right).
///
/// M/S processing comes first: mid (L+R) to the left, side (L−R) to the
/// right, scaled by 1/√2 so that applying it twice gives the original
/// (it works as encoder and decoder, manual §27.6). Width then blends
/// each side with the other: 1 keeps them, 0 sums to mono, −1 swaps.
fn stereo_mix(
    width: f64,
    ms_proc: bool,
) -> [[f64; 2]; 2] {
    let keep = (1.0 + width) / 2.0;
    let cross = (1.0 - width) / 2.0;
    let width_matrix = [[keep, cross], [cross, keep]];
    if !ms_proc {
        return width_matrix;
    }
    let k = std::f64::consts::FRAC_1_SQRT_2;
    let ms = [[k, k], [k, -k]];
    let mut mix = [[0.0; 2]; 2];
    for (row, mix_row) in mix.iter_mut().enumerate() {
        for (col, value) in mix_row.iter_mut().enumerate() {
            *value = width_matrix[row][0] * ms[0][col] + width_matrix[row][1] * ms[1][col];
        }
    }
    mix
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

    #[test]
    fn phase_right_inverts_only_the_right_channel() {
        let mut mixer = Mixer::default();
        mixer.set_phase(SourceKind::Playback, 1, true).unwrap();
        let crosspoints = mixer.render();
        assert_eq!(crosspoints.playbacks[0][0], 16384.0);
        assert_eq!(crosspoints.playbacks[1][1], -16384.0);
    }

    #[test]
    fn width_zero_is_mono_and_minus_one_swaps() {
        let mut mixer = Mixer::default();
        mixer.set_width(SourceKind::Playback, 0, 0.0).unwrap();
        let mono = mixer.render().playbacks;
        for (out, source) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
            assert_eq!(mono[out][source], 8192.0, "out {out} source {source}");
        }
        mixer.set_width(SourceKind::Playback, 0, -1.0).unwrap();
        let swapped = mixer.render().playbacks;
        assert_eq!(swapped[0][1], 16384.0);
        assert_eq!(swapped[1][0], 16384.0);
        assert_eq!(swapped[0][0], 0.0);
        assert_eq!(swapped[1][1], 0.0);
    }

    #[test]
    fn ms_proc_sends_mid_left_and_side_right() {
        let mut mixer = Mixer::default();
        mixer.set_ms_proc(SourceKind::Playback, 0, true).unwrap();
        let crosspoints = mixer.render().playbacks;
        assert_eq!(crosspoints[0][0], 11585.0);
        assert_eq!(crosspoints[0][1], 11585.0);
        assert_eq!(crosspoints[1][0], 11585.0);
        assert_eq!(crosspoints[1][1], -11585.0);
    }

    #[test]
    fn ms_proc_applied_twice_is_the_original() {
        let mix = stereo_mix(1.0, true);
        for row in 0..2 {
            for col in 0..2 {
                let product: f64 = (0..2).map(|k| mix[row][k] * mix[k][col]).sum();
                let expected = if row == col { 1.0 } else { 0.0 };
                assert!((product - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn output_phase_inverts_everything_feeding_that_side() {
        let mut mixer = Mixer::default();
        mixer.set_send_gain(SourceKind::Input, 0, 0, 1.0).unwrap();
        mixer.set_output_phase(1, true).unwrap();
        let crosspoints = mixer.render();
        assert_eq!(crosspoints.playbacks[0][0], 16384.0, "left side untouched");
        assert_eq!(crosspoints.playbacks[1][1], -16384.0);
        assert_eq!(crosspoints.inputs[1][1], -16384.0);
        assert_eq!(crosspoints.outputs[1], 16384.0, "volume stays positive");
    }

    #[test]
    fn output_phase_cancels_source_phase() {
        let mut mixer = Mixer::default();
        mixer.set_phase(SourceKind::Playback, 0, true).unwrap();
        mixer.set_output_phase(0, true).unwrap();
        assert_eq!(mixer.render().playbacks[0][0], 16384.0);
    }

    #[test]
    fn splitting_stereo_resets_width_and_ms() {
        let mut mixer = Mixer::default();
        mixer.set_width(SourceKind::Input, 2, 0.3).unwrap();
        mixer.set_ms_proc(SourceKind::Input, 2, true).unwrap();
        mixer.set_stereo(SourceKind::Input, 2, false).unwrap();
        assert_eq!(mixer.inputs.width[2], 1.0);
        assert!(!mixer.inputs.ms_proc[2]);
    }
}
