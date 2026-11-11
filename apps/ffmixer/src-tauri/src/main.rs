//! ffmixer: a TotalMix FX-style mixer for the RME Fireface 800.
//!
//! It adopts the mix `ffado-dbus-server` reports and shows live hardware
//! meters. It starts read-only every time; after the user goes live, the
//! writer thread sends the model to the unit's fader matrices (and
//! nothing else) under the rules in `mixer_core::writes`.

mod channel_names;
mod hardware;
mod meters;
mod session;
mod status;
mod view;
mod writer;

use std::sync::{Arc, Mutex};
use std::thread;

use tauri::{AppHandle, Emitter, Manager, State};

use channel_names::{ChannelNames, NameRow};
use ffado_dbus::settings::Setting;
use session::{Action, Session};
use view::ViewState;

pub(crate) const STATE_EVENT: &str = "state";

pub(crate) type SharedSession = Arc<Mutex<Session>>;

#[tauri::command]
fn get_state(session: State<'_, SharedSession>) -> ViewState {
    ViewState::from_session(&session.lock().expect("session lock"))
}

#[tauri::command]
fn apply(
    action: Action,
    session: State<'_, SharedSession>,
) -> Result<ViewState, String> {
    let mut session = session.lock().expect("session lock");
    session.apply(action)?;
    Ok(ViewState::from_session(&session))
}

/// The warning the user must confirm before `setting` is applied, if
/// any. The same rule is enforced again when the setting is applied.
#[tauri::command]
fn setting_warning(
    setting: Setting,
    session: State<'_, SharedSession>,
) -> Option<String> {
    let session = session.lock().expect("session lock");
    session.setting_warning(&setting).map(str::to_owned)
}

/// Starts or stops sending to the unit. Going live re-reads the unit
/// first, which takes about a second.
#[tauri::command(async)]
fn set_live(
    value: bool,
    session: State<'_, SharedSession>,
) -> Result<ViewState, String> {
    if !value {
        let mut session = session.lock().expect("session lock");
        session.go_offline(None);
        return Ok(ViewState::from_session(&session));
    }
    let guid = session.lock().expect("session lock").device.clone();
    let opened = hardware::open_link(&guid)?;
    let mut session = session.lock().expect("session lock");
    session.go_live(Box::new(opened.link), opened.crosspoints)?;
    session.adopt_device_settings(opened.settings);
    Ok(ViewState::from_session(&session))
}

/// Discards edits: replaces the model with a fresh read of the unit.
#[tauri::command(async)]
fn reload_from_unit(session: State<'_, SharedSession>) -> Result<ViewState, String> {
    let guid = {
        let session = session.lock().expect("session lock");
        if session.mode != session::Mode::Hardware {
            return Err("no unit to reload from".to_owned());
        }
        session.device.clone()
    };
    let unit = hardware::read_mix(&guid)?;
    let settings = hardware::read_device_settings(&guid);
    let mut session = session.lock().expect("session lock");
    session.reload(unit);
    session.adopt_device_settings(settings);
    Ok(ViewState::from_session(&session))
}

/// Renames a source strip (`index` = its first channel) or an output
/// pair; `None` or blank restores the default name. Saved at once.
#[tauri::command]
fn rename(
    row: NameRow,
    index: usize,
    name: Option<String>,
    app: AppHandle,
    session: State<'_, SharedSession>,
) -> Result<ViewState, String> {
    let mut session = session.lock().expect("session lock");
    session.names.set(row, index, name.as_deref())?;
    let view = ViewState::from_session(&session);
    if let Err(e) = session.names.save() {
        // The name still shows; say it won't survive a restart.
        let _ = app.emit(STATE_EVENT, view);
        return Err(format!("Renamed, but could not save names: {e}"));
    }
    Ok(view)
}

/// Reads the unit's mix in the background; falls back to demo mode.
fn load_session(
    app: AppHandle,
    session: SharedSession,
) {
    thread::spawn(move || {
        let loaded = hardware::load_first_device();
        let state = {
            let mut session = session.lock().expect("session lock");
            match loaded {
                Ok(device) => {
                    session.adopt_hardware(device.guid, device.crosspoints);
                    session.adopt_device_settings(device.settings);
                }
                Err(reason) => session.enter_demo(reason),
            }
            ViewState::from_session(&session)
        };
        let _ = app.emit(STATE_EVENT, state);
    });
}

/// WebKitGTK's DMA-BUF renderer crashes on Wayland with some drivers
/// (seen here on NVIDIA: "Error 71 (Protocol error) dispatching to
/// Wayland display"). Turn it off unless the user chose otherwise.
fn apply_webkit_workarounds() {
    const DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(DMABUF).is_none() {
        // SAFETY: called first thing in `main`, before any other thread
        // exists, so nothing can read the environment concurrently.
        unsafe { std::env::set_var(DMABUF, "1") };
    }
}

fn main() {
    apply_webkit_workarounds();
    let mut initial = Session::loading();
    initial.names = ChannelNames::load();
    let session: SharedSession = Arc::new(Mutex::new(initial));
    tauri::Builder::default()
        .manage(session.clone())
        .invoke_handler(tauri::generate_handler![
            get_state,
            apply,
            setting_warning,
            set_live,
            reload_from_unit,
            rename
        ])
        .setup(move |app| {
            let handle = app.app_handle().clone();
            load_session(handle.clone(), session.clone());
            writer::spawn(handle.clone(), session.clone());
            status::spawn(handle.clone(), session.clone());
            meters::spawn(handle, session);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run tauri application");
}
