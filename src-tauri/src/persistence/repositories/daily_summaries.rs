use crate::persistence::error::Result;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSummary {
    pub date: String,
    pub finalized_at: Option<String>,
    /// The finished summary as JSON; `None` until the day is over.
    pub payload: Option<String>,
    pub reflection: Option<String>,
}

pub fn get(conn: &Connection, date: &str) -> Result<Option<StoredSummary>> {
    Ok(conn
        .query_row(
            "SELECT date, finalized_at, payload, reflection FROM daily_summaries WHERE date = ?1",
            [date],
            |r| {
                Ok(StoredSummary {
                    date: r.get(0)?,
                    finalized_at: r.get(1)?,
                    payload: r.get(2)?,
                    reflection: r.get(3)?,
                })
            },
        )
        .optional()?)
}

/// Stores the finished summary, keeping any reflection already written.
pub fn set_payload(conn: &Connection, date: &str, payload: &str, finalized_at: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO daily_summaries (date, finalized_at, payload) VALUES (?1, ?2, ?3)
         ON CONFLICT(date) DO UPDATE SET finalized_at = excluded.finalized_at, payload = excluded.payload",
        params![date, finalized_at, payload],
    )?;
    Ok(())
}

/// Stores (or, with `None`, clears) the user's reflection, keeping any finished summary.
pub fn set_reflection(conn: &Connection, date: &str, reflection: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT INTO daily_summaries (date, reflection) VALUES (?1, ?2)
         ON CONFLICT(date) DO UPDATE SET reflection = excluded.reflection",
        params![date, reflection],
    )?;
    Ok(())
}

/// Days that have a finished summary, newest first.
pub fn finalized_dates(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT date FROM daily_summaries WHERE payload IS NOT NULL ORDER BY date DESC")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Deletes summaries for days strictly before `cutoff_date` ("YYYY-MM-DD").
pub fn delete_before(conn: &Connection, cutoff_date: &str) -> Result<usize> {
    Ok(conn.execute("DELETE FROM daily_summaries WHERE date < ?1", [cutoff_date])?)
}
