//! Retention configuration model (IMPLEMENTATION.md §21).

use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::repositories::{
    activity_events, daily_summaries, interventions, pauses, sessions, settings,
};
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
    /// `None` = keep indefinitely.
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
    pub interventions_deleted: usize,
    pub pauses_deleted: usize,
    pub daily_summaries_deleted: usize,
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
        // Kept as long as sessions, so summaries can still count accepted/declined check-ins.
        interventions_deleted: interventions::delete_before(conn, &sessions_cutoff)?,
        pauses_deleted: pauses::delete_before(conn, &sessions_cutoff)?,
        // Kept indefinitely unless the user chose a limit.
        daily_summaries_deleted: match policy.daily_summaries_days {
            Some(days) => daily_summaries::delete_before(
                conn,
                &(now - Duration::days(days.into()))
                    .format("%Y-%m-%d")
                    .to_string(),
            )?,
            None => 0,
        },
    })
}

/// One maintenance pass: finished days are summarised *first*, then old data is
/// pruned, so a day's summary is never lost to the sessions it came from.
/// Returns what was pruned.
pub fn maintain(
    conn: &Connection,
    now: DateTime<Utc>,
    tz: chrono::FixedOffset,
) -> Result<RetentionReport> {
    crate::reports::daily::finalize_past_days(conn, now, tz)?;
    apply(conn, now)
}

const MAINTENANCE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Keeps the stored retention policy honest while the app stays open for days.
pub fn spawn_maintenance(db: std::sync::Arc<crate::persistence::Database>) {
    std::thread::Builder::new()
        .name("maintenance".into())
        .spawn(move || loop {
            std::thread::sleep(MAINTENANCE_INTERVAL);
            let _ =
                db.with_conn(|c| maintain(c, Utc::now(), crate::reports::daily::local_offset()));
        })
        .expect("spawn maintenance thread");
}
