//! Frontmost-application tracking. A switch is derived from two consecutive
//! observations, so `application_switch` lives here rather than in its own sensor.
//!
//! While the same app stays in front, an `active_application` heartbeat is
//! re-emitted every `HEARTBEAT_SECS`. Working for two hours in one window would
//! otherwise produce no events at all, and the Sessionizer could not tell that
//! apart from the app having been switched off.

use super::event::ActivityEvent;
use super::probe::AppInfo;
use crate::persistence::time;
use chrono::{DateTime, Duration, Utc};

pub const HEARTBEAT_SECS: i64 = 300;

#[derive(Default)]
pub struct ActiveAppTracker {
    last_app: Option<AppInfo>,
    last_emitted: Option<DateTime<Utc>>,
    ignored_bundle_ids: Vec<String>,
}

impl ActiveAppTracker {
    /// `ignored_bundle_ids`: apps that must not count as a context change (this app itself).
    pub fn new(ignored_bundle_ids: Vec<String>) -> Self {
        Self {
            last_app: None,
            last_emitted: None,
            ignored_bundle_ids,
        }
    }

    /// Feed one observation. `None` (no frontmost app, or the sensor is off) emits nothing.
    /// `allow_heartbeat` is false while the user is idle: the user is away, not working.
    pub fn observe(
        &mut self,
        now: DateTime<Utc>,
        app: Option<AppInfo>,
        allow_heartbeat: bool,
    ) -> Vec<ActivityEvent> {
        let Some(app) = app else { return vec![] };

        // Looking at this app's own window is not a context change; the previous
        // app is simply still "current" for continuity.
        if self.ignored_bundle_ids.contains(&app.bundle_id) {
            return self.heartbeat(now, allow_heartbeat).into_iter().collect();
        }

        let same = self
            .last_app
            .as_ref()
            .is_some_and(|last| last.bundle_id == app.bundle_id);
        if same {
            return self.heartbeat(now, allow_heartbeat).into_iter().collect();
        }

        let mut events = Vec::with_capacity(2);
        if let Some(from) = self.last_app.take() {
            events.push(ActivityEvent::ApplicationSwitch {
                timestamp: time::format(now),
                from_bundle_id: from.bundle_id,
                to_bundle_id: app.bundle_id.clone(),
            });
        }
        events.push(active_event(now, &app));
        self.last_app = Some(app);
        self.last_emitted = Some(now);
        events
    }

    fn heartbeat(&mut self, now: DateTime<Utc>, allowed: bool) -> Option<ActivityEvent> {
        let app = self.last_app.as_ref()?;
        let due = self
            .last_emitted
            .is_some_and(|t| now - t >= Duration::seconds(HEARTBEAT_SECS));
        if !(allowed && due) {
            return None;
        }
        self.last_emitted = Some(now);
        Some(active_event(now, app))
    }

    /// Forget the previous app, e.g. after the sensor was switched off, so that
    /// re-enabling does not fabricate a switch from a stale app.
    pub fn reset(&mut self) {
        self.last_app = None;
        self.last_emitted = None;
    }
}

fn active_event(now: DateTime<Utc>, app: &AppInfo) -> ActivityEvent {
    ActivityEvent::ActiveApplication {
        timestamp: time::format(now),
        bundle_id: app.bundle_id.clone(),
        application_name: app.name.clone(),
    }
}
