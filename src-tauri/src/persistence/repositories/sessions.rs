use crate::persistence::error::{PersistenceError, Result};
use crate::persistence::time;
use crate::sessions::model::{Category, Session};
use rusqlite::{params, Connection, Row};

/// Inserts or replaces a session (the Sessionizer updates the open session in place).
pub fn upsert(conn: &Connection, s: &Session) -> Result<()> {
    let started_at = time::normalize(&s.started_at)?;
    let ended_at = s.ended_at.as_deref().map(time::normalize).transpose()?;
    conn.execute(
        "INSERT INTO sessions (id, started_at, ended_at, active_minutes, idle_minutes,
                               context_switches, category, application_ids, project_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
             started_at = excluded.started_at, ended_at = excluded.ended_at,
             active_minutes = excluded.active_minutes, idle_minutes = excluded.idle_minutes,
             context_switches = excluded.context_switches, category = excluded.category,
             application_ids = excluded.application_ids, project_id = excluded.project_id",
        params![
            s.id,
            started_at,
            ended_at,
            s.active_minutes,
            s.idle_minutes,
            s.context_switches,
            s.category.as_str(),
            serde_json::to_string(&s.application_ids)?,
            s.project_id,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Session>> {
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE id = ?1"))?;
    let mut rows = stmt.query_map([id], from_row)?;
    rows.next().transpose()?.transpose()
}

/// Sessions with `from <= started_at < to`, oldest first.
pub fn list_started_between(conn: &Connection, from: &str, to: &str) -> Result<Vec<Session>> {
    let (from, to) = (time::normalize(from)?, time::normalize(to)?);
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE started_at >= ?1 AND started_at < ?2 ORDER BY started_at"
    ))?;
    let rows = stmt.query_map(params![from, to], from_row)?;
    rows.map(|r| r?).collect()
}

pub fn count(conn: &Connection) -> Result<u64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?)
}

/// Deletes sessions that started strictly before `cutoff`. Returns the number removed.
pub fn delete_started_before(conn: &Connection, cutoff: &str) -> Result<usize> {
    let cutoff = time::normalize(cutoff)?;
    Ok(conn.execute("DELETE FROM sessions WHERE started_at < ?1", [cutoff])?)
}

const SELECT: &str = "SELECT id, started_at, ended_at, active_minutes, idle_minutes,
    context_switches, category, application_ids, project_id FROM sessions";

fn from_row(r: &Row<'_>) -> rusqlite::Result<Result<Session>> {
    let category: String = r.get(6)?;
    let apps: String = r.get(7)?;
    let build = || -> Result<Session> {
        Ok(Session {
            id: r.get(0)?,
            started_at: r.get(1)?,
            ended_at: r.get(2)?,
            active_minutes: r.get(3)?,
            idle_minutes: r.get(4)?,
            context_switches: r.get(5)?,
            category: Category::parse(&category).ok_or_else(|| {
                PersistenceError::Invalid(format!("unknown category {category:?}"))
            })?,
            application_ids: serde_json::from_str(&apps)?,
            project_id: r.get(8)?,
        })
    };
    Ok(build())
}
