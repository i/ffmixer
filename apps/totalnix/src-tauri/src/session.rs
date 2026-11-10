//! The app's mixer session: the model, where it came from, and the
//! actions the UI may apply to it.
//!
//! Writes to the hardware are **not implemented**. Every action only
//! changes the in-memory model; `pending_changes` reports how many
//! crosspoints would change on the unit if writes were enabled.

use mixer_core::{Crosspoints, Mixer, SourceKind, law};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Reading the mix from `ffado-dbus-server`.
    Loading,
    /// Mix adopted from the unit; meters are live; nothing is written.
    Hardware,
    /// No unit available: default mix and simulated meters.
    Demo,
}

pub struct Session {
    pub mode: Mode,
    pub device: String,
    pub status: String,
    pub mixer: Mixer,
    /// Crosspoints read from the unit, for the pending-changes count.
    pub hardware: Option<Crosspoints>,
    pub warnings: Vec<String>,
    /// Strips whose mute/solo the master buttons switched off, so the
    /// next click can restore them (manual §25.5).
    mute_memory: Vec<(SourceKind, usize)>,
    solo_memory: Vec<(SourceKind, usize)>,
}

/// Everything the UI can ask for. Send actions edit the selected submix.
#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    SelectOutput {
        output: usize,
    },
    SetSendGain {
        kind: SourceKind,
        channel: usize,
        db: Option<f64>,
    },
    SetSendPan {
        kind: SourceKind,
        channel: usize,
        pan: f64,
    },
    SetMuted {
        kind: SourceKind,
        channel: usize,
        value: bool,
    },
    SetSoloed {
        kind: SourceKind,
        channel: usize,
        value: bool,
    },
    SetStereo {
        kind: SourceKind,
        pair: usize,
        value: bool,
    },
    SetOutputVolume {
        output: usize,
        db: Option<f64>,
    },
    SetOutputBalance {
        output: usize,
        pan: f64,
    },
    SetOutputMuted {
        output: usize,
        value: bool,
    },
    ToggleMuteMaster,
    ToggleSoloMaster,
}

impl Session {
    pub fn loading() -> Self {
        Self {
            mode: Mode::Loading,
            device: String::new(),
            status: "Connecting to ffado-dbus-server…".to_owned(),
            mixer: Mixer::default(),
            hardware: None,
            warnings: Vec::new(),
            mute_memory: Vec::new(),
            solo_memory: Vec::new(),
        }
    }

    pub fn adopt_hardware(
        &mut self,
        guid: String,
        crosspoints: Crosspoints,
    ) {
        let (mixer, report) = mixer_core::import(&crosspoints);
        self.mode = Mode::Hardware;
        self.status = "Read-only: changes are not sent to the unit".to_owned();
        self.device = guid;
        self.mixer = mixer;
        self.hardware = Some(crosspoints);
        self.warnings = report.warnings;
    }

    pub fn enter_demo(
        &mut self,
        reason: String,
    ) {
        self.mode = Mode::Demo;
        self.status = format!("Demo mode: {reason}");
        self.device = "Demo".to_owned();
        self.mixer = Mixer::default();
        self.hardware = None;
        self.warnings.clear();
    }

    /// Crosspoints the unit would need to change to match the model.
    pub fn pending_changes(&self) -> Option<usize> {
        self.hardware
            .as_ref()
            .map(|hardware| self.mixer.render().differences(hardware))
    }

    pub fn apply(
        &mut self,
        action: Action,
    ) -> Result<(), String> {
        if self.mode == Mode::Loading {
            return Err("still loading the mix from the unit".to_owned());
        }
        let mixer = &mut self.mixer;
        let output = mixer.selected_output;
        let result = match action {
            Action::SelectOutput { output } => mixer.select_output(output),
            Action::SetSendGain { kind, channel, db } => {
                mixer.set_send_gain(kind, channel, output, law::db_to_gain(db))
            }
            Action::SetSendPan { kind, channel, pan } => {
                mixer.set_send_pan(kind, channel, output, pan)
            }
            Action::SetMuted {
                kind,
                channel,
                value,
            } => {
                self.mute_memory.clear();
                mixer.set_muted(kind, channel, value)
            }
            Action::SetSoloed {
                kind,
                channel,
                value,
            } => {
                self.solo_memory.clear();
                mixer.set_soloed(kind, channel, value)
            }
            Action::SetStereo { kind, pair, value } => mixer.set_stereo(kind, pair, value),
            Action::SetOutputVolume { output, db } => {
                mixer.set_output_volume(output, law::db_to_gain(db))
            }
            Action::SetOutputBalance { output, pan } => mixer.set_output_balance(output, pan),
            Action::SetOutputMuted { output, value } => mixer.set_output_muted(output, value),
            Action::ToggleMuteMaster => {
                toggle_master(mixer, &mut self.mute_memory, |source| &mut source.muted);
                Ok(())
            }
            Action::ToggleSoloMaster => {
                toggle_master(mixer, &mut self.solo_memory, |source| &mut source.soloed);
                Ok(())
            }
        };
        result.map_err(|e| e.to_string())
    }
}

/// Master button: if any flag is set, remember and clear them all;
/// otherwise restore the remembered set.
fn toggle_master(
    mixer: &mut Mixer,
    memory: &mut Vec<(SourceKind, usize)>,
    flag: fn(&mut mixer_core::Source) -> &mut bool,
) {
    let mut active = Vec::new();
    for kind in [SourceKind::Input, SourceKind::Playback] {
        for strip in mixer.strips(kind) {
            let bank = match kind {
                SourceKind::Input => &mut mixer.inputs,
                SourceKind::Playback => &mut mixer.playbacks,
            };
            let value = flag(&mut bank.channels[strip.channel]);
            if *value {
                active.push((kind, strip.channel));
                *value = false;
            }
        }
    }
    if active.is_empty() {
        for (kind, channel) in memory.drain(..) {
            let stereo_right = channel % 2 == 1 && mixer.is_stereo(kind, channel);
            if stereo_right {
                continue;
            }
            let bank = match kind {
                SourceKind::Input => &mut mixer.inputs,
                SourceKind::Playback => &mut mixer.playbacks,
            };
            *flag(&mut bank.channels[channel]) = true;
        }
    } else {
        *memory = active;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> Session {
        let mut session = Session::loading();
        session.enter_demo("test".to_owned());
        session
    }

    #[test]
    fn actions_are_rejected_while_loading() {
        let mut session = Session::loading();
        assert!(session.apply(Action::ToggleMuteMaster).is_err());
    }

    #[test]
    fn mute_master_clears_and_restores() {
        let mut session = ready();
        for channel in [0, 4] {
            session
                .apply(Action::SetMuted {
                    kind: SourceKind::Input,
                    channel,
                    value: true,
                })
                .unwrap();
        }
        session.apply(Action::ToggleMuteMaster).unwrap();
        assert!(!session.mixer.any_muted());
        session.apply(Action::ToggleMuteMaster).unwrap();
        assert!(session.mixer.inputs.channels[0].muted);
        assert!(session.mixer.inputs.channels[4].muted);
    }

    #[test]
    fn pending_changes_track_edits() {
        let mut session = Session::loading();
        session.adopt_hardware("guid".to_owned(), Mixer::default().render());
        assert_eq!(session.pending_changes(), Some(0));
        session
            .apply(Action::SetSendGain {
                kind: SourceKind::Input,
                channel: 0,
                db: Some(0.0),
            })
            .unwrap();
        assert_eq!(session.pending_changes(), Some(2));
    }

    #[test]
    fn actions_deserialize_from_ui_json() {
        let action: Action = serde_json::from_str(
            r#"{"type":"setSendGain","kind":"playback","channel":2,"db":null}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::SetSendGain {
                channel: 2,
                db: None,
                ..
            }
        ));
    }
}
