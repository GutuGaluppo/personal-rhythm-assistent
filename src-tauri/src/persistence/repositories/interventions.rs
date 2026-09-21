use crate::persistence::error::Result;
use crate::persistence::time;
use rusqlite::{params, Connection};

pub struct NewIntervention<'a> {
    pub id: &'a str,
    pub shown_at: &'a str,
    pub is_retry: bool,
    pub active_minutes: f64,
    pub reasons: &'a [String],
}

pub fn insert(conn: &Connection, i: &NewIntervention) -> Result<()> {
    conn.execute(
        "INSERT INTO interventions (id, shown_at, is_retry, active_minutes, reasons)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            i.id,
            time::normalize(i.shown_at)?,
            i.is_retry,
            i.active_minutes,
            serde_json::to_string(i.reasons)?
        ],
    )?;
    Ok(())
}

/// `kind` is "energy" or "action".
pub fn add_feedback(
    conn: &Connection,
    intervention_id: &str,
    at: &str,
    kind: &str,
    value: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO intervention_feedback (intervention_id, at, kind, value)
         VALUES (?1, ?2, ?3, ?4)",
        params![intervention_id, time::normalize(at)?, kind, value],
    )?;
    Ok(())
}

/// (kind, value) pairs for one intervention, in the order given.
pub fn feedback_for(conn: &Connection, intervention_id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT kind, value FROM intervention_feedback WHERE intervention_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([intervention_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn count(conn: &Connection) -> Result<u64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM interventions", [], |r| r.get(0))?)
}

/// Deletes interventions shown strictly before `cutoff` (their feedback goes with them).
pub fn delete_before(conn: &Connection, cutoff: &str) -> Result<usize> {
    let cutoff = time::normalize(cutoff)?;
    Ok(conn.execute("DELETE FROM interventions WHERE shown_at < ?1", [cutoff])?)
}

/// The stored reasons of one intervention.
pub fn reasons_of(conn: &Connection, id: &str) -> Result<Vec<String>> {
    let raw: String = conn.query_row(
        "SELECT reasons FROM interventions WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}
