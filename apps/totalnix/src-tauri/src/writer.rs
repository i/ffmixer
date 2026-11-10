//! Writer thread: while the session is live, sends the model to the unit
//! one safe step per tick (see `Session::sync`) and tells the UI when the
//! sync state changes.

use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::view::ViewState;
use crate::{STATE_EVENT, SharedSession};

const TICK: Duration = Duration::from_millis(10);
/// How often to check that `ffado-dbus-server` is still the same process.
const LINK_CHECK_EVERY: Duration = Duration::from_secs(1);
/// Minimum gap between state events while changes are still flowing.
const EMIT_EVERY: Duration = Duration::from_millis(100);

pub fn spawn(
    app: AppHandle,
    session: SharedSession,
) {
    thread::spawn(move || {
        let mut last_check = Instant::now();
        let mut last_emit = Instant::now();
        let mut emitted = None;
        loop {
            thread::sleep(TICK);
            let snapshot = {
                let mut session = session.lock().expect("session lock");
                if !session.is_live() && emitted.is_none() {
                    continue;
                }
                if last_check.elapsed() >= LINK_CHECK_EVERY {
                    session.check_link();
                    last_check = Instant::now();
                }
                session.sync();
                let key = (session.is_live(), session.pending_changes());
                let settled = key.1 == Some(0) || !key.0;
                let due = settled || last_emit.elapsed() >= EMIT_EVERY;
                (emitted != Some(key) && due).then(|| (key, ViewState::from_session(&session)))
            };
            if let Some((key, state)) = snapshot {
                let _ = app.emit(STATE_EVENT, state);
                last_emit = Instant::now();
                // Once offline and reported, go idle until live again.
                emitted = if key.0 { Some(key) } else { None };
            }
        }
    });
}
