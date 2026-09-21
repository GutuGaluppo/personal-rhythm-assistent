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

use persistence::Database;
use sensors::bus::EventBus;
use sensors::collector::Collector;
use sensors::service::{SensorService, SharedSnapshot};
use std::sync::Arc;
use tauri::Manager;

/// Must match `identifier` in tauri.conf.json. This app's own windows are not user activity.
const OWN_BUNDLE_ID: &str = "app.personalrhythm.assistant";

/// Identities this app can have in the frontmost-app probe: its bundle id when
/// packaged, or the `name:` fallback for an unbundled dev binary.
fn own_app_ids() -> Vec<String> {
    vec![
        OWN_BUNDLE_ID.to_string(),
        format!("name:{}", env!("CARGO_PKG_NAME")),
    ]
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
                Collector::with_defaults(own_app_ids()),
                db.clone(),
                bus.clone(),
                snapshot.clone(),
            )
            .spawn();

            app.manage(db);
            app.manage(bus);
            app.manage(snapshot);

            app::tray::install(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::delete_all_local_data,
            app::commands::get_retention_policy,
            app::commands::set_retention_policy,
            app::commands::get_privacy_toggles,
            app::commands::set_privacy_toggles,
            app::commands::get_sensor_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Personal Rhythm Assistant");
}
