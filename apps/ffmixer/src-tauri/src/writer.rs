//! Writer thread: while the session is live, sends the model to the unit
//! one safe step per tick (see `Session::sync`) and tells the UI when the
//! sync state changes.

use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::view::{self, ViewState};
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
        loop {
            thread::sleep(TICK);
            let snapshot = {
                let mut session = session.lock().expect("session lock");
                // Offline and already shown as offline: nothing to report.
                let shown = view::last_shown();
                if !session.is_live() && shown.is_none_or(|(live, _)| !live) {
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
                // Compare with what the UI was last shown from any path, not
                // just what this thread sent: a command reply can carry a
                // count the writer has since cleared.
                (shown != Some(key) && due).then(|| ViewState::from_session(&session))
            };
            if let Some(state) = snapshot {
                let _ = app.emit(STATE_EVENT, state);
                last_emit = Instant::now();
            }
        }
    });
}
