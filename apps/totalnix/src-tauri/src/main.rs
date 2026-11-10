//! totalnix: a TotalMix FX-style mixer for the RME Fireface 800.
//!
//! This build never writes to the unit. It adopts the mix
//! `ffado-dbus-server` reports, shows live hardware meters, and lets the
//! UI edit an in-memory copy of the mix.

mod hardware;
mod meters;
mod session;
mod view;

use std::sync::{Arc, Mutex};
use std::thread;

use tauri::{AppHandle, Emitter, Manager, State};

use session::{Action, Session};
use view::ViewState;

const STATE_EVENT: &str = "state";

type SharedSession = Arc<Mutex<Session>>;

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
                Ok(device) => session.adopt_hardware(device.guid, device.crosspoints),
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
    let session: SharedSession = Arc::new(Mutex::new(Session::loading()));
    tauri::Builder::default()
        .manage(session.clone())
        .invoke_handler(tauri::generate_handler![get_state, apply])
        .setup(move |app| {
            let handle = app.app_handle().clone();
            load_session(handle.clone(), session.clone());
            meters::spawn(handle, session);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run tauri application");
}
