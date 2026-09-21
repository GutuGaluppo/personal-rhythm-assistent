//! Retention configuration model (IMPLEMENTATION.md §21).

use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::repositories::{activity_events, sessions, settings};
use crate::persistence::time;
use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const SETTINGS_KEY: &str = "retention_policy";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    pub activity_events_days: u32,
    pub sessions_days: u32,
    /// `None` = keep indefinitely. Applied once daily summaries exist (Milestone 10).
    pub daily_summaries_days: Option<u32>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            activity_events_days: 7,
            sessions_days: 30,
            daily_summaries_days: None,
        }
    }
}

impl RetentionPolicy {
    pub fn validate(&self) -> Result<()> {
        let zero = self.activity_events_days == 0
            || self.sessions_days == 0
            || self.daily_summaries_days == Some(0);
        if zero {
            return Err(PersistenceError::Invalid(
                "retention periods must be at least 1 day".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionReport {
    pub activity_events_deleted: usize,
    pub sessions_deleted: usize,
}

pub fn load(conn: &Connection) -> Result<RetentionPolicy> {
    Ok(settings::get_json(conn, SETTINGS_KEY)?.unwrap_or_default())
}

/// Changes the policy for the future. Does not delete anything by itself.
pub fn save(conn: &Connection, policy: &RetentionPolicy) -> Result<()> {
    policy.validate()?;
    settings::set_json(conn, SETTINGS_KEY, policy)
}

/// Deletes data older than the stored policy relative to `now`.
pub fn apply(conn: &Connection, now: DateTime<Utc>) -> Result<RetentionReport> {
    let policy = load(conn)?;
    let events_cutoff = time::format(now - Duration::days(policy.activity_events_days.into()));
    let sessions_cutoff = time::format(now - Duration::days(policy.sessions_days.into()));
    Ok(RetentionReport {
        activity_events_deleted: activity_events::delete_before(conn, &events_cutoff)?,
        sessions_deleted: sessions::delete_started_before(conn, &sessions_cutoff)?,
    })
}
