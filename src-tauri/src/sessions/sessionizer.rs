//! The Sessionizer: a pure, deterministic function from neutral events to sessions.
//!
//! Being a pure function of (events, now, live idle) means a session computed
//! live is identical to one recomputed later from the stored events.
//!
//! Rules
//! - A session starts at the first moment the user is active in a known app.
//! - An idle period of `break_gap` or more ends the session at the last input;
//!   a new one starts when the user returns. Shorter idle stays inside the
//!   session and is counted as `idle_minutes`.
//! - App changes that happen *during* idle (a background app taking focus) are
//!   not the user switching context, so they are not counted.
//! - No event at all for `sensor_gap` (app off, crash) ends the session at the
//!   last event: unobserved time is never counted as activity.
//! - `context_switches` counts `application_switch` events inside the session.

use crate::persistence::time;
use crate::sensors::event::ActivityEvent;
use crate::sessions::model::{Category, Session};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct SessionConfig {
    pub break_gap_secs: i64,
    pub sensor_gap_secs: i64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            break_gap_secs: 15 * 60,
            sensor_gap_secs: 15 * 60,
        }
    }
}

pub type Classifier = Box<dyn Fn(&str) -> Category + Send + Sync>;

pub struct Sessionizer {
    config: SessionConfig,
    classify: Classifier,
}

impl Sessionizer {
    /// Every app is `Unknown` until a classifier is supplied (Milestone 4).
    pub fn new(config: SessionConfig) -> Self {
        Self::with_classifier(config, Box::new(|_| Category::Unknown))
    }

    pub fn with_classifier(config: SessionConfig, classify: Classifier) -> Self {
        Self { config, classify }
    }

    /// Builds all sessions visible in `events`. At most the last one is open
    /// (`ended_at == None`). `live_idle_since` is the start of an idle period
    /// still in progress: it is only reported once it ends, but it must not be
    /// counted as active time in the meantime.
    pub fn run(
        &self,
        events: &[ActivityEvent],
        now: DateTime<Utc>,
        live_idle_since: Option<DateTime<Utc>>,
    ) -> Vec<Session> {
        let mut timeline: Vec<(DateTime<Utc>, &ActivityEvent)> = events
            .iter()
            .filter_map(|e| parse(e.timestamp()).map(|t| (t, e)))
            .collect();
        timeline.sort_by_key(|(t, _)| *t); // stable: ties keep insertion order

        let mut run = Run {
            sessionizer: self,
            current_app: None,
            open: None,
            closed: vec![],
        };
        let mut idle_until: Option<DateTime<Utc>> = None;
        let gap = Duration::seconds(self.config.sensor_gap_secs);
        let break_gap = Duration::seconds(self.config.break_gap_secs);

        for (ts, event) in timeline {
            // Nothing observed for too long: the session ended at the last thing we saw.
            if run
                .open
                .as_ref()
                .is_some_and(|b| ts - b.last_activity > gap)
            {
                run.close_at_last_activity();
            }
            let in_idle = idle_until.is_some_and(|until| ts < until);

            match event {
                ActivityEvent::ActiveApplication { bundle_id, .. } => {
                    run.observe_app(ts, bundle_id, in_idle);
                }
                ActivityEvent::ApplicationSwitch { to_bundle_id, .. } => {
                    run.switch_app(ts, to_bundle_id, in_idle);
                }
                ActivityEvent::Idle { seconds, .. } => {
                    let end = ts + Duration::seconds((*seconds).into());
                    run.idle(ts, end, Duration::seconds((*seconds).into()) >= break_gap);
                    idle_until = Some(end);
                }
            }
        }

        run.finish(now, live_idle_since, gap, break_gap)
    }
}

struct Run<'a> {
    sessionizer: &'a Sessionizer,
    current_app: Option<String>,
    open: Option<Builder>,
    closed: Vec<Session>,
}

impl Run<'_> {
    fn start(&mut self, at: DateTime<Utc>) {
        self.open = self.current_app.clone().map(|app| Builder::new(at, app));
    }

    fn close(&mut self, ended_at: DateTime<Utc>) {
        if let Some(mut b) = self.open.take() {
            b.credit(self.current_app.as_deref(), ended_at);
            self.closed.push(b.into_session(
                Some(ended_at),
                ended_at,
                0.0,
                &self.sessionizer.classify,
            ));
        }
    }

    fn close_at_last_activity(&mut self) {
        if let Some(at) = self.open.as_ref().map(|b| b.last_activity) {
            self.close(at);
        }
    }

    /// `active_application`: also serves as the heartbeat for an unchanged app.
    fn observe_app(&mut self, ts: DateTime<Utc>, bundle_id: &str, in_idle: bool) {
        if in_idle {
            // Background focus change while the user is away.
            self.current_app = Some(bundle_id.to_string());
            return;
        }
        match self.open.as_mut() {
            Some(b) => {
                if self.current_app.as_deref() != Some(bundle_id) {
                    b.credit(self.current_app.as_deref(), ts);
                    b.touch_app(bundle_id);
                    self.current_app = Some(bundle_id.to_string());
                }
                b.last_activity = ts;
            }
            None => {
                self.current_app = Some(bundle_id.to_string());
                self.start(ts);
            }
        }
    }

    fn switch_app(&mut self, ts: DateTime<Utc>, to: &str, in_idle: bool) {
        if in_idle {
            self.current_app = Some(to.to_string());
            return;
        }
        if let Some(b) = self.open.as_mut() {
            b.credit(self.current_app.as_deref(), ts);
            b.touch_app(to);
            b.switches += 1;
            b.last_activity = ts;
        }
        self.current_app = Some(to.to_string());
    }

    /// An idle period `[start, end)` that has finished.
    fn idle(&mut self, start: DateTime<Utc>, end: DateTime<Utc>, is_break: bool) {
        match self.open.as_mut() {
            Some(_) if is_break => {
                self.close(start);
                self.start(end);
            }
            Some(b) => {
                b.credit(self.current_app.as_deref(), start);
                b.idle_secs += (end - start).num_milliseconds() as f64 / 1000.0;
                b.app_since = end;
                b.last_activity = end;
            }
            None => self.start(end),
        }
    }

    fn finish(
        mut self,
        now: DateTime<Utc>,
        live_idle_since: Option<DateTime<Utc>>,
        gap: Duration,
        break_gap: Duration,
    ) -> Vec<Session> {
        if let Some(mut b) = self.open.take() {
            let live_idle = live_idle_since.filter(|li| *li >= b.started_at && *li <= now);
            let classify = &self.sessionizer.classify;
            let app = self.current_app.as_deref();

            if let Some(li) = live_idle {
                b.credit(app, li);
                if now - li >= break_gap {
                    self.closed
                        .push(b.into_session(Some(li), li, 0.0, classify));
                } else {
                    let live_secs = (now - li).num_milliseconds() as f64 / 1000.0;
                    self.closed
                        .push(b.into_session(None, now, live_secs, classify));
                }
            } else if now - b.last_activity > gap {
                let at = b.last_activity;
                b.credit(app, at);
                self.closed
                    .push(b.into_session(Some(at), at, 0.0, classify));
            } else {
                b.credit(app, now);
                self.closed.push(b.into_session(None, now, 0.0, classify));
            }
        }
        self.closed
    }
}

struct Builder {
    started_at: DateTime<Utc>,
    /// Instant from which the current app's time has not been credited yet.
    app_since: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    idle_secs: f64,
    switches: u32,
    apps: Vec<String>,
    app_secs: HashMap<String, f64>,
}

impl Builder {
    fn new(at: DateTime<Utc>, app: String) -> Self {
        Self {
            started_at: at,
            app_since: at,
            last_activity: at,
            idle_secs: 0.0,
            switches: 0,
            apps: vec![app],
            app_secs: HashMap::new(),
        }
    }

    fn touch_app(&mut self, app: &str) {
        if !self.apps.iter().any(|a| a == app) {
            self.apps.push(app.to_string());
        }
    }

    /// Attributes the time since `app_since` to `app`, up to `until`.
    fn credit(&mut self, app: Option<&str>, until: DateTime<Utc>) {
        if until <= self.app_since {
            return;
        }
        if let Some(app) = app {
            self.touch_app(app);
            let secs = (until - self.app_since).num_milliseconds() as f64 / 1000.0;
            *self.app_secs.entry(app.to_string()).or_insert(0.0) += secs;
        }
        self.app_since = until;
    }

    fn into_session(
        self,
        ended_at: Option<DateTime<Utc>>,
        measured_until: DateTime<Utc>,
        live_idle_secs: f64,
        classify: &Classifier,
    ) -> Session {
        let total = (measured_until - self.started_at).num_milliseconds() as f64 / 1000.0;
        let idle = self.idle_secs + live_idle_secs;
        let active = (total - idle).max(0.0);

        // Dominant category by time; ties go to the category seen first.
        let mut by_category: Vec<(Category, f64)> = Vec::new();
        for app in &self.apps {
            let category = classify(app);
            let secs = self.app_secs.get(app).copied().unwrap_or(0.0);
            match by_category.iter_mut().find(|(c, _)| *c == category) {
                Some((_, total)) => *total += secs,
                None => by_category.push((category, secs)),
            }
        }
        let category = by_category
            .iter()
            .fold(None::<(Category, f64)>, |best, &(c, s)| match best {
                Some((_, bs)) if bs >= s => best,
                _ => Some((c, s)),
            })
            .map_or(Category::Unknown, |(c, _)| c);

        Session {
            id: format!("session-{}", time::format(self.started_at)),
            started_at: time::format(self.started_at),
            ended_at: ended_at.map(time::format),
            active_minutes: active / 60.0,
            idle_minutes: idle / 60.0,
            context_switches: self.switches,
            category,
            application_ids: self.apps,
            project_id: None,
        }
    }
}

fn parse(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}
