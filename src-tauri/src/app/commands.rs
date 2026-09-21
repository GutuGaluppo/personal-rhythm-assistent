use crate::context;
use crate::context::engine::ContextConfig;
use crate::context::signals::ContextAssessment;
use crate::persistence::repositories::activity_events;
use crate::persistence::Database;
use crate::policy::rules::PolicyDecision;
use crate::policy::service::{PolicyService, PolicyView};
use crate::privacy::retention::{self, RetentionPolicy};
use crate::privacy::toggles::{self, PrivacyToggles};
use crate::reports::my_day::{self, MyDay};
use crate::sensors::service::SharedSnapshot;
use crate::sensors::system_state::SensorSnapshot;
use crate::sessions::classification::{AppMapping, Classification};
use crate::sessions::model::{Category, Session};
use crate::sessions::service::SessionService;
use crate::sessions::sessionizer::SessionConfig;
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

#[tauri::command]
pub fn get_my_day(
    db: State<'_, Arc<Database>>,
    sessions: State<'_, Arc<Mutex<SessionService>>>,
    snapshot: State<'_, SharedSnapshot>,
) -> Result<MyDay, String> {
    let now = chrono::Utc::now();
    let service = sessions.lock().unwrap();
    my_day::load(
        &db,
        &service,
        &snapshot,
        now,
        my_day::local_day_start(now),
        SessionConfig::default().break_gap_secs as u32,
    )
    .map_err(err)
}

#[tauri::command]
pub fn list_app_mappings(
    db: State<'_, Arc<Database>>,
    classification: State<'_, Arc<Classification>>,
) -> Result<Vec<AppMapping>, String> {
    db.with_conn(|conn| classification.list_mappings(conn))
        .map_err(err)
}

#[tauri::command]
pub fn set_app_category(
    db: State<'_, Arc<Database>>,
    classification: State<'_, Arc<Classification>>,
    bundle_id: String,
    category: Category,
) -> Result<(), String> {
    db.with_conn(|conn| {
        let name = activity_events::distinct_applications(conn)?
            .into_iter()
            .find(|(id, _)| *id == bundle_id)
            .map(|(_, name)| name);
        classification.set(conn, &bundle_id, name.as_deref(), category)
    })
    .map_err(err)
}

#[tauri::command]
pub fn reset_app_category(
    db: State<'_, Arc<Database>>,
    classification: State<'_, Arc<Classification>>,
    bundle_id: String,
) -> Result<(), String> {
    db.with_conn(|conn| classification.reset(conn, &bundle_id))
        .map_err(err)
}

/// Developer view: the signals and the evidence behind them, right now.
#[tauri::command]
pub fn get_context_assessment(
    db: State<'_, Arc<Database>>,
    sessions: State<'_, Arc<Mutex<SessionService>>>,
) -> Result<ContextAssessment, String> {
    let service = sessions.lock().unwrap();
    context::service::assess_now(&db, &service, &ContextConfig::default(), chrono::Utc::now())
        .map_err(err)
}

#[tauri::command]
pub fn get_policy_view(policy: State<'_, Arc<PolicyService>>) -> Result<PolicyView, String> {
    let now = chrono::Utc::now();
    policy.view(now, my_day::local_day_start(now)).map_err(err)
}

#[tauri::command]
pub fn set_silence(policy: State<'_, Arc<PolicyService>>, silent: bool) -> Result<(), String> {
    policy.set_silent(silent).map_err(err)
}

#[tauri::command]
pub fn set_on_fire(policy: State<'_, Arc<PolicyService>>, on: bool) -> Result<(), String> {
    policy.set_on_fire(on, chrono::Utc::now()).map_err(err)
}

#[derive(serde::Serialize)]
pub struct PolicyDebug {
    view: PolicyView,
    decision: PolicyDecision,
}

/// Developer view: what the Policy Engine would do with the current evidence, and why.
#[tauri::command]
pub fn get_policy_debug(
    db: State<'_, Arc<Database>>,
    sessions: State<'_, Arc<Mutex<SessionService>>>,
    policy: State<'_, Arc<PolicyService>>,
) -> Result<PolicyDebug, String> {
    let now = chrono::Utc::now();
    let day_start = my_day::local_day_start(now);
    let assessment = {
        let service = sessions.lock().unwrap();
        context::service::assess_now(&db, &service, &ContextConfig::default(), now).map_err(err)?
    };
    Ok(PolicyDebug {
        view: policy.view(now, day_start).map_err(err)?,
        decision: policy
            .decide(now, day_start, assessment.active_signal_count)
            .map_err(err)?,
    })
}
