//! Menu bar / tray (IMPLEMENTATION.md §16). Commands are added by the milestones
//! that own them. Order follows §16: rhythm, break, on fire, inbox, open, silence.

use crate::interventions::pause::{PauseKind, PauseService};
use crate::policy::service::{PolicyService, PolicyView};
use crate::reports::my_day::local_day_start;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

pub const MENU_RHYTHM: &str = "how_is_my_rhythm";
pub const MENU_INBOX: &str = "interest_inbox";
pub const MENU_BREAK: &str = "take_a_break";
pub const MENU_ON_FIRE: &str = "on_fire";
pub const MENU_OPEN: &str = "open";
pub const MENU_SILENCE: &str = "silence";
pub const MENU_QUIT: &str = "quit";
const TRAY_ID: &str = "main";

/// Icons and check marks are re-synced this often (on-fire and pauses end by themselves).
const SYNC_INTERVAL: Duration = Duration::from_secs(5);

/// Which icon the menu bar shows. One at a time, in this order of precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Paused,
    /// The approved warm state. Never animated.
    OnFire,
    Silent,
    Normal,
}

pub fn tray_state(view: &PolicyView, pause_active: bool) -> TrayState {
    if pause_active {
        TrayState::Paused
    } else if view.on_fire_until.is_some() {
        TrayState::OnFire
    } else if view.silent {
        TrayState::Silent
    } else {
        TrayState::Normal
    }
}

/// The state in words: an icon's meaning is never carried by its look alone.
pub fn tooltip(state: TrayState, on_fire_until: Option<DateTime<Utc>>) -> String {
    const NAME: &str = "Personal Rhythm Assistant";
    match state {
        TrayState::Paused => format!("{NAME} · In a pause"),
        TrayState::OnFire => match on_fire_until {
            Some(until) => format!("{NAME} · I'm on fire until {}", until.format("%H:%M UTC")),
            None => format!("{NAME} · I'm on fire"),
        },
        TrayState::Silent => format!("{NAME} · Silent"),
        TrayState::Normal => NAME.to_string(),
    }
}

fn icon(state: TrayState) -> Image<'static> {
    let bytes: &[u8] = match state {
        TrayState::Paused => include_bytes!("../../icons/tray/pause.png"),
        TrayState::OnFire => include_bytes!("../../icons/tray/on_fire.png"),
        TrayState::Silent => include_bytes!("../../icons/tray/silent.png"),
        TrayState::Normal => include_bytes!("../../icons/tray/default.png"),
    };
    Image::from_bytes(bytes).expect("bundled tray icons are valid PNGs")
}

struct Items {
    on_fire: CheckMenuItem<tauri::Wry>,
    silence: CheckMenuItem<tauri::Wry>,
}

/// Brings icon, tooltip and check marks in line with the actual state.
fn sync(app: &AppHandle, policy: &PolicyService, pause: &PauseService, items: &Items) {
    let now = Utc::now();
    let Ok(view) = policy.view(now, local_day_start(now)) else {
        return;
    };
    let _ = items.on_fire.set_checked(view.on_fire_until.is_some());
    let _ = items.silence.set_checked(view.silent);

    let state = tray_state(&view, pause.is_active());
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon(state)));
        let _ = tray.set_tooltip(Some(tooltip(state, view.on_fire_until)));
    }
}

pub fn install(
    app: &AppHandle,
    policy: Arc<PolicyService>,
    pause: Arc<PauseService>,
) -> tauri::Result<()> {
    let view = policy.view(Utc::now(), local_day_start(Utc::now())).ok();
    let rhythm = MenuItem::with_id(app, MENU_RHYTHM, "How is my rhythm?", true, None::<&str>)?;
    let take_break = MenuItem::with_id(app, MENU_BREAK, "Take a break", true, None::<&str>)?;
    let on_fire = CheckMenuItem::with_id(
        app,
        MENU_ON_FIRE,
        "I'm on fire",
        true,
        view.as_ref().is_some_and(|v| v.on_fire_until.is_some()),
        None::<&str>,
    )?;
    let inbox = MenuItem::with_id(app, MENU_INBOX, "Interest Inbox", true, None::<&str>)?;
    let open = MenuItem::with_id(app, MENU_OPEN, "Open app", true, None::<&str>)?;
    let silence = CheckMenuItem::with_id(
        app,
        MENU_SILENCE,
        "Silence",
        true,
        view.as_ref().is_some_and(|v| v.silent),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &rhythm,
            &take_break,
            &on_fire,
            &inbox,
            &open,
            &silence,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let items = Arc::new(Items { on_fire, silence });

    let (h_policy, h_pause, h_items) = (policy.clone(), pause.clone(), items.clone());
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip(TrayState::Normal, None))
        .icon(icon(TrayState::Normal))
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let now = Utc::now();
            match event.id.as_ref() {
                MENU_RHYTHM => show_main(app, Some("my-day")),
                MENU_INBOX => show_main(app, Some("interest-inbox")),
                MENU_BREAK => h_pause.offer(PauseKind::Silence),
                MENU_ON_FIRE => {
                    let on = !h_policy
                        .view(now, local_day_start(now))
                        .is_ok_and(|v| v.on_fire_until.is_some());
                    let _ = h_policy.set_on_fire(on, now);
                }
                MENU_SILENCE => {
                    let on = !h_policy.state().is_ok_and(|s| s.silent);
                    let _ = h_policy.set_silent(on);
                }
                MENU_OPEN => show_main(app, None),
                MENU_QUIT => app.exit(0),
                _ => {}
            }
            sync(app, &h_policy, &h_pause, &h_items);
        });
    builder.build(app)?;

    let handle = app.clone();
    std::thread::Builder::new()
        .name("tray-sync".into())
        .spawn(move || loop {
            sync(&handle, &policy, &pause, &items);
            std::thread::sleep(SYNC_INTERVAL);
        })
        .expect("spawn tray sync thread");
    Ok(())
}

/// Brings the main window forward, optionally on a specific page (the app listens
/// for the `navigate` event).
fn show_main(app: &AppHandle, page: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Some(page) = page {
        let _ = app.emit_to("main", "navigate", page);
    }
}
