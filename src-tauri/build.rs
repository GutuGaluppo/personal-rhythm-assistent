/// Every command the frontend may call. Listing them here turns on Tauri's
/// per-window permissions: a window can call a command only if its capability
/// (`capabilities/*.json`) grants `allow-<command-name>`. A test keeps this list,
/// the invoke handler in `src/lib.rs` and the capabilities in step.
const COMMANDS: &[&str] = &[
    "delete_all_local_data",
    "get_retention_policy",
    "set_retention_policy",
    "get_privacy_toggles",
    "set_privacy_toggles",
    "get_sensor_state",
    "get_current_session",
    "get_my_day",
    "list_app_mappings",
    "set_app_category",
    "reset_app_category",
    "get_context_assessment",
    "get_policy_view",
    "set_silence",
    "set_on_fire",
    "get_policy_debug",
    "get_current_intervention",
    "answer_intervention",
    "dismiss_intervention",
    "debug_show_intervention",
    "get_pause_view",
    "start_pause",
    "end_pause",
    "open_pause",
    "list_interests",
    "add_interest",
    "archive_interest",
    "restore_interest",
    "delete_interest",
    "get_interest_suggestion",
    "get_daily_summary",
    "list_summary_days",
    "save_reflection",
    "get_weekly_review",
    "delete_raw_data",
    "get_data_overview",
    "list_recent_activity_events",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
