//! Frontmost-application tracking. A switch is derived from two consecutive
//! observations, so `application_switch` lives here rather than in its own sensor.

use super::event::ActivityEvent;
use super::probe::AppInfo;
use crate::persistence::time;
use chrono::{DateTime, Utc};

#[derive(Default)]
pub struct ActiveAppTracker {
    last_bundle_id: Option<String>,
    ignored_bundle_ids: Vec<String>,
}

impl ActiveAppTracker {
    /// `ignored_bundle_ids`: apps that must not count as user activity (this app itself).
    pub fn new(ignored_bundle_ids: Vec<String>) -> Self {
        Self {
            last_bundle_id: None,
            ignored_bundle_ids,
        }
    }

    /// Feed one observation. `None` (no frontmost app, or the sensor is off) emits nothing.
    pub fn observe(&mut self, now: DateTime<Utc>, app: Option<AppInfo>) -> Vec<ActivityEvent> {
        let Some(app) = app else { return vec![] };
        if self.ignored_bundle_ids.contains(&app.bundle_id) {
            return vec![];
        }
        if self.last_bundle_id.as_deref() == Some(app.bundle_id.as_str()) {
            return vec![];
        }

        let timestamp = time::format(now);
        let mut events = Vec::with_capacity(2);
        if let Some(from) = self.last_bundle_id.take() {
            events.push(ActivityEvent::ApplicationSwitch {
                timestamp: timestamp.clone(),
                from_bundle_id: from,
                to_bundle_id: app.bundle_id.clone(),
            });
        }
        events.push(ActivityEvent::ActiveApplication {
            timestamp,
            bundle_id: app.bundle_id.clone(),
            application_name: app.name,
        });
        self.last_bundle_id = Some(app.bundle_id);
        events
    }

    /// Forget the previous app, e.g. after the sensor was switched off, so that
    /// re-enabling does not fabricate a switch from a stale app.
    pub fn reset(&mut self) {
        self.last_bundle_id = None;
    }
}
