//! Milestone 14 (pulled forward): "I'm on fire" can be activated from the keyboard.
//! Post-v0.1 adds a second shortcut for a quick, untimed pause.

use chrono::{TimeZone, Utc};
use personal_rhythm_assistant_lib::app::shortcuts::{
    is_on_fire, is_quick_pause, ON_FIRE, QUICK_PAUSE,
};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::policy::rules::PolicyConfig;
use personal_rhythm_assistant_lib::policy::service::PolicyService;
use std::str::FromStr;
use std::sync::Arc;
use tauri_plugin_global_shortcut::Shortcut;

#[test]
fn the_shortcut_strings_parse() {
    for shortcut in [ON_FIRE, QUICK_PAUSE] {
        assert!(
            Shortcut::from_str(shortcut).is_ok(),
            "{shortcut:?} must be a valid accelerator"
        );
    }
}

#[test]
fn the_two_shortcuts_are_distinct() {
    assert_ne!(ON_FIRE, QUICK_PAUSE);
}

#[test]
fn only_the_exact_combination_matches() {
    let matching = Shortcut::from_str(ON_FIRE).unwrap();
    assert!(is_on_fire(&matching));
    assert!(
        !is_quick_pause(&matching),
        "must not also trigger quick pause"
    );

    for other in [
        "CommandOrControl+F",
        "CommandOrControl+Alt+F",
        "Alt+Shift+F",
        QUICK_PAUSE,
    ] {
        let s = Shortcut::from_str(other).unwrap();
        assert!(!is_on_fire(&s), "{other:?} must not trigger it");
    }
}

#[test]
fn only_the_exact_quick_pause_combination_matches() {
    let matching = Shortcut::from_str(QUICK_PAUSE).unwrap();
    assert!(is_quick_pause(&matching));
    assert!(!is_on_fire(&matching), "must not also trigger on-fire");

    for other in [
        "CommandOrControl+P",
        "CommandOrControl+Alt+P",
        "Alt+Shift+P",
        ON_FIRE,
    ] {
        let s = Shortcut::from_str(other).unwrap();
        assert!(!is_quick_pause(&s), "{other:?} must not trigger it");
    }
}

#[test]
fn toggling_on_fire_is_the_same_action_the_menu_bar_and_the_shortcut_both_use() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let policy = PolicyService::new(db, PolicyConfig::default());
    let now = Utc.with_ymd_and_hms(2026, 3, 4, 10, 0, 0).unwrap();
    let day_start = Utc.with_ymd_and_hms(2026, 3, 4, 0, 0, 0).unwrap();

    assert!(policy.toggle_on_fire(now, day_start).unwrap(), "turns on");
    assert!(policy.state().unwrap().on_fire_active(now));
    assert!(
        !policy.toggle_on_fire(now, day_start).unwrap(),
        "turns off again"
    );
    assert!(!policy.state().unwrap().on_fire_active(now));
}
