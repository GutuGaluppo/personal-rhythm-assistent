use crate::persistence::Database;
use crate::privacy::retention::{self, RetentionPolicy};
use tauri::State;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn delete_all_local_data(db: State<'_, Database>) -> Result<(), String> {
    db.delete_all_local_data().map_err(err)
}

#[tauri::command]
pub fn get_retention_policy(db: State<'_, Database>) -> Result<RetentionPolicy, String> {
    db.with_conn(retention::load).map_err(err)
}

#[tauri::command]
pub fn set_retention_policy(
    db: State<'_, Database>,
    policy: RetentionPolicy,
) -> Result<(), String> {
    db.with_conn(|conn| retention::save(conn, &policy))
        .map_err(err)
}
