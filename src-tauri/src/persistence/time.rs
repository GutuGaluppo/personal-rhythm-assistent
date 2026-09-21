//! One canonical timestamp format so SQL string comparison is chronological.

use super::error::{PersistenceError, Result};
use chrono::{DateTime, SecondsFormat, Utc};

pub fn format(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn now() -> String {
    format(Utc::now())
}

/// Parses any RFC 3339 timestamp and returns it as UTC with millisecond precision.
pub fn normalize(s: &str) -> Result<String> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| format(t.with_timezone(&Utc)))
        .map_err(|e| PersistenceError::Invalid(format!("timestamp {s:?}: {e}")))
}
