//! "My Day" (IMPLEMENTATION.md §17): plain facts about today so far.
//! No score, no target, no comparison. Interventions are added in Milestone 7.

use crate::persistence::error::Result;
use crate::persistence::repositories::{activity_events, sessions};
use crate::persistence::{time, Database};
use crate::privacy::toggles::{self, PrivacyToggles};
use crate::sensors::service::SharedSnapshot;
use crate::sensors::system_state::ActivityState;
use crate::sessions::model::{Category, Session};
use crate::sessions::service::SessionService;
use chrono::{DateTime, Duration, Local, NaiveTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSession {
    pub started_at: String,
    /// Wall-clock time since the session started (active + idle).
    pub elapsed_minutes: f64,
    pub active_minutes: f64,
    pub category: Category,
    pub context_switches: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastBreak {
    /// Length of the break so far (if in progress) or in total.
    pub minutes: f64,
    /// `None` while the break is still going on.
    pub ended_minutes_ago: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyDay {
    /// False while the "Active time" toggle is off: nothing is being tracked.
    pub tracking_enabled: bool,
    pub state: ActivityState,
    pub frontmost_application: Option<String>,
    pub current_session: Option<CurrentSession>,
    pub active_minutes_today: f64,
    pub context_switches_today: u32,
    pub last_break: Option<LastBreak>,
}

pub struct Inputs<'a> {
    pub now: DateTime<Utc>,
    /// Start of the user's local day, in UTC.
    pub day_start: DateTime<Utc>,
    /// Stored sessions that may overlap today.
    pub sessions: &'a [Session],
    /// The session in progress, freshly computed (supersedes its stored copy).
    pub current: Option<&'a Session>,
    /// Latest finished idle period counted as a real break: (start, length in seconds).
    pub last_break: Option<(DateTime<Utc>, u32)>,
    /// Idle in progress since this instant.
    pub live_idle_since: Option<DateTime<Utc>>,
    pub break_gap_secs: u32,
    pub state: ActivityState,
    pub frontmost_application: Option<String>,
}

pub fn build(input: Inputs) -> MyDay {
    let now = input.now;

    // Stored copy of the open session is replaced by the live one.
    let mut all: Vec<&Session> = input
        .sessions
        .iter()
        .filter(|s| input.current.is_none_or(|c| c.id != s.id))
        .collect();
    all.extend(input.current);

    let mut active_minutes_today = 0.0;
    let mut context_switches_today = 0;
    for s in all {
        let Ok(start) = time::parse(&s.started_at) else {
            continue;
        };
        let end = s
            .ended_at
            .as_deref()
            .and_then(|e| time::parse(e).ok())
            .unwrap_or(now);
        if end <= input.day_start {
            continue;
        }
        if start >= input.day_start {
            active_minutes_today += s.active_minutes;
            context_switches_today += s.context_switches;
        } else {
            // Started before midnight: count the share that falls inside today.
            let total = (end - start).num_milliseconds() as f64;
            let inside = (end - input.day_start).num_milliseconds() as f64;
            if total > 0.0 {
                active_minutes_today += s.active_minutes * inside / total;
            }
        }
    }

    let break_gap = Duration::seconds(input.break_gap_secs.into());
    let last_break = match input
        .live_idle_since
        .filter(|since| now - *since >= break_gap)
    {
        Some(since) => Some(LastBreak {
            minutes: minutes_between(since, now),
            ended_minutes_ago: None,
        }),
        None => input.last_break.map(|(start, secs)| {
            let end = start + Duration::seconds(secs.into());
            LastBreak {
                minutes: f64::from(secs) / 60.0,
                ended_minutes_ago: Some(minutes_between(end, now).max(0.0)),
            }
        }),
    };

    MyDay {
        tracking_enabled: true,
        state: input.state,
        frontmost_application: input.frontmost_application,
        current_session: input.current.and_then(|s| {
            let start = time::parse(&s.started_at).ok()?;
            Some(CurrentSession {
                started_at: s.started_at.clone(),
                elapsed_minutes: minutes_between(start, now),
                active_minutes: s.active_minutes,
                category: s.category,
                context_switches: s.context_switches,
            })
        }),
        active_minutes_today,
        context_switches_today,
        last_break,
    }
}

fn minutes_between(from: DateTime<Utc>, to: DateTime<Utc>) -> f64 {
    (to - from).num_milliseconds() as f64 / 60_000.0
}

/// Midnight of the user's local day containing `now`, as a UTC instant.
pub fn local_day_start(now: DateTime<Utc>) -> DateTime<Utc> {
    now.with_timezone(&Local)
        .date_naive()
        .and_time(NaiveTime::MIN)
        .and_local_timezone(Local)
        .earliest()
        .map_or(now - Duration::hours(24), |t| t.with_timezone(&Utc))
}

/// Gathers everything My Day needs from the local database and the live sensor state.
pub fn load(
    db: &Database,
    session_service: &SessionService,
    snapshot: &SharedSnapshot,
    now: DateTime<Utc>,
    day_start: DateTime<Utc>,
    break_gap_secs: u32,
) -> Result<MyDay> {
    let tracking = db
        .with_conn(toggles::load)
        .unwrap_or_else(|_| PrivacyToggles::all_off());
    let snap = snapshot.lock().unwrap().clone();
    if !tracking.active_time {
        return Ok(MyDay {
            tracking_enabled: false,
            state: snap.state,
            frontmost_application: None,
            current_session: None,
            active_minutes_today: 0.0,
            context_switches_today: 0,
            last_break: None,
        });
    }

    let current = session_service.refresh(now)?;
    let (stored, last_break) = db.with_conn(|conn| {
        // A session that began up to a day before midnight can still reach into today.
        let from = time::format(day_start - Duration::hours(24));
        let stored = sessions::list_started_between(conn, &from, "9999-12-31T23:59:59Z")?;
        let last_break = activity_events::latest_idle_at_least(conn, break_gap_secs)?;
        Ok((stored, last_break))
    })?;

    Ok(build(Inputs {
        now,
        day_start,
        sessions: &stored,
        current: current.as_ref(),
        last_break: last_break
            .and_then(|(start, secs)| time::parse(&start).ok().map(|s| (s, secs))),
        live_idle_since: snap.idle_since.as_deref().and_then(|s| time::parse(s).ok()),
        break_gap_secs,
        state: snap.state,
        frontmost_application: snap.frontmost_application,
    }))
}
