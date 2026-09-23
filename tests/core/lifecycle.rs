//! The app keeps running in the background: closing its window is not quitting.

use personal_rhythm_assistant_lib::app::lifecycle::{
    hide_instead_of_closing, keep_running_after, MAIN_WINDOW,
};

#[test]
fn closing_the_main_window_hides_it_instead() {
    assert!(hide_instead_of_closing(MAIN_WINDOW));
    assert!(hide_instead_of_closing("main"));
}

#[test]
fn the_small_windows_really_close() {
    for label in ["intervention", "pause"] {
        assert!(
            !hide_instead_of_closing(label),
            "{label} is opened and closed by the app itself"
        );
    }
}

#[test]
fn the_last_window_going_away_does_not_end_the_app() {
    assert!(keep_running_after(None));
}

#[test]
fn an_explicit_quit_does_end_it() {
    for code in [0, 1, -1] {
        assert!(!keep_running_after(Some(code)), "exit code {code}");
    }
}

#[test]
fn the_menu_bar_can_still_quit() {
    // The tray's Quit calls `app.exit(0)`, which carries a code, so it is never held back.
    let tray =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/tray.rs")).unwrap();
    assert!(tray.contains("MENU_QUIT => app.exit(0)"));
}
