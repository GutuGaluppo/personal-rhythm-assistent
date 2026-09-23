//! Daily summary (IMPLEMENTATION.md §19): what the day looked like, in plain facts.
//!
//! No productivity score, no target, no comparison with other days, no verdict.
//! Everything here is derived from persisted sessions and pauses. Once a day is
//! over its summary is stored, so it stays readable after the sessions it came
//! from have been pruned.

use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::repositories::{daily_summaries, pauses, sessions};
use crate::persistence::{time, Database};
use crate::sessions::model::{Category, Session};
use crate::sessions::service::SessionService;
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const REFLECTIVE_QUESTION: &str = "How did today's rhythm feel to you?";
pub const MAX_REFLECTION_CHARS: usize = 500;
const FAR_FUTURE: &str = "9999-12-31T23:59:59Z";
const LONG_AGO: &str = "1970-01-01T00:00:00Z";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryShare {
    pub category: Category,
    pub minutes: f64,
    /// 0.0 to 1.0 of the day's active time.
    pub share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummary {
    /// The local calendar day, "YYYY-MM-DD".
    pub date: String,
    /// True once the day is over and the summary has been stored.
    pub is_final: bool,
    pub active_minutes: f64,
    /// Only categories with time in them, largest first.
    pub category_distribution: Vec<CategoryShare>,
    /// Active minutes of the longest single session (its share inside this day).
    pub longest_session_minutes: f64,
    pub context_switches: u32,
    /// Pauses actually started, from a check-in, the menu bar or My Day.
    pub pauses_taken: u32,
    pub reflective_question: String,
    /// The user's own words, if they chose to write any.
    pub reflection: Option<String>,
}

pub struct Inputs<'a> {
    pub date: NaiveDate,
    pub day_start: DateTime<Utc>,
    pub day_end: DateTime<Utc>,
    pub now: DateTime<Utc>,
    /// Stored sessions that may overlap the day.
    pub sessions: &'a [Session],
    /// The session in progress (today only); supersedes its stored copy.
    pub current: Option<&'a Session>,
    pub pauses_taken: u32,
}

pub fn build(input: Inputs) -> DailySummary {
    let mut all: Vec<&Session> = input
        .sessions
        .iter()
        .filter(|s| input.current.is_none_or(|c| c.id != s.id))
        .collect();
    all.extend(input.current);

    let mut active = 0.0;
    let mut switches = 0;
    let mut longest: f64 = 0.0;
    let mut by_category: Vec<(Category, f64)> = Vec::new();

    for s in all {
        let Ok(start) = time::parse(&s.started_at) else {
            continue;
        };
        let end = s
            .ended_at
            .as_deref()
            .and_then(|e| time::parse(e).ok())
            .unwrap_or(input.now);
        // The part of the session that falls inside this day.
        let inside = end.min(input.day_end) - start.max(input.day_start);
        let total = end - start;
        if inside <= Duration::zero() || total <= Duration::zero() {
            continue;
        }
        let minutes =
            s.active_minutes * inside.num_milliseconds() as f64 / total.num_milliseconds() as f64;

        active += minutes;
        longest = longest.max(minutes);
        // Switches are whole events: they belong to the day the session began.
        if start >= input.day_start {
            switches += s.context_switches;
        }
        match by_category.iter_mut().find(|(c, _)| *c == s.category) {
            Some((_, m)) => *m += minutes,
            None => by_category.push((s.category, minutes)),
        }
    }

    by_category.retain(|(_, m)| *m > 0.0);
    by_category.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.as_str().cmp(b.0.as_str()))
    });

    DailySummary {
        date: input.date.format("%Y-%m-%d").to_string(),
        is_final: false,
        active_minutes: active,
        category_distribution: by_category
            .into_iter()
            .map(|(category, minutes)| CategoryShare {
                category,
                minutes,
                share: if active > 0.0 { minutes / active } else { 0.0 },
            })
            .collect(),
        longest_session_minutes: longest,
        context_switches: switches,
        pauses_taken: input.pauses_taken,
        reflective_question: REFLECTIVE_QUESTION.to_string(),
        reflection: None,
    }
}

// ---- days -------------------------------------------------------------------------

pub fn today(now: DateTime<Utc>, tz: FixedOffset) -> NaiveDate {
    now.with_timezone(&tz).date_naive()
}

/// `[start, end)` of a local calendar day, as UTC instants.
pub fn day_bounds(date: NaiveDate, tz: FixedOffset) -> (DateTime<Utc>, DateTime<Utc>) {
    let start = tz
        .from_local_datetime(&date.and_time(NaiveTime::MIN))
        .single()
        .expect("a fixed offset has no ambiguous local times")
        .with_timezone(&Utc);
    (start, start + Duration::hours(24))
}

pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| PersistenceError::Invalid(format!("{s:?} is not a date like 2026-03-04")))
}

/// The local offset right now (fine for a day-sized question; there is no DST inside a day's arithmetic here).
pub fn local_offset() -> FixedOffset {
    *chrono::Local::now().offset()
}

// ---- from the database -------------------------------------------------------------

fn compute_from_storage(
    conn: &Connection,
    date: NaiveDate,
    tz: FixedOffset,
    now: DateTime<Utc>,
    current: Option<&Session>,
) -> Result<DailySummary> {
    let (start, end) = day_bounds(date, tz);
    let (from, to) = (time::format(start), time::format(end));
    let stored = sessions::list_overlapping(conn, &from, &to)?;
    let pauses_taken = pauses::count_started_between(conn, &from, &to)?;
    Ok(build(Inputs {
        date,
        day_start: start,
        day_end: end,
        now,
        sessions: &stored,
        current,
        pauses_taken,
    }))
}

/// The summary for `date`. Today's is live; a past day comes from its stored
/// snapshot, or is computed and stored now if that has not happened yet.
pub fn summary_for(
    db: &Database,
    session_service: &SessionService,
    now: DateTime<Utc>,
    tz: FixedOffset,
    date: NaiveDate,
) -> Result<DailySummary> {
    let today = today(now, tz);
    let key = date.format("%Y-%m-%d").to_string();
    let stored = db.with_conn(|c| daily_summaries::get(c, &key))?;
    let reflection = stored.as_ref().and_then(|s| s.reflection.clone());

    if date > today {
        return Err(PersistenceError::Invalid(
            "that day has not happened yet".into(),
        ));
    }

    let mut summary = if date == today {
        let current = session_service.refresh(now)?;
        db.with_conn(|c| compute_from_storage(c, date, tz, now, current.as_ref()))?
    } else if let Some(payload) = stored.as_ref().and_then(|s| s.payload.as_deref()) {
        let mut done: DailySummary = serde_json::from_str(payload)?;
        done.is_final = true;
        done
    } else {
        db.with_conn(|c| finalize_day(c, date, tz, now))?
    };
    summary.reflection = reflection;
    Ok(summary)
}

/// Computes a finished day and stores it. Days with nothing in them are not stored.
fn finalize_day(
    conn: &Connection,
    date: NaiveDate,
    tz: FixedOffset,
    now: DateTime<Utc>,
) -> Result<DailySummary> {
    let mut summary = compute_from_storage(conn, date, tz, now, None)?;
    if summary.active_minutes > 0.0 || summary.pauses_taken > 0 {
        summary.is_final = true;
        daily_summaries::set_payload(
            conn,
            &summary.date,
            &serde_json::to_string(&summary)?,
            &time::format(now),
        )?;
    }
    Ok(summary)
}

/// Stores a summary for every finished day that has sessions but no snapshot yet.
/// Cheap; safe to run at startup and repeatedly. Returns the days it stored.
pub fn finalize_past_days(
    conn: &Connection,
    now: DateTime<Utc>,
    tz: FixedOffset,
) -> Result<Vec<String>> {
    let today = today(now, tz);
    let all = sessions::list_overlapping(conn, LONG_AGO, FAR_FUTURE)?;

    let mut days = BTreeSet::new();
    for s in &all {
        let Ok(start) = time::parse(&s.started_at) else {
            continue;
        };
        let end = s
            .ended_at
            .as_deref()
            .and_then(|e| time::parse(e).ok())
            .unwrap_or(start);
        let (first, last) = (
            start.with_timezone(&tz).date_naive(),
            end.with_timezone(&tz).date_naive(),
        );
        let mut d = first;
        while d <= last {
            days.insert(d);
            d = d.succ_opt().expect("date in range");
        }
    }

    let mut stored = Vec::new();
    for date in days.into_iter().filter(|d| *d < today) {
        let key = date.format("%Y-%m-%d").to_string();
        if daily_summaries::get(conn, &key)?.is_some_and(|s| s.payload.is_some()) {
            continue;
        }
        if finalize_day(conn, date, tz, now)?.is_final {
            stored.push(key);
        }
    }
    Ok(stored)
}

/// Days worth showing, newest first: today, every day with a stored summary, and
/// every day with sessions still on disk.
pub fn list_days(conn: &Connection, now: DateTime<Utc>, tz: FixedOffset) -> Result<Vec<String>> {
    let mut days: BTreeSet<String> = daily_summaries::finalized_dates(conn)?
        .into_iter()
        .collect();
    days.insert(today(now, tz).format("%Y-%m-%d").to_string());
    for s in sessions::list_overlapping(conn, LONG_AGO, FAR_FUTURE)? {
        if let Ok(start) = time::parse(&s.started_at) {
            days.insert(start.with_timezone(&tz).format("%Y-%m-%d").to_string());
        }
    }
    Ok(days.into_iter().rev().collect())
}

/// Saves (or, when empty, clears) the user's reflection for a day that has begun.
pub fn save_reflection(
    conn: &Connection,
    date: NaiveDate,
    text: &str,
    now: DateTime<Utc>,
    tz: FixedOffset,
) -> Result<()> {
    if date > today(now, tz) {
        return Err(PersistenceError::Invalid(
            "that day has not happened yet".into(),
        ));
    }
    let text = text.trim();
    if text.chars().count() > MAX_REFLECTION_CHARS {
        return Err(PersistenceError::Invalid(format!(
            "a reflection is at most {MAX_REFLECTION_CHARS} characters"
        )));
    }
    let key = date.format("%Y-%m-%d").to_string();
    daily_summaries::set_reflection(conn, &key, (!text.is_empty()).then_some(text))
}
