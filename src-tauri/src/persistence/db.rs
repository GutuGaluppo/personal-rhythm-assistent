use super::error::{PersistenceError, Result};
use super::migrations;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Shared handle to the local database. Migrations run on open.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        // Deleted content is overwritten, not just unlinked (privacy).
        conn.pragma_update(None, "secure_delete", true)?;
        migrations::run(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| PersistenceError::Invalid("database lock poisoned".into()))?;
        f(&guard)
    }

    /// Removes every row from every table (settings included) and compacts the
    /// file. Schema and migration state are kept. Discovers tables dynamically so
    /// tables added by later milestones are covered automatically.
    pub fn delete_all_local_data(&self) -> Result<()> {
        self.with_conn(|conn| {
            let tables: Vec<String> = conn
                .prepare(
                    "SELECT name FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                )?
                .query_map([], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;

            let tx = conn.unchecked_transaction()?;
            for table in &tables {
                tx.execute(
                    &format!("DELETE FROM \"{}\"", table.replace('"', "\"\"")),
                    [],
                )?;
            }
            // Restart AUTOINCREMENT counters (table exists once any AUTOINCREMENT table does).
            tx.execute("DELETE FROM sqlite_sequence", [])?;
            tx.commit()?;
            conn.execute_batch("VACUUM")?;
            Ok(())
        })
    }

    /// Removes the raw activity events only (app and idle facts). Sessions, summaries,
    /// check-ins and everything else stay. The file is compacted so nothing lingers.
    pub fn delete_raw_data(&self) -> Result<usize> {
        self.with_conn(|conn| {
            let removed = conn.execute("DELETE FROM activity_events", [])?;
            conn.execute(
                "DELETE FROM sqlite_sequence WHERE name = 'activity_events'",
                [],
            )?;
            conn.execute_batch("VACUUM")?;
            Ok(removed)
        })
    }
}
