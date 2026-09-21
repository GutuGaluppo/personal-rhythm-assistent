use crate::persistence::error::Result;
use crate::persistence::time;
use rusqlite::{params, Connection};

pub fn insert(
    conn: &Connection,
    id: &str,
    started_at: &str,
    kind: &str,
    planned_seconds: u32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO pauses (id, started_at, kind, planned_seconds) VALUES (?1, ?2, ?3, ?4)",
        params![id, time::normalize(started_at)?, kind, planned_seconds],
    )?;
    Ok(())
}

pub fn set_ended(conn: &Connection, id: &str, ended_at: &str) -> Result<()> {
    conn.execute(
        "UPDATE pauses SET ended_at = ?2 WHERE id = ?1 AND ended_at IS NULL",
        params![id, time::normalize(ended_at)?],
    )?;
    Ok(())
}

/// Pauses started in `[from, to)`.
pub fn count_started_between(conn: &Connection, from: &str, to: &str) -> Result<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM pauses WHERE started_at >= ?1 AND started_at < ?2",
        params![time::normalize(from)?, time::normalize(to)?],
        |r| r.get(0),
    )?)
}

/// Deletes pauses started strictly before `cutoff`.
pub fn delete_before(conn: &Connection, cutoff: &str) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM pauses WHERE started_at < ?1",
        [time::normalize(cutoff)?],
    )?)
}
