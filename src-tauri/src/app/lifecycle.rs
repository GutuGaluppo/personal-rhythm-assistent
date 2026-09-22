//! Keeping the app alive in the background (IMPLEMENTATION.md §3, §27: "runs in
//! background"). The window is only how you look at the app; the sensors and the
//! menu bar are the app. Closing the window hides it, and only Quit ends the app.

use tauri::{AppHandle, Manager};

pub const MAIN_WINDOW: &str = "main";

/// Closing the main window hides it. The small windows are opened and closed by
/// the app itself and really close.
pub fn hide_instead_of_closing(window_label: &str) -> bool {
    window_label == MAIN_WINDOW
}

/// Tauri asks to exit when the last window goes away (`code == None`); that must
/// not end the app. An explicit exit, such as the menu bar's Quit, carries a code.
pub fn keep_running_after(exit_code: Option<i32>) -> bool {
    exit_code.is_none()
}

/// Shows and focuses the main window, e.g. from the menu bar or the Dock icon.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
