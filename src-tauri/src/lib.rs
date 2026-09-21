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
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("rhythm.sqlite");
            let db = Database::open(&db_path)?;
            // Enforce the stored retention policy at startup.
            db.with_conn(|conn| privacy::retention::apply(conn, chrono::Utc::now()))?;
            app.manage(db);

            app::tray::install(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::delete_all_local_data,
            app::commands::get_retention_policy,
            app::commands::set_retention_policy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Personal Rhythm Assistant");
}
