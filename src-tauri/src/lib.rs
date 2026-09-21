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

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app::tray::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Personal Rhythm Assistant");
}
