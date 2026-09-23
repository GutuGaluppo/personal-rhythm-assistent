use crate::interventions::pause::PauseReason;
use crate::persistence::error::Result;
use crate::persistence::time;
use rusqlite::{params, Connection};

/// `planned_seconds` is `None` for a quick, untimed pause. `reason` is the
/// user's optional justification (chip kind + note, only for "other").
pub fn insert(
    conn: &Connection,
    id: &str,
    started_at: &str,
    kind: &str,
    planned_seconds: Option<u32>,
    reason: Option<&PauseReason>,
) -> Result<()> {
    let (reason_kind, reason_note) = match reason {
        Some(r) => (Some(r.kind.as_str()), r.note.as_deref()),
        None => (None, None),
    };
    conn.execute(
        "INSERT INTO pauses (id, started_at, kind, planned_seconds, reason, reason_note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            time::normalize(started_at)?,
            kind,
            planned_seconds,
            reason_kind,
            reason_note
        ],
    )?;
    Ok(())
}

/// Replaces the reason on a still-running pause. `None` clears it.
pub fn set_reason(conn: &Connection, id: &str, reason: Option<&PauseReason>) -> Result<bool> {
    let (reason_kind, reason_note) = match reason {
        Some(r) => (Some(r.kind.as_str()), r.note.as_deref()),
        None => (None, None),
    };
    let changed = conn.execute(
        "UPDATE pauses SET reason = ?2, reason_note = ?3 WHERE id = ?1 AND ended_at IS NULL",
        params![id, reason_kind, reason_note],
    )?;
    Ok(changed > 0)
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
