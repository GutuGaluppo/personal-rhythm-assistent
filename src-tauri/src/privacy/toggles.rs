//! Privacy toggles (IMPLEMENTATION.md §22).

use crate::persistence::error::Result;
use crate::persistence::repositories::settings;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const SETTINGS_KEY: &str = "privacy_toggles";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyToggles {
    pub active_application: bool,
    /// Enforced by the Sessionizer (Milestone 3): when off, no sessions are built.
    pub active_time: bool,
    pub idle_detection: bool,
    // Placeholders: not implemented yet, so they can never be switched on
    // (they must not pretend to be active).
    pub window_title: bool,
    pub keyboard_mouse_rhythm: bool,
    pub calendar: bool,
    pub cloud_processing: bool,
}

impl Default for PrivacyToggles {
    fn default() -> Self {
        Self {
            active_application: true,
            active_time: true,
            idle_detection: true,
            window_title: false,
            keyboard_mouse_rhythm: false,
            calendar: false,
            cloud_processing: false,
        }
    }
}

impl PrivacyToggles {
    pub fn all_off() -> Self {
        Self {
            active_application: false,
            active_time: false,
            idle_detection: false,
            ..Self::default().without_placeholders()
        }
    }

    /// Forces every not-yet-functional toggle off.
    pub fn without_placeholders(mut self) -> Self {
        self.window_title = false;
        self.keyboard_mouse_rhythm = false;
        self.calendar = false;
        self.cloud_processing = false;
        self
    }
}

pub fn load(conn: &Connection) -> Result<PrivacyToggles> {
    let stored: Option<PrivacyToggles> = settings::get_json(conn, SETTINGS_KEY)?;
    Ok(stored.unwrap_or_default().without_placeholders())
}

/// Stores the toggles and returns what was actually stored (placeholders forced off).
pub fn save(conn: &Connection, toggles: PrivacyToggles) -> Result<PrivacyToggles> {
    let toggles = toggles.without_placeholders();
    settings::set_json(conn, SETTINGS_KEY, &toggles)?;
    Ok(toggles)
}
