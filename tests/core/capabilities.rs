//! The check-in window must not have broad app permissions (IMPLEMENTATION.md §16).
//! Three lists describe the same commands and must not drift apart:
//! `build.rs` (the ACL manifest), the invoke handler in `lib.rs`, and the capabilities.

use std::collections::BTreeSet;
use std::path::PathBuf;

const INTERVENTION_WINDOW_COMMANDS: [&str; 3] = [
    "get_current_intervention",
    "answer_intervention",
    "dismiss_intervention",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}
fn allow(command: &str) -> String {
    format!("allow-{}", command.replace('_', "-"))
}

/// Quoted names between `COMMANDS: &[&str] = &[` and `];` in build.rs.
fn manifest_commands() -> BTreeSet<String> {
    let src = read("build.rs");
    let start =
        src.find("COMMANDS: &[&str] = &[").expect("COMMANDS list") + "COMMANDS: &[&str] = &[".len();
    let end = start + src[start..].find("];").unwrap();
    src[start..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(String::from)
        .collect()
}

/// `app::commands::<name>` entries in the invoke handler.
fn handler_commands() -> BTreeSet<String> {
    let src = read("src/lib.rs");
    let start = src.find("generate_handler![").expect("invoke handler");
    let end = start + src[start..].find("])").unwrap();
    src[start..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("app::commands::"))
        .map(|l| l.trim_end_matches(',').to_string())
        .collect()
}

fn capability(file: &str) -> (Vec<String>, BTreeSet<String>) {
    let json: serde_json::Value =
        serde_json::from_str(&read(&format!("capabilities/{file}"))).unwrap();
    let list = |key: &str| -> Vec<String> {
        json[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    (list("windows"), list("permissions").into_iter().collect())
}

#[test]
fn the_manifest_and_the_invoke_handler_list_the_same_commands() {
    assert_eq!(manifest_commands(), handler_commands());
    assert!(manifest_commands().len() >= 20, "list parsing looks broken");
}

#[test]
fn the_check_in_window_can_call_exactly_three_commands_and_nothing_from_core() {
    let (windows, permissions) = capability("intervention.json");
    assert_eq!(windows, ["intervention"]);
    let expected: BTreeSet<String> = INTERVENTION_WINDOW_COMMANDS
        .iter()
        .map(|c| allow(c))
        .collect();
    assert_eq!(permissions, expected);
    assert!(
        permissions.iter().all(|p| !p.starts_with("core:")),
        "no core permissions"
    );
}

#[test]
fn the_check_in_window_cannot_reach_the_main_screens_commands() {
    let (_, permissions) = capability("intervention.json");
    for command in manifest_commands() {
        if !INTERVENTION_WINDOW_COMMANDS.contains(&command.as_str()) {
            assert!(
                !permissions.contains(&allow(&command)),
                "{command} must not be allowed"
            );
        }
    }
}

#[test]
fn the_main_window_is_granted_every_command_it_needs_and_no_more() {
    let (windows, permissions) = capability("default.json");
    assert_eq!(windows, ["main"]);
    for command in manifest_commands() {
        let granted = permissions.contains(&allow(&command));
        if INTERVENTION_WINDOW_COMMANDS.contains(&command.as_str()) {
            assert!(!granted, "{command} belongs to the check-in window");
        } else {
            assert!(granted, "main window is missing {command}");
        }
    }
    // No permission that does not correspond to a command (typos would silently do nothing).
    let known: BTreeSet<String> = manifest_commands().iter().map(|c| allow(c)).collect();
    for p in permissions.iter().filter(|p| p.starts_with("allow-")) {
        assert!(known.contains(p), "unknown permission {p}");
    }
}

#[test]
fn no_capability_targets_a_wildcard_window() {
    for file in ["default.json", "intervention.json"] {
        let (windows, _) = capability(file);
        assert!(
            windows.iter().all(|w| w != "*"),
            "{file} must name its windows"
        );
    }
}
