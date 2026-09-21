//! Menu bar / tray placeholder (Milestone 0).
//!
//! Only "Open app" and "Quit" are wired. The remaining commands from
//! IMPLEMENTATION.md §16 are added by the milestones that own them.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub const MENU_OPEN: &str = "open";
pub const MENU_QUIT: &str = "quit";

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open app", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Personal Rhythm Assistant")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_OPEN => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            MENU_QUIT => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
