//! Idle detection. An `idle` event describes a *finished* idle period:
//! `timestamp` is when the last input happened, `seconds` how long until the
//! next one. Whether the user is idle right now is exposed via `is_idle`.

use super::event::ActivityEvent;
use crate::persistence::time;
use chrono::{DateTime, Duration, Utc};

pub const DEFAULT_IDLE_THRESHOLD_SECS: f64 = 120.0;

/// Idle time dropping by more than this between ticks means input happened.
const RESUME_EPSILON_SECS: f64 = 0.5;

pub struct IdleTracker {
    threshold_secs: f64,
    previous_idle_secs: f64,
    /// Time of the last input before the current idle period, once the threshold is crossed.
    idle_since: Option<DateTime<Utc>>,
}

impl IdleTracker {
    pub fn new(threshold_secs: f64) -> Self {
        Self {
            threshold_secs,
            previous_idle_secs: 0.0,
            idle_since: None,
        }
    }

    pub fn is_idle(&self) -> bool {
        self.idle_since.is_some()
    }

    /// When the current idle period began (time of the last input), once the threshold is crossed.
    pub fn idle_since(&self) -> Option<DateTime<Utc>> {
        self.idle_since
    }

    pub fn observe(&mut self, now: DateTime<Utc>, idle_secs: f64) -> Option<ActivityEvent> {
        let last_input = now - millis(idle_secs);
        let mut event = None;

        match self.idle_since {
            None => {
                if idle_secs >= self.threshold_secs {
                    self.idle_since = Some(last_input);
                }
            }
            Some(started) => {
                if idle_secs + RESUME_EPSILON_SECS < self.previous_idle_secs {
                    // Input happened since the last tick: the idle period ended at `last_input`.
                    let length = (last_input - started).num_milliseconds() as f64 / 1000.0;
                    event = Some(ActivityEvent::Idle {
                        timestamp: time::format(started),
                        seconds: length.max(0.0).round() as u32,
                    });
                    self.idle_since = None;
                }
            }
        }
        self.previous_idle_secs = idle_secs;
        event
    }

    /// Discard any idle period in progress (sensor switched off).
    pub fn reset(&mut self) {
        self.idle_since = None;
        self.previous_idle_secs = 0.0;
    }
}

fn millis(secs: f64) -> Duration {
    Duration::milliseconds((secs * 1000.0) as i64)
}
