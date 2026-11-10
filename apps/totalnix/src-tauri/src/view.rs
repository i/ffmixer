//! What the UI receives: a flat, display-ready snapshot of the session.

use mixer_core::{OUTPUT_PAIRS, SourceKind, law, names};
use serde::Serialize;

use crate::session::{Mode, Session};

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
    pub warnings: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StripView {
    pub kind: SourceKind,
    pub channel: usize,
    pub stereo: bool,
    pub name: String,
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
            let single = match kind {
                SourceKind::Input => names::input_name,
                SourceKind::Playback => names::output_name,
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
                    StripView {
                        kind,
                        channel: strip.channel,
                        stereo: strip.stereo,
                        name: if strip.stereo {
                            names::pair_name(strip.channel, single)
                        } else {
                            single(strip.channel)
                        },
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
                OutputView {
                    pair,
                    name: names::pair_name(2 * pair, names::output_name),
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
            warnings: session.warnings.clone(),
        }
    }
}
