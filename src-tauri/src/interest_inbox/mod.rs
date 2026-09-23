//! Interest Inbox v0.1 (IMPLEMENTATION.md §18): simple notes to self.
//! No tags, no priorities, no scoring. Everything stays on this machine.

use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::repositories::interests::{self, Interest};
use crate::persistence::time;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

pub const MAX_CHARS: usize = 280;

/// Adds an interest. Surrounding whitespace is dropped and inner runs of it are
/// collapsed. Adding something already in the inbox (ignoring case) is not an
/// error and creates no duplicate: the existing entry is returned.
pub fn add(conn: &Connection, text: &str, now: DateTime<Utc>) -> Result<Interest> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Err(PersistenceError::Invalid(
            "an interest cannot be empty".into(),
        ));
    }
    if text.chars().count() > MAX_CHARS {
        return Err(PersistenceError::Invalid(format!(
            "an interest is at most {MAX_CHARS} characters"
        )));
    }
    if let Some(existing) = interests::find_active_by_text(conn, &text)? {
        return Ok(existing);
    }
    let id = interests::insert(conn, &text, &time::format(now))?;
    Ok(interests::get(conn, id)?.expect("just inserted"))
}

pub fn list(conn: &Connection, archived: bool) -> Result<Vec<Interest>> {
    interests::list(conn, archived)
}

/// Archiving is reversible; returns whether the interest exists.
pub fn archive(conn: &Connection, id: i64, now: DateTime<Utc>) -> Result<bool> {
    interests::set_archived(conn, id, Some(&time::format(now)))
}

pub fn restore(conn: &Connection, id: i64) -> Result<bool> {
    interests::set_archived(conn, id, None)
}

/// Permanent.
pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    interests::delete(conn, id)
}

/// A simple, deterministic suggestion source: one interest per local day, the one
/// that has gone longest without being offered, so forgotten ones come back.
/// Stable for the whole day (asking again returns the same one); if it is
/// archived meanwhile, the next candidate takes its place. `day` is "YYYY-MM-DD".
pub fn suggestion_for(conn: &Connection, day: &str) -> Result<Option<Interest>> {
    if let Some(today) = interests::suggested_on(conn, day)? {
        return Ok(Some(today));
    }
    let Some(next) = interests::least_recently_suggested(conn)? else {
        return Ok(None);
    };
    interests::mark_suggested(conn, next.id, day)?;
    Ok(Some(next))
}
