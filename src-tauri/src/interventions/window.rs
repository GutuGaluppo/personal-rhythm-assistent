//! The dedicated check-in window. Small, undecorated, always visible, and it
//! never takes keyboard focus from what the user is doing.

use super::manager::{InterventionManager, Presenter};
use super::model::InterventionView;
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const WINDOW_LABEL: &str = "intervention";
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 300.0;
const MARGIN: f64 = 24.0;

pub struct TauriPresenter {
    app: AppHandle,
}

impl TauriPresenter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Presenter for TauriPresenter {
    fn show(&self, _view: &InterventionView) -> std::result::Result<(), String> {
        let app = self.app.clone();
        self.app
            .run_on_main_thread(move || {
                if let Err(e) = build(&app) {
                    eprintln!("could not open the check-in window: {e}");
                }
            })
            .map_err(|e| e.to_string())
    }

    fn close(&self) {
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
                let _ = window.destroy();
            }
        });
    }
}

fn build(app: &AppHandle) -> tauri::Result<()> {
    let handle = app.clone();
    let window = WebviewWindowBuilder::new(
        app,
        WINDOW_LABEL,
        WebviewUrl::App("index.html?view=intervention".into()),
    )
    .title("Check-in")
    .inner_size(WIDTH, HEIGHT)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible_on_all_workspaces(true)
    // Never steal the keyboard from someone who is typing.
    .focused(false)
    .build()?;

    // Bottom-right of the primary display, clear of the dock and the menu bar.
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let x = f64::from(size.width) - (WIDTH + MARGIN) * scale;
        let y = f64::from(size.height) - (HEIGHT + MARGIN * 4.0) * scale;
        let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
    }

    // Closed some other way than through the manager: that is a dismissal.
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            if let Some(manager) = handle.try_state::<Arc<InterventionManager>>() {
                let _ = manager.window_closed(Utc::now());
            }
        }
    });
    Ok(())
}

// ---- Pause window ------------------------------------------------------------------

use super::pause::{PausePresenter, PauseService};

pub const PAUSE_WINDOW_LABEL: &str = "pause";

pub struct TauriPausePresenter {
    app: AppHandle,
}

impl TauriPausePresenter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl PausePresenter for TauriPausePresenter {
    fn show(&self) {
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            // Reopening while it is already up just brings it forward. The
            // window is never destroyed between pauses (see `close`), so its
            // page stays loaded; `set_focus` fires a DOM `focus` event the
            // frontend uses to resync instead of trusting stale state.
            if let Some(window) = app.get_webview_window(PAUSE_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.set_focus();
            } else if let Err(e) = build_pause(&app) {
                eprintln!("could not open the pause window: {e}");
            }
        });
    }

    fn close(&self) {
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            // Hidden, not destroyed: destroying and rebuilding the same label
            // raced with `show`'s reuse path and could surface a torn-down,
            // blank webview. Hiding keeps the window (and its JS) around so the
            // next `show` is instant and always has something to display.
            if let Some(window) = app.get_webview_window(PAUSE_WINDOW_LABEL) {
                let _ = window.hide();
            }
        });
    }
}

fn build_pause(app: &AppHandle) -> tauri::Result<()> {
    let handle = app.clone();
    let window = WebviewWindowBuilder::new(
        app,
        PAUSE_WINDOW_LABEL,
        WebviewUrl::App("index.html?view=pause".into()),
    )
    .title("Pause")
    .inner_size(460.0, 540.0)
    .resizable(false)
    .center()
    // The user asked for this one, so it may take focus.
    .focused(true)
    .build()?;

    // Closed some other way than through the app's own buttons (e.g. Cmd+W):
    // hide rather than destroy, so the window stays reusable (see `close`
    // above), and still counts as the user leaving the pause.
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(window) = handle.get_webview_window(PAUSE_WINDOW_LABEL) {
                let _ = window.hide();
            }
            if let Some(pause) = handle.try_state::<Arc<PauseService>>() {
                pause.window_closed(Utc::now());
            }
        }
        // Safety net for paths that do destroy the window outright (app quit).
        WindowEvent::Destroyed => {
            if let Some(pause) = handle.try_state::<Arc<PauseService>>() {
                pause.window_closed(Utc::now());
            }
        }
        _ => {}
    });
    Ok(())
}
