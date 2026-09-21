pub mod app;
pub mod context;
pub mod interventions;
pub mod persistence;
pub mod platform;
pub mod policy;
pub mod privacy;
pub mod reports;
pub mod sensors;
pub mod sessions;

use interventions::manager::{InterventionConfig, InterventionManager};
use interventions::window::TauriPresenter;
use persistence::Database;
use policy::rules::PolicyConfig;
use policy::service::PolicyService;
use sensors::bus::EventBus;
use sensors::collector::Collector;
use sensors::service::{SensorService, SharedSnapshot};
use sessions::classification::Classification;
use sessions::service::SessionService;
use sessions::sessionizer::{SessionConfig, Sessionizer};
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Must match `identifier` in tauri.conf.json. This app's own windows are not user activity.
const OWN_BUNDLE_ID: &str = "app.personalrhythm.assistant";

/// System UI that takes focus when the user is away (lock screen, screen saver).
/// It is not something the user works in, so it must not read as a context switch.
const SYSTEM_UI_BUNDLE_IDS: [&str; 2] = ["com.apple.loginwindow", "com.apple.ScreenSaver.Engine"];

/// Apps that never count as user activity: this app (by bundle id when packaged, or
/// by the `name:` fallback for an unbundled dev binary) and system UI.
fn ignored_app_ids() -> Vec<String> {
    let mut ids = vec![
        OWN_BUNDLE_ID.to_string(),
        format!("name:{}", env!("CARGO_PKG_NAME")),
    ];
    ids.extend(SYSTEM_UI_BUNDLE_IDS.map(String::from));
    ids
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("rhythm.sqlite");
            let db = Arc::new(Database::open(&db_path)?);
            // Enforce the stored retention policy at startup.
            db.with_conn(|conn| privacy::retention::apply(conn, chrono::Utc::now()))?;

            let bus = Arc::new(EventBus::new());
            let snapshot = SharedSnapshot::default();
            SensorService::new(
                platform::default_probe(),
                Collector::with_defaults(ignored_app_ids()),
                db.clone(),
                bus.clone(),
                snapshot.clone(),
            )
            .spawn();

            let classification = Arc::new(db.with_conn(Classification::load)?);
            let classifier = classification.clone();
            let sessions = Arc::new(Mutex::new(SessionService::new(
                Sessionizer::with_classifier(
                    SessionConfig::default(),
                    Box::new(move |app| classifier.category_for(app)),
                ),
                db.clone(),
                snapshot.clone(),
            )));
            SessionService::spawn(sessions.clone());

            let policy = Arc::new(PolicyService::new(db.clone(), PolicyConfig::default()));

            let db_for_manager = db.clone();
            let db_for_scheduler = db.clone();
            let sessions_for_scheduler = sessions.clone();
            let snapshot_for_scheduler = snapshot.clone();

            app.manage(db);
            app.manage(sessions);
            app.manage(classification);
            app.manage(bus);
            app.manage(snapshot);

            let manager = Arc::new(InterventionManager::new(
                db_for_manager,
                policy.clone(),
                Box::new(TauriPresenter::new(app.handle().clone())),
                InterventionConfig::default(),
            ));
            app.manage(manager.clone());
            interventions::scheduler::spawn(
                manager,
                db_for_scheduler,
                sessions_for_scheduler,
                snapshot_for_scheduler,
            );

            app::tray::install(app.handle(), policy.clone())?;
            app.manage(policy);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::delete_all_local_data,
            app::commands::get_retention_policy,
            app::commands::set_retention_policy,
            app::commands::get_privacy_toggles,
            app::commands::set_privacy_toggles,
            app::commands::get_sensor_state,
            app::commands::get_current_session,
            app::commands::get_my_day,
            app::commands::list_app_mappings,
            app::commands::set_app_category,
            app::commands::reset_app_category,
            app::commands::get_context_assessment,
            app::commands::get_policy_view,
            app::commands::set_silence,
            app::commands::set_on_fire,
            app::commands::get_policy_debug,
            app::commands::get_current_intervention,
            app::commands::answer_intervention,
            app::commands::dismiss_intervention,
            app::commands::debug_show_intervention,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Personal Rhythm Assistant");
}
