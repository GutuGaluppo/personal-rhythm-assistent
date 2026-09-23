use crate::persistence::error::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyPlanItem {
    pub id: i64,
    pub day: String,
    pub text: String,
    pub created_at: String,
    pub done_at: Option<String>,
}

const COLUMNS: &str = "id, day, text, created_at, done_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<DailyPlanItem> {
    Ok(DailyPlanItem {
        id: r.get(0)?,
        day: r.get(1)?,
        text: r.get(2)?,
        created_at: r.get(3)?,
        done_at: r.get(4)?,
    })
}

pub fn insert(conn: &Connection, day: &str, text: &str, created_at: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO daily_plan_items (day, text, created_at) VALUES (?1, ?2, ?3)",
        params![day, text, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<DailyPlanItem>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM daily_plan_items WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()?)
}

/// Oldest first: no reordering in v1, so insertion order is the natural order.
pub fn list_for_day(conn: &Connection, day: &str) -> Result<Vec<DailyPlanItem>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM daily_plan_items WHERE day = ?1 ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map([day], from_row)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn set_done(conn: &Connection, id: i64, done_at: Option<&str>) -> Result<bool> {
    Ok(conn.execute(
        "UPDATE daily_plan_items SET done_at = ?2 WHERE id = ?1",
        params![id, done_at],
    )? > 0)
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM daily_plan_items WHERE id = ?1", [id])? > 0)
}
