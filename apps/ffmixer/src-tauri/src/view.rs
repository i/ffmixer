//! What the UI receives: a flat, display-ready snapshot of the session.

use ffado_dbus::settings::DeviceSettings;
use mixer_core::{OUTPUT_PAIRS, SourceKind, law, names};
use serde::Serialize;

use crate::channel_names::NameRow;
use crate::session::{Mode, Session};

/// Output pair feeding the main monitors (AN 1/2).
pub const MAIN_PAIR: usize = 0;
/// Output pair feeding the front headphone jack (PH 9/10).
pub const PHONES_PAIR: usize = 4;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewState {
    pub mode: &'static str,
    pub device: String,
    pub status: String,
    pub selected_output: usize,
    pub inputs: Vec<StripView>,
    pub playbacks: Vec<StripView>,
    pub outputs: Vec<OutputView>,
    pub any_muted: bool,
    pub any_soloed: bool,
    /// Crosspoints that differ from the unit; `None` in demo mode.
    pub pending_changes: Option<usize>,
    /// True while edits are being sent to the unit.
    pub live: bool,
    /// The unit's device settings; `None` if FFADO would not report them.
    pub device_settings: Option<DeviceSettings>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StripView {
    pub kind: SourceKind,
    pub channel: usize,
    pub stereo: bool,
    pub name: String,
    /// The channel's own name, shown when `name` is a custom one.
    pub default_name: String,
    /// Send level into the selected submix; `None` is −∞.
    pub gain_db: Option<f64>,
    pub pan: f64,
    pub muted: bool,
    pub soloed: bool,
    /// Sends to at least one output.
    pub routed: bool,
    /// Phase inverted, per channel of the strip (one entry if mono).
    pub phase: Vec<bool>,
    /// Stereo width, 1 = normal (stereo strips only).
    pub width: f64,
    pub ms_proc: bool,
    /// Meter indices into the row's meter array.
    pub meters: Vec<usize>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputView {
    pub pair: usize,
    pub name: String,
    pub default_name: String,
    pub volume_db: Option<f64>,
    pub balance: f64,
    pub muted: bool,
    /// Phase L / Phase R.
    pub phase: [bool; 2],
    pub meters: [usize; 2],
}

impl ViewState {
    pub fn from_session(session: &Session) -> Self {
        let mixer = &session.mixer;
        let selected = mixer.selected_output;
        let strips = |kind: SourceKind| {
            let (single, row) = match kind {
                SourceKind::Input => (names::input_name as fn(usize) -> String, NameRow::Input),
                SourceKind::Playback => {
                    (names::output_name as fn(usize) -> String, NameRow::Playback)
                }
            };
            mixer
                .strips(kind)
                .into_iter()
                .map(|strip| {
                    let bank = mixer.bank(kind);
                    let source = &bank.channels[strip.channel];
                    let pair = strip.channel / 2;
                    let channels = if strip.stereo {
                        vec![strip.channel, strip.channel + 1]
                    } else {
                        vec![strip.channel]
                    };
                    let send = source.sends[selected];
                    let default_name = if strip.stereo {
                        names::pair_name(strip.channel, single)
                    } else {
                        single(strip.channel)
                    };
                    StripView {
                        kind,
                        channel: strip.channel,
                        stereo: strip.stereo,
                        name: session
                            .names
                            .get(row, strip.channel)
                            .map_or_else(|| default_name.clone(), str::to_owned),
                        default_name,
                        gain_db: law::gain_to_db(send.gain),
                        pan: send.pan,
                        muted: source.muted,
                        soloed: source.soloed,
                        routed: mixer.is_routed(kind, strip.channel),
                        phase: channels
                            .iter()
                            .map(|&channel| bank.channels[channel].phase_inverted)
                            .collect(),
                        width: bank.width[pair],
                        ms_proc: bank.ms_proc[pair],
                        meters: channels,
                    }
                })
                .collect()
        };
        let outputs = (0..OUTPUT_PAIRS)
            .map(|pair| {
                let output = mixer.outputs[pair];
                let default_name = match pair {
                    MAIN_PAIR => "Main".to_owned(),
                    PHONES_PAIR => "Phones".to_owned(),
                    _ => names::pair_name(2 * pair, names::output_name),
                };
                OutputView {
                    pair,
                    name: session
                        .names
                        .get(NameRow::Output, pair)
                        .map_or_else(|| default_name.clone(), str::to_owned),
                    default_name,
                    volume_db: law::gain_to_db(output.volume),
                    balance: output.balance,
                    muted: output.muted,
                    phase: output.phase_inverted,
                    meters: [2 * pair, 2 * pair + 1],
                }
            })
            .collect();

        Self {
            mode: match session.mode {
                Mode::Loading => "loading",
                Mode::Hardware => "hardware",
                Mode::Demo => "demo",
            },
            device: session.device.clone(),
            status: session.status.clone(),
            selected_output: selected,
            inputs: strips(SourceKind::Input),
            playbacks: strips(SourceKind::Playback),
            outputs,
            any_muted: mixer.any_muted(),
            any_soloed: mixer.any_soloed(),
            pending_changes: session.pending_changes(),
            live: session.is_live(),
            device_settings: session.device_settings,
            warnings: session.warnings.clone(),
        }
    }
}
