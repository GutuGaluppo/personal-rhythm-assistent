//! The one global keyboard shortcut (IMPLEMENTATION.md §14: "I'm on fire" can be
//! switched on from the keyboard). On macOS a global shortcut needs no permission:
//! the system tells the app when exactly this key combination is pressed, and the
//! app never sees any other key.

use std::str::FromStr;
use tauri_plugin_global_shortcut::Shortcut;

/// Command + Option + Shift + F. Toggles "I'm on fire".
pub const ON_FIRE: &str = "CommandOrControl+Alt+Shift+F";

/// How the same combination is written on a menu item (shown as a hint only).
pub const ON_FIRE_MENU_HINT: &str = "CmdOrCtrl+Alt+Shift+F";

pub fn is_on_fire(pressed: &Shortcut) -> bool {
    Shortcut::from_str(ON_FIRE).is_ok_and(|expected| &expected == pressed)
}
