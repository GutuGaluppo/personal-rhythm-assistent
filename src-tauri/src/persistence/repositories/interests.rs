use crate::persistence::error::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Interest {
    pub id: i64,
    pub text: String,
    pub created_at: String,
    pub archived_at: Option<String>,
}

const COLUMNS: &str = "id, text, created_at, archived_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<Interest> {
    Ok(Interest {
        id: r.get(0)?,
        text: r.get(1)?,
        created_at: r.get(2)?,
        archived_at: r.get(3)?,
    })
}

pub fn insert(conn: &Connection, text: &str, created_at: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO interests (text, created_at) VALUES (?1, ?2)",
        params![text, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Interest>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM interests WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()?)
}

/// An active (not archived) interest with the same text, ignoring case.
pub fn find_active_by_text(conn: &Connection, text: &str) -> Result<Option<Interest>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM interests
                 WHERE archived_at IS NULL AND lower(text) = lower(?1) LIMIT 1"
            ),
            [text],
            from_row,
        )
        .optional()?)
}

/// Newest first.
pub fn list(conn: &Connection, archived: bool) -> Result<Vec<Interest>> {
    let filter = if archived { "IS NOT NULL" } else { "IS NULL" };
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM interests WHERE archived_at {filter} ORDER BY created_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn set_archived(conn: &Connection, id: i64, archived_at: Option<&str>) -> Result<bool> {
    Ok(conn.execute(
        "UPDATE interests SET archived_at = ?2 WHERE id = ?1",
        params![id, archived_at],
    )? > 0)
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM interests WHERE id = ?1", [id])? > 0)
}

pub fn count(conn: &Connection) -> Result<u64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM interests", [], |r| r.get(0))?)
}

/// The active interest already offered as `day`'s suggestion, if any.
pub fn suggested_on(conn: &Connection, day: &str) -> Result<Option<Interest>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM interests
                 WHERE archived_at IS NULL AND last_suggested_on = ?1 LIMIT 1"
            ),
            [day],
            from_row,
        )
        .optional()?)
}

/// The active interest that has gone longest without being offered: never-offered
/// first (oldest first), then least recently offered.
pub fn least_recently_suggested(conn: &Connection) -> Result<Option<Interest>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM interests WHERE archived_at IS NULL
                 ORDER BY last_suggested_on IS NOT NULL, last_suggested_on, created_at, id LIMIT 1"
            ),
            [],
            from_row,
        )
        .optional()?)
}

pub fn mark_suggested(conn: &Connection, id: i64, day: &str) -> Result<()> {
    conn.execute(
        "UPDATE interests SET last_suggested_on = ?2 WHERE id = ?1",
        params![id, day],
    )?;
    Ok(())
}
