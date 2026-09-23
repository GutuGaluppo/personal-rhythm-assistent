use std::fmt;

#[derive(Debug)]
pub enum PersistenceError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    /// Caller supplied data the store refuses to hold (bad timestamp, zero retention...).
    Invalid(String),
    /// The database was written by a newer app version.
    SchemaTooNew {
        found: usize,
        supported: usize,
    },
}

pub type Result<T> = std::result::Result<T, PersistenceError>;

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "database error: {e}"),
            Self::Json(e) => write!(f, "stored value is not valid JSON: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Invalid(m) => write!(f, "invalid data: {m}"),
            Self::SchemaTooNew { found, supported } => write!(
                f,
                "database schema version {found} is newer than this app supports ({supported})"
            ),
        }
    }
}

impl std::error::Error for PersistenceError {}

impl From<rusqlite::Error> for PersistenceError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
impl From<serde_json::Error> for PersistenceError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<std::io::Error> for PersistenceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
