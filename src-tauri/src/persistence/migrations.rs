//! Forward-only migrations, applied automatically on open.
//! The applied count is tracked in `PRAGMA user_version`.

use super::error::{PersistenceError, Result};
use rusqlite::Connection;

/// Append only. Never edit a shipped migration.
const MIGRATIONS: &[&str] = &[include_str!("../../migrations/0001_initial.sql")];

pub fn latest_version() -> usize {
    MIGRATIONS.len()
}

pub fn run(conn: &mut Connection) -> Result<()> {
    let current: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if current > MIGRATIONS.len() {
        return Err(PersistenceError::SchemaTooNew {
            found: current,
            supported: MIGRATIONS.len(),
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", index + 1)?;
        tx.commit()?;
    }
    Ok(())
}
