//! Brings the pause window forward the moment the user is back at the
//! keyboard, while a pause is running -- so a quick pause always ends with a
//! deliberate "Back to work", not silently, whichever way it started.

use super::pause::PauseService;
use crate::sensors::service::SharedSnapshot;
use crate::sensors::system_state::ActivityState;
use std::sync::Arc;
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub fn spawn(pause: Arc<PauseService>, snapshot: SharedSnapshot) {
    std::thread::Builder::new()
        .name("pause-presence-watcher".into())
        .spawn(move || {
            let mut was_active = false;
            loop {
                std::thread::sleep(POLL_INTERVAL);
                let active_now = snapshot.lock().unwrap().state == ActivityState::Active;
                if active_now && !was_active {
                    // `refocus_if_active` is a no-op unless a pause is actually
                    // running, so this thread otherwise does nothing.
                    pause.refocus_if_active();
                }
                was_active = active_now;
            }
        })
        .expect("spawn pause presence watcher");
}
