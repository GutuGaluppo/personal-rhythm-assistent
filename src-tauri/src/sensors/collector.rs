use super::active_app::ActiveAppTracker;
use super::event::ActivityEvent;
use super::idle::{IdleTracker, DEFAULT_IDLE_THRESHOLD_SECS};
use super::probe::AppInfo;
use super::system_state::{ActivityState, SensorSnapshot};
use crate::privacy::toggles::PrivacyToggles;
use chrono::{DateTime, Utc};

pub struct Collector {
    apps: ActiveAppTracker,
    idle: IdleTracker,
    frontmost_name: Option<String>,
    idle_detection_on: bool,
}

impl Collector {
    pub fn new(ignored_bundle_ids: Vec<String>, idle_threshold_secs: f64) -> Self {
        Self {
            apps: ActiveAppTracker::new(ignored_bundle_ids),
            idle: IdleTracker::new(idle_threshold_secs),
            frontmost_name: None,
            idle_detection_on: false,
        }
    }

    pub fn with_defaults(ignored_bundle_ids: Vec<String>) -> Self {
        Self::new(ignored_bundle_ids, DEFAULT_IDLE_THRESHOLD_SECS)
    }

    /// One observation round. An observation is `None` when its sensor is off:
    /// the caller must not even have probed the OS in that case.
    pub fn tick(
        &mut self,
        now: DateTime<Utc>,
        toggles: &PrivacyToggles,
        frontmost: Option<AppInfo>,
        idle_secs: Option<f64>,
    ) -> Vec<ActivityEvent> {
        let mut events = Vec::new();

        if toggles.active_application {
            if let Some(app) = &frontmost {
                self.frontmost_name = Some(app.name.clone());
            }
            events.extend(self.apps.observe(now, frontmost));
        } else {
            self.apps.reset();
            self.frontmost_name = None;
        }

        self.idle_detection_on = toggles.idle_detection;
        match (toggles.idle_detection, idle_secs) {
            (true, Some(secs)) => events.extend(self.idle.observe(now, secs)),
            _ => self.idle.reset(),
        }
        events
    }

    pub fn snapshot(&self) -> SensorSnapshot {
        let state = match (self.idle_detection_on, self.idle.is_idle()) {
            (false, _) => ActivityState::Unknown,
            (true, true) => ActivityState::Idle,
            (true, false) => ActivityState::Active,
        };
        SensorSnapshot {
            state,
            frontmost_application: self.frontmost_name.clone(),
        }
    }
}
