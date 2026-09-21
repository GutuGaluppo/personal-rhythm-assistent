use crate::persistence::Database;
use crate::privacy::retention::{self, RetentionPolicy};
use crate::privacy::toggles::{self, PrivacyToggles};
use crate::sensors::service::SharedSnapshot;
use crate::sensors::system_state::SensorSnapshot;
use crate::sessions::model::Session;
use crate::sessions::service::SessionService;
use std::sync::{Arc, Mutex};
use tauri::State;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn delete_all_local_data(db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.delete_all_local_data().map_err(err)
}

#[tauri::command]
pub fn get_retention_policy(db: State<'_, Arc<Database>>) -> Result<RetentionPolicy, String> {
    db.with_conn(retention::load).map_err(err)
}

#[tauri::command]
pub fn set_retention_policy(
    db: State<'_, Arc<Database>>,
    policy: RetentionPolicy,
) -> Result<(), String> {
    db.with_conn(|conn| retention::save(conn, &policy))
        .map_err(err)
}

#[tauri::command]
pub fn get_privacy_toggles(db: State<'_, Arc<Database>>) -> Result<PrivacyToggles, String> {
    db.with_conn(toggles::load).map_err(err)
}

/// Returns the toggles as stored: not-yet-functional ones are forced off.
#[tauri::command]
pub fn set_privacy_toggles(
    db: State<'_, Arc<Database>>,
    toggles: PrivacyToggles,
) -> Result<PrivacyToggles, String> {
    db.with_conn(|conn| toggles::save(conn, toggles))
        .map_err(err)
}

#[tauri::command]
pub fn get_sensor_state(snapshot: State<'_, SharedSnapshot>) -> SensorSnapshot {
    snapshot.lock().unwrap().clone()
}

/// The session in progress, computed from the stored events right now.
#[tauri::command]
pub fn get_current_session(
    service: State<'_, Arc<Mutex<SessionService>>>,
) -> Result<Option<Session>, String> {
    service
        .lock()
        .unwrap()
        .refresh(chrono::Utc::now())
        .map_err(err)
}
