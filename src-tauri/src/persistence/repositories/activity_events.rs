use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::time;
use crate::sensors::event::ActivityEvent;
use rusqlite::{params, Connection, Row};

/// Inserts an event, storing its timestamp in canonical UTC form. Returns the row id.
pub fn insert(conn: &Connection, event: &ActivityEvent) -> Result<i64> {
    let ts = time::normalize(event.timestamp())?;
    match event {
        ActivityEvent::ActiveApplication {
            bundle_id,
            application_name,
            ..
        } => conn.execute(
            "INSERT INTO activity_events (timestamp, type, bundle_id, application_name)
             VALUES (?1, 'active_application', ?2, ?3)",
            params![ts, bundle_id, application_name],
        )?,
        ActivityEvent::Idle { seconds, .. } => conn.execute(
            "INSERT INTO activity_events (timestamp, type, seconds) VALUES (?1, 'idle', ?2)",
            params![ts, seconds],
        )?,
        ActivityEvent::ApplicationSwitch {
            from_bundle_id,
            to_bundle_id,
            ..
        } => conn.execute(
            "INSERT INTO activity_events (timestamp, type, from_bundle_id, to_bundle_id)
             VALUES (?1, 'application_switch', ?2, ?3)",
            params![ts, from_bundle_id, to_bundle_id],
        )?,
    };
    Ok(conn.last_insert_rowid())
}

/// Events with `from <= timestamp < to`, oldest first.
pub fn list_range(conn: &Connection, from: &str, to: &str) -> Result<Vec<ActivityEvent>> {
    let (from, to) = (time::normalize(from)?, time::normalize(to)?);
    let mut stmt = conn.prepare(
        "SELECT timestamp, type, bundle_id, application_name, seconds, from_bundle_id, to_bundle_id
         FROM activity_events WHERE timestamp >= ?1 AND timestamp < ?2
         ORDER BY timestamp, id",
    )?;
    let rows = stmt.query_map(params![from, to], from_row)?;
    rows.map(|r| r?).collect()
}

/// Every application seen in the retained events, with its most recent display name.
pub fn distinct_applications(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT bundle_id, application_name FROM activity_events
         WHERE type = 'active_application' AND id IN (
             SELECT MAX(id) FROM activity_events WHERE type = 'active_application' GROUP BY bundle_id)
         ORDER BY application_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// The most recent idle period of at least `min_seconds`, as (start, length in seconds).
pub fn latest_idle_at_least(conn: &Connection, min_seconds: u32) -> Result<Option<(String, u32)>> {
    Ok(conn
        .query_row(
            "SELECT timestamp, seconds FROM activity_events
             WHERE type = 'idle' AND seconds >= ?1 ORDER BY timestamp DESC LIMIT 1",
            [min_seconds],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?)
}

pub fn count(conn: &Connection) -> Result<u64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM activity_events", [], |r| r.get(0))?)
}

/// Deletes events strictly older than `cutoff`. Returns the number removed.
pub fn delete_before(conn: &Connection, cutoff: &str) -> Result<usize> {
    let cutoff = time::normalize(cutoff)?;
    Ok(conn.execute("DELETE FROM activity_events WHERE timestamp < ?1", [cutoff])?)
}

pub fn delete_all(conn: &Connection) -> Result<usize> {
    Ok(conn.execute("DELETE FROM activity_events", [])?)
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<Result<ActivityEvent>> {
    let timestamp: String = r.get(0)?;
    let kind: String = r.get(1)?;
    let event = match kind.as_str() {
        "active_application" => ActivityEvent::ActiveApplication {
            timestamp,
            bundle_id: r.get(2)?,
            application_name: r.get(3)?,
        },
        "idle" => ActivityEvent::Idle {
            timestamp,
            seconds: r.get(4)?,
        },
        "application_switch" => ActivityEvent::ApplicationSwitch {
            timestamp,
            from_bundle_id: r.get(5)?,
            to_bundle_id: r.get(6)?,
        },
        other => {
            return Ok(Err(PersistenceError::Invalid(format!(
                "unknown event type {other:?}"
            ))))
        }
    };
    Ok(Ok(event))
}
