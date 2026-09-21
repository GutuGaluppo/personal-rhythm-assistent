use serde::Serialize;

/// Whether the user is at the machine, as far as the enabled sensors can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    Active,
    Idle,
    /// Idle detection is off, so the app does not know.
    Unknown,
}

/// Latest observation, for display. Held in memory only; never persisted.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorSnapshot {
    pub state: ActivityState,
    pub frontmost_application: Option<String>,
    /// RFC 3339 time of the last input, while the user is idle.
    pub idle_since: Option<String>,
}

impl Default for SensorSnapshot {
    fn default() -> Self {
        Self {
            state: ActivityState::Unknown,
            frontmost_application: None,
            idle_since: None,
        }
    }
}
