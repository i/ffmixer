//! The controls of RME's "Fireface Settings" dialog (manual ch. 7) that
//! FFADO exposes, as typed values.
//!
//! Everything the app may change on the unit besides mixer gains is a
//! [`Setting`]. A setting is an enum, not a path, so only these controls
//! can be reached, and each one encodes to the raw integer FFADO expects
//! ([`Setting::writes`]). Flash, bandwidth limit, sample rate, nickname
//! and the TCO controls are not representable.
//!
//! FFADO applies a change by rewriting the unit's whole configuration
//! block from its own copy of the settings, and its D-Bus `setValue`
//! echoes the value whatever the outcome. So a write is only trusted
//! after reading the setting back ([`confirm_applied`]).

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// The only D-Bus controls a [`Setting`] ever writes, relative to the
/// device (see [`crate::device_path`]).
pub const WRITABLE_CONTROLS: &[&str] = &[
    CONTROL_PHANTOM,
    CONTROL_CH1_SOURCE,
    CONTROL_CH7_SOURCE,
    CONTROL_CH8_SOURCE,
    CONTROL_INPUT_LEVEL,
    CONTROL_OUTPUT_LEVEL,
    CONTROL_INSTRUMENT,
    CONTROL_SPDIF_INPUT,
    CONTROL_ADAT2_OUTPUT,
    CONTROL_SPDIF_PRO,
    CONTROL_SPDIF_EMPHASIS,
    CONTROL_CLOCK_MODE,
    CONTROL_SYNC_REF,
];

const CONTROL_PHANTOM: &str = "Control/Phantom";
const CONTROL_CH1_SOURCE: &str = "Control/Chan1_source";
const CONTROL_CH7_SOURCE: &str = "Control/Chan7_source";
const CONTROL_CH8_SOURCE: &str = "Control/Chan8_source";
const CONTROL_INPUT_LEVEL: &str = "Control/Input_level";
const CONTROL_OUTPUT_LEVEL: &str = "Control/Output_level";
const CONTROL_INSTRUMENT: &str = "Control/Chan1_instr_opts";
const CONTROL_SPDIF_INPUT: &str = "Control/SPDIF_input_mode";
const CONTROL_ADAT2_OUTPUT: &str = "Control/SPDIF_output_optical";
const CONTROL_SPDIF_PRO: &str = "Control/SPDIF_output_pro";
const CONTROL_SPDIF_EMPHASIS: &str = "Control/SPDIF_output_emphasis";
const CONTROL_CLOCK_MODE: &str = "Control/Clock_mode";
const CONTROL_SYNC_REF: &str = "Control/Sync_ref";

// Read-only status controls.
const CONTROL_SYNC_STATUS: &str = "Control/sync_status";
const CONTROL_SYSCLOCK_MODE: &str = "Control/sysclock_mode";
const CONTROL_SYSCLOCK_FREQ: &str = "Control/sysclock_freq";
const CONTROL_AUTOSYNC_FREQ: &str = "Control/autosync_freq";
const CONTROL_AUTOSYNC_SRC: &str = "Control/autosync_src";
const CONTROL_SPDIF_FREQ: &str = "Control/spdif_freq";
const CONTROL_TCO_PRESENT: &str = "Control/TCO_present";

/// Instrument option bits of `Chan1_instr_opts` (FFADO `FF800_INSTR_OPT_*`).
const INSTR_SPEAKER_EMULATION: i32 = 0x02;
const INSTR_DRIVE: i32 = 0x04;
const INSTR_LIMITER: i32 = 0x08;

/// Generates `raw()` / `from_raw()` for an enum whose variants map to
/// FFADO's integer values.
macro_rules! raw_values {
    ($ty:ident { $($variant:ident = $raw:literal),+ $(,)? }) => {
        impl $ty {
            fn raw(self) -> i32 {
                match self {
                    $($ty::$variant => $raw),+
                }
            }

            fn from_raw(raw: i32) -> Result<Self> {
                match raw {
                    $($raw => Ok($ty::$variant),)+
                    other => Err(Error::Unexpected(format!(
                        concat!("FFADO reports {} for ", stringify!($ty)),
                        other
                    ))),
                }
            }
        }
    };
}

/// A microphone preamp with phantom power. The numbers are the unit's
/// channel numbers; FFADO counts them 0–3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mic {
    Mic7,
    Mic8,
    Mic9,
    Mic10,
}

impl Mic {
    pub const fn index(self) -> usize {
        match self {
            Mic::Mic7 => 0,
            Mic::Mic8 => 1,
            Mic::Mic9 => 2,
            Mic::Mic10 => 3,
        }
    }
}

/// An input with a front/rear jack choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputChannel {
    Ch1,
    Ch7,
    Ch8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputSource {
    Front,
    Rear,
    FrontAndRear,
}
raw_values!(InputSource {
    Front = 0,
    Rear = 1,
    FrontAndRear = 2,
});

/// Reference level of the analog line inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputLevel {
    LoGain,
    Minus10dBV,
    Plus4dBu,
}
raw_values!(InputLevel {
    LoGain = 0,
    Minus10dBV = 1,
    Plus4dBu = 2,
});

/// Reference level of the analog line outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputLevel {
    Minus10dBV,
    Plus4dBu,
    HiGain,
}
raw_values!(OutputLevel {
    Minus10dBV = 0,
    Plus4dBu = 1,
    HiGain = 2,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpdifInput {
    Coax,
    Optical,
}
raw_values!(SpdifInput {
    Coax = 0,
    Optical = 1,
});

/// What the second optical output carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Adat2Output {
    Adat,
    Spdif,
}
raw_values!(Adat2Output {
    Adat = 0,
    Spdif = 1,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpdifFormat {
    Consumer,
    Professional,
}
raw_values!(SpdifFormat {
    Consumer = 0,
    Professional = 1,
});

/// Where the unit takes its clock from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClockSource {
    Internal,
    Word,
    Adat1,
    Adat2,
    Spdif,
}

impl ClockSource {
    /// FFADO's `Sync_ref` value for an external source.
    const fn sync_ref(self) -> Option<i32> {
        match self {
            ClockSource::Internal => None,
            ClockSource::Word => Some(0),
            ClockSource::Adat1 => Some(1),
            ClockSource::Adat2 => Some(2),
            ClockSource::Spdif => Some(3),
        }
    }

    /// `None` for the TCO (sync ref 4), which this app does not support.
    fn from_raw(
        clock_mode: i32,
        sync_ref: i32,
    ) -> Result<Option<Self>> {
        match (clock_mode, sync_ref) {
            (0, _) => Ok(Some(ClockSource::Internal)),
            (1, 0) => Ok(Some(ClockSource::Word)),
            (1, 1) => Ok(Some(ClockSource::Adat1)),
            (1, 2) => Ok(Some(ClockSource::Adat2)),
            (1, 3) => Ok(Some(ClockSource::Spdif)),
            (1, 4) => Ok(None),
            (mode, reference) => Err(Error::Unexpected(format!(
                "FFADO reports clock mode {mode}, sync reference {reference}"
            ))),
        }
    }
}

/// Input 1's instrument options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    /// Adds gain and distortion (manual: +25 dB, "Drive").
    pub drive: bool,
    /// Soft limiter. Only switchable with input 1 on the front jack;
    /// otherwise the unit keeps it on.
    pub limiter: bool,
    pub speaker_emulation: bool,
}

impl Instrument {
    fn from_bits(bits: i32) -> Self {
        Self {
            drive: bits & INSTR_DRIVE != 0,
            limiter: bits & INSTR_LIMITER != 0,
            speaker_emulation: bits & INSTR_SPEAKER_EMULATION != 0,
        }
    }

    fn bits(self) -> i32 {
        let flag = |on: bool, bit: i32| if on { bit } else { 0 };
        flag(self.drive, INSTR_DRIVE)
            | flag(self.limiter, INSTR_LIMITER)
            | flag(self.speaker_emulation, INSTR_SPEAKER_EMULATION)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct InputSources {
    pub ch1: InputSource,
    pub ch7: InputSource,
    pub ch8: InputSource,
}

impl InputSources {
    fn get_mut(
        &mut self,
        channel: InputChannel,
    ) -> &mut InputSource {
        match channel {
            InputChannel::Ch1 => &mut self.ch1,
            InputChannel::Ch7 => &mut self.ch7,
            InputChannel::Ch8 => &mut self.ch8,
        }
    }
}

/// The unit's settings as FFADO holds them. FFADO loaded these from the
/// unit's flash when it started and sent them to the unit, so they match
/// the hardware unless something else has changed it since.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSettings {
    pub input_sources: InputSources,
    pub input_level: InputLevel,
    pub output_level: OutputLevel,
    /// Phantom power for Mic 7–10. `None` is unknown: FFADO's getter
    /// skips Mic 10, so it is only known after we have set it.
    pub phantom: [Option<bool>; 4],
    pub instrument: Instrument,
    pub spdif_input: SpdifInput,
    pub adat2_output: Adat2Output,
    pub spdif_format: SpdifFormat,
    pub emphasis: bool,
    /// `None` if the unit is set to the TCO.
    pub clock_source: Option<ClockSource>,
}

impl DeviceSettings {
    /// Reads every setting through `get`, which takes a control name
    /// relative to the device and returns its D-Bus value.
    pub fn read_with(get: impl Fn(&str) -> Result<i32>) -> Result<Self> {
        let phantom_bits = get(CONTROL_PHANTOM)?;
        let flag = |raw: i32| raw != 0;
        Ok(Self {
            input_sources: InputSources {
                ch1: InputSource::from_raw(get(CONTROL_CH1_SOURCE)?)?,
                ch7: InputSource::from_raw(get(CONTROL_CH7_SOURCE)?)?,
                ch8: InputSource::from_raw(get(CONTROL_CH8_SOURCE)?)?,
            },
            input_level: InputLevel::from_raw(get(CONTROL_INPUT_LEVEL)?)?,
            output_level: OutputLevel::from_raw(get(CONTROL_OUTPUT_LEVEL)?)?,
            phantom: [
                Some(phantom_bits & 0b001 != 0),
                Some(phantom_bits & 0b010 != 0),
                Some(phantom_bits & 0b100 != 0),
                None,
            ],
            instrument: Instrument::from_bits(get(CONTROL_INSTRUMENT)?),
            spdif_input: SpdifInput::from_raw(get(CONTROL_SPDIF_INPUT)?)?,
            adat2_output: Adat2Output::from_raw(get(CONTROL_ADAT2_OUTPUT)?)?,
            spdif_format: SpdifFormat::from_raw(get(CONTROL_SPDIF_PRO)?)?,
            emphasis: flag(get(CONTROL_SPDIF_EMPHASIS)?),
            clock_source: ClockSource::from_raw(get(CONTROL_CLOCK_MODE)?, get(CONTROL_SYNC_REF)?)?,
        })
    }

    /// Fills in what FFADO cannot report (Mic 10 phantom) from an
    /// earlier snapshot.
    pub fn keep_unreadable_from(
        &mut self,
        previous: &DeviceSettings,
    ) {
        if self.phantom[Mic::Mic10.index()].is_none() {
            self.phantom[Mic::Mic10.index()] = previous.phantom[Mic::Mic10.index()];
        }
    }
}

/// One change to the unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "control",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Setting {
    InputSource {
        channel: InputChannel,
        source: InputSource,
    },
    InputLevel {
        level: InputLevel,
    },
    OutputLevel {
        level: OutputLevel,
    },
    Phantom {
        mic: Mic,
        on: bool,
    },
    Drive {
        on: bool,
    },
    Limiter {
        on: bool,
    },
    SpeakerEmulation {
        on: bool,
    },
    SpdifInput {
        input: SpdifInput,
    },
    Adat2Output {
        output: Adat2Output,
    },
    SpdifFormat {
        format: SpdifFormat,
    },
    Emphasis {
        on: bool,
    },
    Clock {
        source: ClockSource,
    },
}

/// One D-Bus `setValue` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlWrite {
    pub control: &'static str,
    pub value: i32,
}

/// Why a change needs the user's confirmation first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    /// 48 V can destroy some microphones and sources.
    Phantom,
    /// The signal level at the unit's analog stages may rise sharply.
    Louder,
    /// Audio may drop out, or the unit lose its clock.
    Clock,
    /// The change rewrites input 1's analog path bits; the level can jump.
    InputPath,
}

impl Risk {
    pub const fn warning(self) -> &'static str {
        match self {
            Risk::Phantom => {
                "48 V phantom power can damage ribbon microphones and some unbalanced or active \
                 sources. Turn it on only for a condenser microphone or a device that needs it."
            }
            Risk::Louder => {
                "This can make the signal much louder, at the outputs and in headphones. Turn \
                 monitors and headphones down first."
            }
            Risk::InputPath => {
                "This switches the analog path of input 1, and the level can jump, in either \
                 direction (seen with Speaker Emulation on a Fireface 800). Turn monitors and \
                 headphones down first."
            }
            Risk::Clock => {
                "Changing the clock source can interrupt audio, and the unit loses its clock if \
                 the chosen source has no signal. Check the lock status first."
            }
        }
    }
}

impl Setting {
    /// Checks that the unit can honour this change in its current state.
    pub fn validate(
        &self,
        current: &DeviceSettings,
    ) -> Result<()> {
        if matches!(self, Setting::Limiter { .. })
            && current.input_sources.ch1 != InputSource::Front
        {
            return Err(Error::Rejected(
                "the limiter can only be switched with input 1 on the front jack".to_owned(),
            ));
        }
        Ok(())
    }

    /// The D-Bus writes that make this change, in order. `current` is
    /// needed because FFADO packs the instrument options into one value.
    pub fn writes(
        &self,
        current: &DeviceSettings,
    ) -> Vec<ControlWrite> {
        let write = |control: &'static str, value: i32| ControlWrite { control, value };
        let instrument = |change: &dyn Fn(&mut Instrument)| {
            let mut options = current.instrument;
            change(&mut options);
            vec![write(CONTROL_INSTRUMENT, options.bits())]
        };
        match *self {
            Setting::InputSource { channel, source } => {
                let control = match channel {
                    InputChannel::Ch1 => CONTROL_CH1_SOURCE,
                    InputChannel::Ch7 => CONTROL_CH7_SOURCE,
                    InputChannel::Ch8 => CONTROL_CH8_SOURCE,
                };
                vec![write(control, source.raw())]
            }
            Setting::InputLevel { level } => vec![write(CONTROL_INPUT_LEVEL, level.raw())],
            Setting::OutputLevel { level } => vec![write(CONTROL_OUTPUT_LEVEL, level.raw())],
            Setting::Phantom { mic, on } => {
                // Low 16 bits: state. High 16 bits: which channels to
                // change, so the others are left alone.
                let bit = 1 << mic.index();
                let state = if on { bit } else { 0 };
                vec![write(CONTROL_PHANTOM, state | (bit << 16))]
            }
            Setting::Drive { on } => instrument(&|o| o.drive = on),
            Setting::Limiter { on } => instrument(&|o| o.limiter = on),
            Setting::SpeakerEmulation { on } => instrument(&|o| o.speaker_emulation = on),
            Setting::SpdifInput { input } => vec![write(CONTROL_SPDIF_INPUT, input.raw())],
            Setting::Adat2Output { output } => vec![write(CONTROL_ADAT2_OUTPUT, output.raw())],
            Setting::SpdifFormat { format } => vec![write(CONTROL_SPDIF_PRO, format.raw())],
            Setting::Emphasis { on } => vec![write(CONTROL_SPDIF_EMPHASIS, on as i32)],
            Setting::Clock { source } => match source.sync_ref() {
                // Internal: leave the reference as it is.
                None => vec![write(CONTROL_CLOCK_MODE, 0)],
                // External: pick the reference before following it.
                Some(reference) => vec![
                    write(CONTROL_SYNC_REF, reference),
                    write(CONTROL_CLOCK_MODE, 1),
                ],
            },
        }
    }

    /// Records this change in a snapshot, as FFADO does.
    pub fn apply_to(
        &self,
        settings: &mut DeviceSettings,
    ) {
        match *self {
            Setting::InputSource { channel, source } => {
                *settings.input_sources.get_mut(channel) = source;
            }
            Setting::InputLevel { level } => settings.input_level = level,
            Setting::OutputLevel { level } => settings.output_level = level,
            Setting::Phantom { mic, on } => settings.phantom[mic.index()] = Some(on),
            Setting::Drive { on } => settings.instrument.drive = on,
            Setting::Limiter { on } => settings.instrument.limiter = on,
            Setting::SpeakerEmulation { on } => settings.instrument.speaker_emulation = on,
            Setting::SpdifInput { input } => settings.spdif_input = input,
            Setting::Adat2Output { output } => settings.adat2_output = output,
            Setting::SpdifFormat { format } => settings.spdif_format = format,
            Setting::Emphasis { on } => settings.emphasis = on,
            Setting::Clock { source } => settings.clock_source = Some(source),
        }
    }

    /// Whether the user must confirm this change first, given what the
    /// unit holds now. Setting a value it already has is never risky.
    pub fn risk(
        &self,
        current: &DeviceSettings,
    ) -> Option<Risk> {
        let mut after = *current;
        self.apply_to(&mut after);
        if after == *current {
            return None;
        }
        match *self {
            Setting::Phantom { on: true, .. } => Some(Risk::Phantom),
            Setting::InputLevel { .. }
            | Setting::OutputLevel { .. }
            | Setting::Drive { on: true }
            | Setting::Limiter { on: false } => Some(Risk::Louder),
            Setting::Drive { on: false }
            | Setting::Limiter { on: true }
            | Setting::SpeakerEmulation { .. } => Some(Risk::InputPath),
            Setting::Clock { .. } => Some(Risk::Clock),
            _ => None,
        }
    }
}

/// Checks that a read-back (`after`) shows `setting` applied to `before`
/// and nothing else changed, and returns the settings to keep.
pub fn confirm_applied(
    setting: &Setting,
    before: &DeviceSettings,
    mut after: DeviceSettings,
) -> Result<DeviceSettings> {
    let mut expected = *before;
    setting.apply_to(&mut expected);
    after.keep_unreadable_from(&expected);
    if after == expected {
        Ok(after)
    } else {
        Err(Error::Rejected(format!(
            "FFADO did not apply {setting:?}: read back {after:?}"
        )))
    }
}

/// Lock state of one clock input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Lock {
    NoLock,
    Locked,
    Synced,
}

impl Lock {
    fn from_raw(bits: i32) -> Self {
        match bits & 0b11 {
            1 => Lock::Locked,
            2 => Lock::Synced,
            _ => Lock::NoLock,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncSource {
    Adat1,
    Adat2,
    Spdif,
    Word,
    Tco,
}

/// The unit's clock and input lock state (read-only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockStatus {
    /// Rate the unit is running at, in Hz.
    pub sample_rate: u32,
    /// True if the unit is its own clock master.
    pub master: bool,
    /// The input the unit currently follows, if any.
    pub autosync_source: Option<SyncSource>,
    pub autosync_rate: u32,
    pub spdif_rate: u32,
    pub word: Lock,
    pub spdif: Lock,
    pub adat1: Lock,
    pub adat2: Lock,
    /// `None` unless the optional TCO card is fitted.
    pub tco: Option<Lock>,
}

impl ClockStatus {
    /// Reads the status through `get`; see [`DeviceSettings::read_with`].
    pub fn read_with(get: impl Fn(&str) -> Result<i32>) -> Result<Self> {
        let sync = get(CONTROL_SYNC_STATUS)?;
        let tco_present = get(CONTROL_TCO_PRESENT)? != 0;
        let hertz = |raw: i32| raw.max(0) as u32;
        let source = match get(CONTROL_AUTOSYNC_SRC)? {
            0 => None,
            1 => Some(SyncSource::Adat1),
            2 => Some(SyncSource::Adat2),
            3 => Some(SyncSource::Spdif),
            4 => Some(SyncSource::Word),
            5 => Some(SyncSource::Tco),
            other => {
                return Err(Error::Unexpected(format!(
                    "FFADO reports autosync source {other}"
                )));
            }
        };
        // FFADO reports 44.1 kHz on the S/PDIF input as 41000.
        let spdif_rate = match hertz(get(CONTROL_SPDIF_FREQ)?) {
            41000 => 44100,
            rate => rate,
        };
        Ok(Self {
            sample_rate: hertz(get(CONTROL_SYSCLOCK_FREQ)?),
            master: get(CONTROL_SYSCLOCK_MODE)? == 0,
            autosync_source: source,
            autosync_rate: hertz(get(CONTROL_AUTOSYNC_FREQ)?),
            spdif_rate,
            adat1: Lock::from_raw(sync),
            adat2: Lock::from_raw(sync >> 2),
            spdif: Lock::from_raw(sync >> 4),
            word: Lock::from_raw(sync >> 6),
            tco: tco_present.then(|| Lock::from_raw(sync >> 8)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn settings() -> DeviceSettings {
        DeviceSettings {
            input_sources: InputSources {
                ch1: InputSource::Front,
                ch7: InputSource::Rear,
                ch8: InputSource::Front,
            },
            input_level: InputLevel::Plus4dBu,
            output_level: OutputLevel::Plus4dBu,
            phantom: [Some(false), Some(false), Some(false), None],
            instrument: Instrument {
                drive: false,
                limiter: true,
                speaker_emulation: false,
            },
            spdif_input: SpdifInput::Optical,
            adat2_output: Adat2Output::Adat,
            spdif_format: SpdifFormat::Consumer,
            emphasis: false,
            clock_source: Some(ClockSource::Internal),
        }
    }

    fn json(text: &str) -> Setting {
        serde_json::from_str(text).unwrap()
    }

    fn all_settings() -> Vec<Setting> {
        let mut all = vec![
            Setting::Drive { on: true },
            Setting::Limiter { on: false },
            Setting::SpeakerEmulation { on: true },
            Setting::Emphasis { on: true },
        ];
        for mic in [Mic::Mic7, Mic::Mic8, Mic::Mic9, Mic::Mic10] {
            all.push(Setting::Phantom { mic, on: true });
        }
        for channel in [InputChannel::Ch1, InputChannel::Ch7, InputChannel::Ch8] {
            for source in [
                InputSource::Front,
                InputSource::Rear,
                InputSource::FrontAndRear,
            ] {
                all.push(Setting::InputSource { channel, source });
            }
        }
        for level in [
            InputLevel::LoGain,
            InputLevel::Minus10dBV,
            InputLevel::Plus4dBu,
        ] {
            all.push(Setting::InputLevel { level });
        }
        for level in [
            OutputLevel::Minus10dBV,
            OutputLevel::Plus4dBu,
            OutputLevel::HiGain,
        ] {
            all.push(Setting::OutputLevel { level });
        }
        for input in [SpdifInput::Coax, SpdifInput::Optical] {
            all.push(Setting::SpdifInput { input });
        }
        for output in [Adat2Output::Adat, Adat2Output::Spdif] {
            all.push(Setting::Adat2Output { output });
        }
        for format in [SpdifFormat::Consumer, SpdifFormat::Professional] {
            all.push(Setting::SpdifFormat { format });
        }
        for source in [
            ClockSource::Internal,
            ClockSource::Word,
            ClockSource::Adat1,
            ClockSource::Adat2,
            ClockSource::Spdif,
        ] {
            all.push(Setting::Clock { source });
        }
        all
    }

    #[test]
    fn writes_only_whitelisted_controls() {
        let forbidden = [
            "Control/Flash_control",
            "Control/Bandwidth_limit",
            "Control/Mixer_preset",
            "Generic/SamplerateSelect",
            "Generic/ClockSelect",
            "Generic/Nickname",
        ];
        for setting in all_settings() {
            for write in setting.writes(&settings()) {
                assert!(
                    WRITABLE_CONTROLS.contains(&write.control),
                    "{setting:?} writes {}",
                    write.control
                );
                assert!(!forbidden.contains(&write.control));
                assert!(!write.control.starts_with("Control/Tco_"));
                assert!(!write.control.starts_with("Mixer/"));
            }
        }
    }

    #[test]
    fn phantom_changes_only_its_own_channel() {
        let on = Setting::Phantom {
            mic: Mic::Mic9,
            on: true,
        };
        let off = Setting::Phantom {
            mic: Mic::Mic9,
            on: false,
        };
        assert_eq!(
            on.writes(&settings()),
            [ControlWrite {
                control: "Control/Phantom",
                value: 0x0004_0004
            }]
        );
        assert_eq!(
            off.writes(&settings()),
            [ControlWrite {
                control: "Control/Phantom",
                value: 0x0004_0000
            }]
        );
        let mic10 = Setting::Phantom {
            mic: Mic::Mic10,
            on: true,
        };
        assert_eq!(mic10.writes(&settings())[0].value, 0x0008_0008);
    }

    #[test]
    fn instrument_options_keep_the_other_bits() {
        let mut current = settings();
        current.instrument = Instrument {
            drive: false,
            limiter: true,
            speaker_emulation: true,
        };
        let drive = Setting::Drive { on: true }.writes(&current)[0].value;
        assert_eq!(drive, 0x04 | 0x08 | 0x02);
        let limiter_off = Setting::Limiter { on: false }.writes(&current)[0].value;
        assert_eq!(limiter_off, 0x02);
        let emulation_off = Setting::SpeakerEmulation { on: false }.writes(&current)[0].value;
        assert_eq!(emulation_off, 0x08);
    }

    #[test]
    fn clock_picks_the_reference_before_following_it() {
        let writes = Setting::Clock {
            source: ClockSource::Adat2,
        }
        .writes(&settings());
        assert_eq!(
            writes,
            [
                ControlWrite {
                    control: "Control/Sync_ref",
                    value: 2
                },
                ControlWrite {
                    control: "Control/Clock_mode",
                    value: 1
                },
            ]
        );
        let internal = Setting::Clock {
            source: ClockSource::Internal,
        }
        .writes(&settings());
        assert_eq!(
            internal,
            [ControlWrite {
                control: "Control/Clock_mode",
                value: 0
            }]
        );
    }

    #[test]
    fn raw_values_match_ffados_panel() {
        // From ffado's rme.py radio buttons.
        assert_eq!(InputLevel::LoGain.raw(), 0);
        assert_eq!(InputLevel::Minus10dBV.raw(), 1);
        assert_eq!(InputLevel::Plus4dBu.raw(), 2);
        assert_eq!(OutputLevel::Minus10dBV.raw(), 0);
        assert_eq!(OutputLevel::Plus4dBu.raw(), 1);
        assert_eq!(OutputLevel::HiGain.raw(), 2);
        assert_eq!(SpdifInput::Coax.raw(), 0);
        assert_eq!(SpdifInput::Optical.raw(), 1);
        assert!(InputLevel::from_raw(3).is_err());
    }

    #[test]
    fn risky_changes_need_confirmation() {
        let current = settings();
        let risk = |setting: Setting| setting.risk(&current);
        assert_eq!(
            risk(Setting::Phantom {
                mic: Mic::Mic7,
                on: true
            }),
            Some(Risk::Phantom)
        );
        assert_eq!(
            risk(Setting::OutputLevel {
                level: OutputLevel::HiGain
            }),
            Some(Risk::Louder)
        );
        assert_eq!(
            risk(Setting::InputLevel {
                level: InputLevel::LoGain
            }),
            Some(Risk::Louder)
        );
        assert_eq!(risk(Setting::Drive { on: true }), Some(Risk::Louder));
        assert_eq!(risk(Setting::Limiter { on: false }), Some(Risk::Louder));
        // Changing any instrument option rewrites input 1's analog path.
        // The fixture has drive off, limiter on and speaker emulation off.
        assert_eq!(
            risk(Setting::SpeakerEmulation { on: true }),
            Some(Risk::InputPath)
        );
        let mut emulating = current;
        emulating.instrument.speaker_emulation = true;
        assert_eq!(
            Setting::SpeakerEmulation { on: false }.risk(&emulating),
            Some(Risk::InputPath)
        );
        let mut driven = current;
        driven.instrument.drive = true;
        assert_eq!(
            Setting::Drive { on: false }.risk(&driven),
            Some(Risk::InputPath)
        );
        assert_eq!(risk(Setting::Drive { on: false }), None);
        assert_eq!(
            risk(Setting::Clock {
                source: ClockSource::Word
            }),
            Some(Risk::Clock)
        );
    }

    #[test]
    fn safe_and_unchanged_settings_need_no_confirmation() {
        let current = settings();
        let risk = |setting: Setting| setting.risk(&current);
        // Phantom off, and settings that cannot raise a level.
        assert_eq!(
            risk(Setting::Phantom {
                mic: Mic::Mic8,
                on: false
            }),
            None
        );
        assert_eq!(
            risk(Setting::SpdifInput {
                input: SpdifInput::Coax
            }),
            None
        );
        assert_eq!(
            risk(Setting::InputSource {
                channel: InputChannel::Ch7,
                source: InputSource::Front
            }),
            None
        );
        // Same value as now, even for the risky controls.
        assert_eq!(
            risk(Setting::OutputLevel {
                level: OutputLevel::Plus4dBu
            }),
            None
        );
        assert_eq!(
            risk(Setting::Clock {
                source: ClockSource::Internal
            }),
            None
        );
    }

    #[test]
    fn limiter_needs_input_one_on_the_front_jack() {
        let mut current = settings();
        let limiter = Setting::Limiter { on: false };
        assert!(limiter.validate(&current).is_ok());
        current.input_sources.ch1 = InputSource::Rear;
        assert!(limiter.validate(&current).is_err());
    }

    #[test]
    fn settings_deserialize_from_the_ui_json() {
        assert_eq!(
            json(r#"{"control":"phantom","mic":"mic10","on":true}"#),
            Setting::Phantom {
                mic: Mic::Mic10,
                on: true
            }
        );
        assert_eq!(
            json(r#"{"control":"inputLevel","level":"minus10dBV"}"#),
            Setting::InputLevel {
                level: InputLevel::Minus10dBV
            }
        );
        assert_eq!(
            json(r#"{"control":"inputSource","channel":"ch7","source":"frontAndRear"}"#),
            Setting::InputSource {
                channel: InputChannel::Ch7,
                source: InputSource::FrontAndRear
            }
        );
        assert_eq!(
            json(r#"{"control":"clock","source":"adat1"}"#),
            Setting::Clock {
                source: ClockSource::Adat1
            }
        );
        assert_eq!(
            json(r#"{"control":"speakerEmulation","on":false}"#),
            Setting::SpeakerEmulation { on: false }
        );
    }

    #[test]
    fn invalid_settings_do_not_deserialize() {
        for text in [
            r#"{"control":"phantom","mic":"mic11","on":true}"#,
            r#"{"control":"phantom","mic":"mic7"}"#,
            r#"{"control":"flashControl","value":1}"#,
            r#"{"control":"outputLevel","level":"plus20dBu"}"#,
            r#"{"control":"sampleRate","rate":96000}"#,
        ] {
            assert!(serde_json::from_str::<Setting>(text).is_err(), "{text}");
        }
    }

    fn fake_values(overrides: &[(&str, i32)]) -> HashMap<String, i32> {
        let mut values: HashMap<String, i32> = [
            ("Control/Phantom", 0),
            ("Control/Chan1_source", 0),
            ("Control/Chan7_source", 1),
            ("Control/Chan8_source", 0),
            ("Control/Input_level", 2),
            ("Control/Output_level", 1),
            ("Control/Chan1_instr_opts", 0x08),
            ("Control/SPDIF_input_mode", 1),
            ("Control/SPDIF_output_optical", 0),
            ("Control/SPDIF_output_pro", 0),
            ("Control/SPDIF_output_emphasis", 0),
            ("Control/Clock_mode", 0),
            ("Control/Sync_ref", 0),
        ]
        .map(|(k, v)| (k.to_owned(), v))
        .into();
        for (key, value) in overrides {
            values.insert((*key).to_owned(), *value);
        }
        values
    }

    fn getter(values: &HashMap<String, i32>) -> impl Fn(&str) -> Result<i32> + '_ {
        |control| {
            values
                .get(control)
                .copied()
                .ok_or_else(|| Error::Unexpected(format!("no fake value for {control}")))
        }
    }

    #[test]
    fn reads_every_setting() {
        let values = fake_values(&[("Control/Phantom", 0b101)]);
        let read = DeviceSettings::read_with(getter(&values)).unwrap();
        let mut expected = settings();
        expected.phantom = [Some(true), Some(false), Some(true), None];
        assert_eq!(read, expected);
    }

    #[test]
    fn reads_external_clock_and_unsupported_tco() {
        let wordclock = fake_values(&[("Control/Clock_mode", 1), ("Control/Sync_ref", 0)]);
        assert_eq!(
            DeviceSettings::read_with(getter(&wordclock))
                .unwrap()
                .clock_source,
            Some(ClockSource::Word)
        );
        let tco = fake_values(&[("Control/Clock_mode", 1), ("Control/Sync_ref", 4)]);
        assert_eq!(
            DeviceSettings::read_with(getter(&tco))
                .unwrap()
                .clock_source,
            None
        );
    }

    #[test]
    fn unexpected_values_are_errors() {
        let values = fake_values(&[("Control/Output_level", 7)]);
        assert!(DeviceSettings::read_with(getter(&values)).is_err());
    }

    #[test]
    fn confirm_accepts_the_expected_read_back() {
        let before = settings();
        let setting = Setting::OutputLevel {
            level: OutputLevel::HiGain,
        };
        let mut after = before;
        after.output_level = OutputLevel::HiGain;
        assert_eq!(confirm_applied(&setting, &before, after).unwrap(), after);
    }

    #[test]
    fn confirm_rejects_a_read_back_that_differs() {
        let before = settings();
        let setting = Setting::OutputLevel {
            level: OutputLevel::HiGain,
        };
        // Nothing changed.
        assert!(confirm_applied(&setting, &before, before).is_err());
        // Something else changed too.
        let mut after = before;
        after.output_level = OutputLevel::HiGain;
        after.input_level = InputLevel::LoGain;
        assert!(confirm_applied(&setting, &before, after).is_err());
    }

    #[test]
    fn mic_ten_phantom_survives_read_back() {
        let before = settings();
        let setting = Setting::Phantom {
            mic: Mic::Mic10,
            on: true,
        };
        // FFADO's read-back never includes Mic 10.
        let after = before;
        let kept = confirm_applied(&setting, &before, after).unwrap();
        assert_eq!(kept.phantom[3], Some(true));
    }

    #[test]
    fn decodes_the_clock_status() {
        let mut values = fake_values(&[]);
        for (key, value) in [
            ("Control/sync_status", 0b01_10_00_10),
            ("Control/TCO_present", 0),
            ("Control/autosync_src", 1),
            ("Control/autosync_freq", 48000),
            ("Control/sysclock_mode", 1),
            ("Control/sysclock_freq", 48000),
            ("Control/spdif_freq", 41000),
        ] {
            values.insert(key.to_owned(), value);
        }
        let status = ClockStatus::read_with(getter(&values)).unwrap();
        assert_eq!(status.adat1, Lock::Synced);
        assert_eq!(status.adat2, Lock::NoLock);
        assert_eq!(status.spdif, Lock::Synced);
        assert_eq!(status.word, Lock::Locked);
        assert_eq!(status.tco, None);
        assert_eq!(status.autosync_source, Some(SyncSource::Adat1));
        assert!(!status.master);
        assert_eq!(status.sample_rate, 48000);
        assert_eq!(status.spdif_rate, 44100);
    }
}
