use crate::context;
use crate::context::engine::ContextConfig;
use crate::context::signals::ContextAssessment;
use crate::interest_inbox;
use crate::interventions::manager::InterventionManager;
use crate::interventions::model::{Answer, InterventionView};
use crate::interventions::pause::{PauseKind, PauseService, PauseView};
use crate::persistence::repositories::activity_events;
use crate::persistence::repositories::interests::Interest;
use crate::persistence::Database;
use crate::policy::rules::PolicyDecision;
use crate::policy::service::{PolicyService, PolicyView};
use crate::privacy::retention::{self, RetentionPolicy};
use crate::privacy::toggles::{self, PrivacyToggles};
use crate::reports::daily::{self, DailySummary};
use crate::reports::my_day::{self, MyDay};
use crate::reports::weekly::{self, WeeklyConfig, WeeklyReview};
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

// ---- Check-in window. These three are the ONLY commands its capability allows. ----

#[tauri::command]
pub fn get_current_intervention(
    manager: State<'_, Arc<InterventionManager>>,
) -> Option<InterventionView> {
    manager.current()
}

/// The next screen, or `None` once the check-in is over.
#[tauri::command]
pub fn answer_intervention(
    manager: State<'_, Arc<InterventionManager>>,
    id: String,
    answer: Answer,
) -> Result<Option<InterventionView>, String> {
    manager.answer(chrono::Utc::now(), &id, answer).map_err(err)
}

#[tauri::command]
pub fn dismiss_intervention(
    manager: State<'_, Arc<InterventionManager>>,
    id: String,
) -> Result<(), String> {
    manager.dismiss(chrono::Utc::now(), &id).map_err(err)
}

/// Developer preview of the check-in: skips the policy and records nothing.
#[tauri::command]
pub fn debug_show_intervention(
    db: State<'_, Arc<Database>>,
    sessions: State<'_, Arc<Mutex<SessionService>>>,
    manager: State<'_, Arc<InterventionManager>>,
) -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("only available in development builds".into());
    }
    let now = chrono::Utc::now();
    let assessment = {
        let service = sessions.lock().unwrap();
        context::service::assess_now(&db, &service, &ContextConfig::default(), now).map_err(err)?
    };
    manager.preview(now, &assessment).map(|_| ()).map_err(err)
}

// ---- Pause window. These three are the ONLY commands its capability allows. ----

#[tauri::command]
pub fn get_pause_view(pause: State<'_, Arc<PauseService>>) -> Option<PauseView> {
    pause.view(chrono::Utc::now())
}

/// `minutes` must be 1-60; the presets are 3, 5 and 10.
#[tauri::command]
pub fn start_pause(
    pause: State<'_, Arc<PauseService>>,
    kind: PauseKind,
    minutes: u32,
) -> Result<PauseView, String> {
    pause.start(kind, minutes, chrono::Utc::now()).map_err(err)
}

/// Coming back, or ending early: clears the pause and closes the window.
#[tauri::command]
pub fn end_pause(pause: State<'_, Arc<PauseService>>) {
    pause.end(chrono::Utc::now());
}

/// From the main window ("Take a break"): opens the pause window on its setup screen.
#[tauri::command]
pub fn open_pause(pause: State<'_, Arc<PauseService>>) {
    pause.offer(PauseKind::Silence);
}

// ---- Interest Inbox ----

#[tauri::command]
pub fn list_interests(
    db: State<'_, Arc<Database>>,
    archived: bool,
) -> Result<Vec<Interest>, String> {
    db.with_conn(|c| interest_inbox::list(c, archived))
        .map_err(err)
}

#[tauri::command]
pub fn add_interest(db: State<'_, Arc<Database>>, text: String) -> Result<Interest, String> {
    db.with_conn(|c| interest_inbox::add(c, &text, chrono::Utc::now()))
        .map_err(err)
}

#[tauri::command]
pub fn archive_interest(db: State<'_, Arc<Database>>, id: i64) -> Result<bool, String> {
    db.with_conn(|c| interest_inbox::archive(c, id, chrono::Utc::now()))
        .map_err(err)
}

#[tauri::command]
pub fn restore_interest(db: State<'_, Arc<Database>>, id: i64) -> Result<bool, String> {
    db.with_conn(|c| interest_inbox::restore(c, id))
        .map_err(err)
}

#[tauri::command]
pub fn delete_interest(db: State<'_, Arc<Database>>, id: i64) -> Result<bool, String> {
    db.with_conn(|c| interest_inbox::delete(c, id)).map_err(err)
}

/// One suggestion per local day; the same one all day.
#[tauri::command]
pub fn get_interest_suggestion(db: State<'_, Arc<Database>>) -> Result<Option<Interest>, String> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.with_conn(|c| interest_inbox::suggestion_for(c, &today))
        .map_err(err)
}

// ---- Daily summary ----

/// `date` is "YYYY-MM-DD" in the user's local time; omitted means today.
#[tauri::command]
pub fn get_daily_summary(
    db: State<'_, Arc<Database>>,
    sessions: State<'_, Arc<Mutex<SessionService>>>,
    date: Option<String>,
) -> Result<DailySummary, String> {
    let (now, tz) = (chrono::Utc::now(), daily::local_offset());
    let date = match date {
        Some(d) => daily::parse_date(&d).map_err(err)?,
        None => daily::today(now, tz),
    };
    let service = sessions.lock().unwrap();
    daily::summary_for(&db, &service, now, tz, date).map_err(err)
}

/// Days that have something to show, newest first (always includes today).
#[tauri::command]
pub fn list_summary_days(db: State<'_, Arc<Database>>) -> Result<Vec<String>, String> {
    db.with_conn(|c| daily::list_days(c, chrono::Utc::now(), daily::local_offset()))
        .map_err(err)
}

/// Saves the user's own words about a day; empty text clears it.
#[tauri::command]
pub fn save_reflection(
    db: State<'_, Arc<Database>>,
    date: String,
    text: String,
) -> Result<(), String> {
    let date = daily::parse_date(&date).map_err(err)?;
    db.with_conn(|c| {
        daily::save_reflection(c, date, &text, chrono::Utc::now(), daily::local_offset())
    })
    .map_err(err)
}

// ---- Weekly review ----

/// The last seven local days, ending today.
#[tauri::command]
pub fn get_weekly_review(
    db: State<'_, Arc<Database>>,
    sessions: State<'_, Arc<Mutex<SessionService>>>,
) -> Result<WeeklyReview, String> {
    let service = sessions.lock().unwrap();
    weekly::review(
        &db,
        &service,
        &WeeklyConfig::default(),
        chrono::Utc::now(),
        daily::local_offset(),
    )
    .map_err(err)
}
