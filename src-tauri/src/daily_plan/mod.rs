//! Today's plan: a simple free-text checklist scoped to the local calendar day.
//! No times, no reordering, no scoring -- just what the user meant to get done.

use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::repositories::daily_plan::{self, DailyPlanItem};
use crate::persistence::time;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

pub const MAX_CHARS: usize = 280;

/// Surrounding whitespace is dropped and inner runs of it are collapsed.
pub fn add(conn: &Connection, day: &str, text: &str, now: DateTime<Utc>) -> Result<DailyPlanItem> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Err(PersistenceError::Invalid(
            "a plan item cannot be empty".into(),
        ));
    }
    if text.chars().count() > MAX_CHARS {
        return Err(PersistenceError::Invalid(format!(
            "a plan item is at most {MAX_CHARS} characters"
        )));
    }
    let id = daily_plan::insert(conn, day, &text, &time::format(now))?;
    Ok(daily_plan::get(conn, id)?.expect("just inserted"))
}

pub fn list(conn: &Connection, day: &str) -> Result<Vec<DailyPlanItem>> {
    daily_plan::list_for_day(conn, day)
}

/// Flips done/undone; returns the updated row, or `Ok(None)` if `id` doesn't exist.
pub fn toggle(conn: &Connection, id: i64, now: DateTime<Utc>) -> Result<Option<DailyPlanItem>> {
    let Some(item) = daily_plan::get(conn, id)? else {
        return Ok(None);
    };
    let done_at = if item.done_at.is_some() {
        None
    } else {
        Some(time::format(now))
    };
    daily_plan::set_done(conn, id, done_at.as_deref())?;
    daily_plan::get(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    daily_plan::delete(conn, id)
}
