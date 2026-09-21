//! Menu bar / tray (IMPLEMENTATION.md §16). Commands are added by the milestones
//! that own them; this one owns "I'm on fire" and "Silence".

use crate::policy::service::PolicyService;
use crate::reports::my_day::local_day_start;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub const MENU_ON_FIRE: &str = "on_fire";
pub const MENU_OPEN: &str = "open";
pub const MENU_SILENCE: &str = "silence";
pub const MENU_QUIT: &str = "quit";

/// "I'm on fire" ends by itself; keep its check mark honest.
const SYNC_INTERVAL: Duration = Duration::from_secs(30);

pub fn install(app: &AppHandle, policy: Arc<PolicyService>) -> tauri::Result<()> {
    let state = policy.view(Utc::now(), local_day_start(Utc::now())).ok();
    let on_fire = CheckMenuItem::with_id(
        app,
        MENU_ON_FIRE,
        "I'm on fire",
        true,
        state.as_ref().is_some_and(|s| s.on_fire_until.is_some()),
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, MENU_OPEN, "Open app", true, None::<&str>)?;
    let silence = CheckMenuItem::with_id(
        app,
        MENU_SILENCE,
        "Silence",
        true,
        state.as_ref().is_some_and(|s| s.silent),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &on_fire,
            &open,
            &silence,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let handler_policy = policy.clone();
    let (fire_item, silence_item) = (on_fire.clone(), silence.clone());
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Personal Rhythm Assistant")
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let now = Utc::now();
            match event.id.as_ref() {
                MENU_ON_FIRE => {
                    let view = handler_policy.view(now, local_day_start(now));
                    let turn_on = !view.is_ok_and(|v| v.on_fire_until.is_some());
                    if handler_policy.set_on_fire(turn_on, now).is_ok() {
                        let _ = fire_item.set_checked(turn_on);
                    }
                }
                MENU_SILENCE => {
                    let turn_on = !handler_policy.state().is_ok_and(|s| s.silent);
                    if handler_policy.set_silent(turn_on).is_ok() {
                        let _ = silence_item.set_checked(turn_on);
                    }
                }
                MENU_OPEN => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                MENU_QUIT => app.exit(0),
                _ => {}
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    std::thread::Builder::new()
        .name("tray-sync".into())
        .spawn(move || loop {
            std::thread::sleep(SYNC_INTERVAL);
            let now = Utc::now();
            if let Ok(view) = policy.view(now, local_day_start(now)) {
                let _ = on_fire.set_checked(view.on_fire_until.is_some());
                let _ = silence.set_checked(view.silent);
            }
        })
        .expect("spawn tray sync thread");
    Ok(())
}
