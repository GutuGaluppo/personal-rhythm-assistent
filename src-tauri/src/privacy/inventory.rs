//! What the app has stored, so the user can look at it and decide (Milestone 12).
//! Read-only and free of content: counts, date ranges and, for raw activity,
//! exactly the neutral facts the sensors record.

use crate::persistence::error::Result;
use crate::persistence::repositories::activity_events;
use crate::sensors::event::ActivityEvent;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableStats {
    pub count: u64,
    pub oldest: Option<String>,
    pub newest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataOverview {
    /// Raw app and idle events.
    pub activity_events: TableStats,
    pub sessions: TableStats,
    pub check_ins: TableStats,
    pub pauses: TableStats,
    pub daily_summaries: TableStats,
    pub interests: TableStats,
    /// Apps whose category the user changed.
    pub app_category_overrides: u64,
    pub database_bytes: u64,
}

/// `table` and `column` are constants in this file, never user input.
fn stats(conn: &Connection, table: &str, column: &str) -> Result<TableStats> {
    let (count, oldest, newest) = conn.query_row(
        &format!("SELECT COUNT(*), MIN({column}), MAX({column}) FROM {table}"),
        [],
        |r| Ok((r.get::<_, u64>(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(TableStats {
        count,
        oldest,
        newest,
    })
}

pub fn overview(conn: &Connection) -> Result<DataOverview> {
    let pages: u64 = conn.pragma_query_value(None, "page_count", |r| r.get(0))?;
    let page_size: u64 = conn.pragma_query_value(None, "page_size", |r| r.get(0))?;
    Ok(DataOverview {
        activity_events: stats(conn, "activity_events", "timestamp")?,
        sessions: stats(conn, "sessions", "started_at")?,
        check_ins: stats(conn, "interventions", "shown_at")?,
        pauses: stats(conn, "pauses", "started_at")?,
        daily_summaries: stats(conn, "daily_summaries", "date")?,
        interests: stats(conn, "interests", "created_at")?,
        app_category_overrides: conn.query_row(
            "SELECT COUNT(*) FROM app_category_mappings",
            [],
            |r| r.get(0),
        )?,
        database_bytes: pages * page_size,
    })
}

pub const MAX_RECENT: u32 = 200;

/// The newest raw events, newest first, so the user can see for themselves that
/// only app names, switches and idle lengths are kept. At most `MAX_RECENT`.
pub fn recent_events(conn: &Connection, limit: u32) -> Result<Vec<ActivityEvent>> {
    activity_events::list_recent(conn, limit.clamp(1, MAX_RECENT))
}
