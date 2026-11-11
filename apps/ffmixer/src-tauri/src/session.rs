//! The app's mixer session: the model, where it came from, and the
//! actions the UI may apply to it.
//!
//! Actions only change the in-memory model. While the session is
//! **live** (see [`Session::go_live`]), the writer thread calls
//! [`Session::sync`] every tick, which sends the difference between the
//! model and the unit through a [`UnitLink`], under the safety rules of
//! [`mixer_core::writes`]. Otherwise `pending_changes` reports how many
//! crosspoints would change.
//!
//! Device settings (levels, phantom power, clock) are not part of the
//! model: [`Action::SetDevice`] applies one at once, only while live, and
//! only after confirmation if it is risky (see [`Setting::risk`]).

use ffado_dbus::settings::{DeviceSettings, Setting};
use mixer_core::writes::{self, Write};
use mixer_core::{Crosspoints, Mixer, SourceKind, law};
use serde::Deserialize;

use crate::channel_names::ChannelNames;

const READ_ONLY_STATUS: &str = "Read-only: changes are not sent to the unit";
const LIVE_STATUS: &str = "Live: changes are sent to the unit";

/// Gains sent per tick at most; the rest follow on the next tick.
const WRITES_PER_TICK: usize = 64;

/// Where accepted writes go: `ffado-dbus-server`, or a fake in tests.
pub trait UnitLink: Send {
    fn set_gain(
        &self,
        write: &Write,
    ) -> Result<(), String>;

    /// Sends one device setting, reads the settings back, and returns
    /// them if they show exactly that change.
    fn apply_setting(
        &self,
        setting: &Setting,
        current: &DeviceSettings,
    ) -> Result<DeviceSettings, String>;

    /// False once the server we bound to has been replaced or is gone.
    fn server_unchanged(&self) -> Result<bool, String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Reading the mix from `ffado-dbus-server`.
    Loading,
    /// Mix adopted from the unit; meters are live. Writes only while live.
    Hardware,
    /// No unit available: default mix and simulated meters.
    Demo,
}

pub struct Session {
    pub mode: Mode,
    pub device: String,
    pub status: String,
    pub mixer: Mixer,
    /// What the unit holds: read from it, then updated by every write.
    pub hardware: Option<Crosspoints>,
    pub warnings: Vec<String>,
    /// User channel names (display only).
    pub names: ChannelNames,
    /// The unit's device settings as FFADO holds them; `None` if they
    /// could not be read.
    pub device_settings: Option<DeviceSettings>,
    /// Present while live: every model change is sent to the unit.
    link: Option<Box<dyn UnitLink>>,
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
    /// Phase of any channel, including the right side of a stereo strip.
    SetPhase {
        kind: SourceKind,
        channel: usize,
        value: bool,
    },
    SetWidth {
        kind: SourceKind,
        pair: usize,
        width: f64,
    },
    SetMsProc {
        kind: SourceKind,
        pair: usize,
        value: bool,
    },
    /// Phase of one output channel (0–27).
    SetOutputPhase {
        channel: usize,
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
    /// Changes a device setting on the unit now. Needs `confirmed` if
    /// [`Setting::risk`] says so.
    SetDevice {
        setting: Setting,
        confirmed: bool,
    },
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
            names: ChannelNames::default(),
            device_settings: None,
            link: None,
            mute_memory: Vec::new(),
            solo_memory: Vec::new(),
        }
    }

    pub fn adopt_hardware(
        &mut self,
        guid: String,
        crosspoints: Crosspoints,
    ) {
        self.mode = Mode::Hardware;
        self.status = READ_ONLY_STATUS.to_owned();
        self.device = guid;
        self.reload(crosspoints);
    }

    /// Replaces the model with the unit's current mix, keeping the
    /// selected output and live state. Nothing is left to send.
    pub fn reload(
        &mut self,
        crosspoints: Crosspoints,
    ) {
        let (mut mixer, report) = mixer_core::import(&crosspoints);
        mixer.selected_output = self.mixer.selected_output;
        self.mixer = mixer;
        self.hardware = Some(crosspoints);
        self.warnings = report.warnings;
        self.mute_memory.clear();
        self.solo_memory.clear();
    }

    /// Adopts freshly read device settings, keeping what FFADO cannot
    /// report (Mic 10 phantom) from the previous ones.
    pub fn adopt_device_settings(
        &mut self,
        fresh: Option<DeviceSettings>,
    ) {
        self.device_settings = match (fresh, self.device_settings) {
            (Some(mut fresh), Some(previous)) => {
                fresh.keep_unreadable_from(&previous);
                Some(fresh)
            }
            (fresh, _) => fresh,
        };
    }

    pub fn enter_demo(
        &mut self,
        reason: String,
    ) {
        self.go_offline(None);
        self.mode = Mode::Demo;
        self.status = format!("Demo mode: {reason}");
        self.device = "Demo".to_owned();
        self.mixer = Mixer::default();
        self.hardware = None;
        self.device_settings = None;
        self.warnings.clear();
    }

    pub fn is_live(&self) -> bool {
        self.link.is_some()
    }

    /// Starts sending. `unit` is a fresh read of the unit, so changes
    /// made elsewhere since loading are not mistaken for ours.
    pub fn go_live(
        &mut self,
        link: Box<dyn UnitLink>,
        unit: Crosspoints,
    ) -> Result<(), String> {
        if self.mode != Mode::Hardware {
            return Err("no unit to send to".to_owned());
        }
        self.hardware = Some(unit);
        self.link = Some(link);
        self.status = LIVE_STATUS.to_owned();
        Ok(())
    }

    /// Stops sending. `reason` replaces the status line if given.
    pub fn go_offline(
        &mut self,
        reason: Option<String>,
    ) {
        self.link = None;
        if self.mode == Mode::Hardware {
            self.status = reason.unwrap_or_else(|| READ_ONLY_STATUS.to_owned());
        }
    }

    /// One writer tick: sends the next safe step toward the model.
    /// Any failure drops out of live mode rather than retrying.
    pub fn sync(&mut self) {
        let (Some(link), Some(unit)) = (&self.link, &mut self.hardware) else {
            return;
        };
        let plan = writes::plan(unit, &self.mixer.render(), WRITES_PER_TICK);
        let mut failure = None;
        for write in &plan.writes {
            if let Err(e) = link.set_gain(write) {
                failure = Some(e);
                break;
            }
            writes::apply(unit, write);
        }
        if let Some(e) = failure {
            self.go_offline(Some(format!("Stopped sending: {e}")));
        }
    }

    /// Drops out of live mode if `ffado-dbus-server` was replaced: a new
    /// server reloads the flash mix onto the unit, so our record of the
    /// unit would be wrong.
    pub fn check_link(&mut self) {
        let Some(link) = &self.link else {
            return;
        };
        match link.server_unchanged() {
            Ok(true) => {}
            Ok(false) => self.go_offline(Some(
                "Stopped sending: ffado-dbus-server restarted and reloaded the flash mix. \
                 Reload from unit, or go live to send this mix again."
                    .to_owned(),
            )),
            Err(e) => self.go_offline(Some(format!("Stopped sending: {e}"))),
        }
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
            Action::SetPhase {
                kind,
                channel,
                value,
            } => mixer.set_phase(kind, channel, value),
            Action::SetWidth { kind, pair, width } => mixer.set_width(kind, pair, width),
            Action::SetMsProc { kind, pair, value } => mixer.set_ms_proc(kind, pair, value),
            Action::SetOutputPhase { channel, value } => mixer.set_output_phase(channel, value),
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
            Action::SetDevice { setting, confirmed } => {
                return self.change_device_setting(setting, confirmed);
            }
        };
        result.map_err(|e| e.to_string())
    }

    /// The warning to show before `setting` may be applied, if any.
    pub fn setting_warning(
        &self,
        setting: &Setting,
    ) -> Option<&'static str> {
        let current = self.device_settings.as_ref()?;
        setting.risk(current).map(|risk| risk.warning())
    }

    /// Sends one device setting. Any failure drops out of live mode: the
    /// unit may not be in the state we think it is.
    fn change_device_setting(
        &mut self,
        setting: Setting,
        confirmed: bool,
    ) -> Result<(), String> {
        let (Some(link), Some(current)) = (&self.link, &self.device_settings) else {
            return Err("Go live to change device settings".to_owned());
        };
        if let Some(risk) = setting.risk(current)
            && !confirmed
        {
            return Err(format!("needs confirmation: {}", risk.warning()));
        }
        match link.apply_setting(&setting, current) {
            Ok(applied) => {
                self.device_settings = Some(applied);
                Ok(())
            }
            Err(e) => {
                self.go_offline(Some(format!("Stopped sending: {e}")));
                Err(e)
            }
        }
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

    #[test]
    fn settings_actions_deserialize_from_ui_json() {
        for json in [
            r#"{"type":"setPhase","kind":"input","channel":1,"value":true}"#,
            r#"{"type":"setWidth","kind":"playback","pair":0,"width":0.5}"#,
            r#"{"type":"setMsProc","kind":"input","pair":3,"value":true}"#,
            r#"{"type":"setOutputPhase","channel":5,"value":true}"#,
        ] {
            let action: Result<Action, _> = serde_json::from_str(json);
            assert!(action.is_ok(), "{json}: {action:?}");
        }
    }

    use ffado_dbus::settings::{Mic, OutputLevel};
    use std::sync::{Arc, Mutex};

    /// Records writes; fails once `fail_after` writes have succeeded.
    struct FakeSink {
        sent: Arc<Mutex<Vec<Write>>>,
        settings_sent: Arc<Mutex<Vec<Setting>>>,
        fail_after: usize,
        fail_settings: bool,
        server_unchanged: bool,
    }

    impl UnitLink for FakeSink {
        fn apply_setting(
            &self,
            setting: &Setting,
            current: &DeviceSettings,
        ) -> Result<DeviceSettings, String> {
            if self.fail_settings {
                return Err("bus error".to_owned());
            }
            self.settings_sent.lock().unwrap().push(*setting);
            let mut after = *current;
            setting.apply_to(&mut after);
            Ok(after)
        }

        fn set_gain(
            &self,
            write: &Write,
        ) -> Result<(), String> {
            let mut sent = self.sent.lock().unwrap();
            if sent.len() >= self.fail_after {
                return Err("bus error".to_owned());
            }
            sent.push(*write);
            Ok(())
        }

        fn server_unchanged(&self) -> Result<bool, String> {
            Ok(self.server_unchanged)
        }
    }

    fn live(
        fail_after: usize,
        server_unchanged: bool,
    ) -> (Session, Arc<Mutex<Vec<Write>>>) {
        let unit = Mixer::default().render();
        let mut session = Session::loading();
        session.adopt_hardware("guid".to_owned(), unit.clone());
        let sent = Arc::new(Mutex::new(Vec::new()));
        let sink = FakeSink {
            sent: sent.clone(),
            settings_sent: Arc::default(),
            fail_after,
            fail_settings: false,
            server_unchanged,
        };
        session.go_live(Box::new(sink), unit).unwrap();
        (session, sent)
    }

    /// A live session that also holds device settings (all raw zeros:
    /// front inputs, -10 dBV out, internal clock, everything off).
    fn live_with_device_settings(fail_settings: bool) -> (Session, Arc<Mutex<Vec<Setting>>>) {
        let unit = Mixer::default().render();
        let mut session = Session::loading();
        session.adopt_hardware("guid".to_owned(), unit.clone());
        session.device_settings = Some(DeviceSettings::read_with(|_| Ok(0)).unwrap());
        let settings_sent = Arc::new(Mutex::new(Vec::new()));
        let sink = FakeSink {
            sent: Arc::default(),
            settings_sent: settings_sent.clone(),
            fail_after: usize::MAX,
            fail_settings,
            server_unchanged: true,
        };
        session.go_live(Box::new(sink), unit).unwrap();
        (session, settings_sent)
    }

    fn device(setting: Setting) -> Action {
        Action::SetDevice {
            setting,
            confirmed: false,
        }
    }

    fn confirmed(setting: Setting) -> Action {
        Action::SetDevice {
            setting,
            confirmed: true,
        }
    }

    #[test]
    fn device_settings_need_a_live_session() {
        let mut session = Session::loading();
        session.adopt_hardware("guid".to_owned(), Mixer::default().render());
        session.device_settings = Some(DeviceSettings::read_with(|_| Ok(0)).unwrap());
        let error = session
            .apply(device(Setting::Emphasis { on: true }))
            .unwrap_err();
        assert!(error.contains("Go live"), "{error}");
    }

    #[test]
    fn safe_device_setting_is_applied_at_once() {
        let (mut session, sent) = live_with_device_settings(false);
        session
            .apply(device(Setting::Emphasis { on: true }))
            .unwrap();
        assert_eq!(*sent.lock().unwrap(), [Setting::Emphasis { on: true }]);
        assert!(session.device_settings.unwrap().emphasis);
        assert!(session.is_live());
    }

    #[test]
    fn risky_device_setting_needs_confirmation() {
        let (mut session, sent) = live_with_device_settings(false);
        let setting = Setting::OutputLevel {
            level: OutputLevel::HiGain,
        };
        assert!(session.setting_warning(&setting).is_some());
        let error = session.apply(device(setting)).unwrap_err();
        assert!(error.starts_with("needs confirmation"), "{error}");
        assert!(sent.lock().unwrap().is_empty());

        session.apply(confirmed(setting)).unwrap();
        assert_eq!(*sent.lock().unwrap(), [setting]);
        assert_eq!(
            session.device_settings.unwrap().output_level,
            OutputLevel::HiGain
        );
    }

    #[test]
    fn unchanged_risky_setting_needs_no_confirmation() {
        let (mut session, _) = live_with_device_settings(false);
        let same = Setting::OutputLevel {
            level: OutputLevel::Minus10dBV,
        };
        assert!(session.setting_warning(&same).is_none());
        session.apply(device(same)).unwrap();
    }

    #[test]
    fn failed_device_write_stops_sending() {
        let (mut session, _) = live_with_device_settings(true);
        let error = session
            .apply(device(Setting::Emphasis { on: true }))
            .unwrap_err();
        assert_eq!(error, "bus error");
        assert!(!session.is_live());
        assert!(session.status.starts_with("Stopped sending"));
        assert!(!session.device_settings.unwrap().emphasis);
    }

    #[test]
    fn device_action_deserializes_from_ui_json() {
        let action: Action = serde_json::from_str(
            r#"{"type":"setDevice","setting":{"control":"phantom","mic":"mic7","on":true},"confirmed":false}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::SetDevice {
                setting: Setting::Phantom {
                    mic: Mic::Mic7,
                    on: true
                },
                confirmed: false
            }
        ));
    }

    #[test]
    fn live_session_converges_on_the_model() {
        let (mut session, sent) = live(usize::MAX, true);
        session
            .apply(Action::SetSendGain {
                kind: SourceKind::Input,
                channel: 0,
                db: Some(0.0),
            })
            .unwrap();
        assert_eq!(session.pending_changes(), Some(2));
        for _ in 0..40 {
            session.sync();
        }
        assert_eq!(session.pending_changes(), Some(0));
        assert!(session.is_live());
        // Ramped, not one jump: many writes per side.
        assert!(sent.lock().unwrap().len() > 10);
    }

    #[test]
    fn failed_write_drops_out_of_live_mode() {
        let (mut session, _) = live(0, true);
        session
            .apply(Action::SetOutputMuted {
                output: 0,
                value: true,
            })
            .unwrap();
        session.sync();
        assert!(!session.is_live());
        assert!(session.status.contains("bus error"), "{}", session.status);
    }

    #[test]
    fn server_restart_drops_out_of_live_mode() {
        let (mut session, _) = live(usize::MAX, false);
        session.check_link();
        assert!(!session.is_live());
        assert!(session.status.contains("restarted"));
    }

    #[test]
    fn phase_changes_are_sent() {
        let (mut session, sent) = live(usize::MAX, true);
        session
            .apply(Action::SetPhase {
                kind: SourceKind::Playback,
                channel: 0,
                value: true,
            })
            .unwrap();
        session.sync();
        assert_eq!(session.pending_changes(), Some(0));
        assert!(sent.lock().unwrap().iter().all(|write| write.value < 0.0));
    }

    #[test]
    fn demo_cannot_go_live() {
        let mut session = ready();
        let sink = FakeSink {
            sent: Arc::default(),
            settings_sent: Arc::default(),
            fail_after: 0,
            fail_settings: false,
            server_unchanged: true,
        };
        assert!(
            session
                .go_live(Box::new(sink), Mixer::default().render())
                .is_err()
        );
    }
}
