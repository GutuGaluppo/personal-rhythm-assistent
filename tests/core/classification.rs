//! Milestone 4: manual, default-based classification; the user can remap any app.

use chrono::{TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::{activity_events, sessions};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::sensors::event::ActivityEvent;
use personal_rhythm_assistant_lib::sensors::service::SharedSnapshot;
use personal_rhythm_assistant_lib::sessions::classification::{
    default_category, Classification, MappingSource,
};
use personal_rhythm_assistant_lib::sessions::model::Category;
use personal_rhythm_assistant_lib::sessions::service::SessionService;
use personal_rhythm_assistant_lib::sessions::sessionizer::{SessionConfig, Sessionizer};
use std::sync::Arc;

const VSCODE: &str = "com.microsoft.VSCode";
const FIREFOX: &str = "org.mozilla.firefox";

fn active(ts: &str, bundle: &str, name: &str) -> ActivityEvent {
    ActivityEvent::ActiveApplication {
        timestamp: ts.into(),
        bundle_id: bundle.into(),
        application_name: name.into(),
    }
}

#[test]
fn defaults_follow_the_spec_examples() {
    assert_eq!(default_category(VSCODE), Some(Category::Create));
    assert_eq!(
        default_category("com.apple.Terminal"),
        Some(Category::Create)
    );
    assert_eq!(
        default_category("com.figma.Desktop"),
        Some(Category::Create)
    );
    assert_eq!(
        default_category("com.spotify.client"),
        Some(Category::Recover)
    );
    assert_eq!(default_category("com.apple.iBooksX"), Some(Category::Learn));
    assert_eq!(default_category(FIREFOX), None, "browsers are user-defined");
}

#[test]
fn unclassified_apps_are_unknown() {
    let c = Classification::default();
    assert_eq!(c.category_for(FIREFOX), Category::Unknown);
    assert_eq!(c.category_for("some.new.app"), Category::Unknown);
    assert_eq!(c.category_for(VSCODE), Category::Create);
}

#[test]
fn a_user_choice_beats_the_default_and_survives_a_restart() {
    let db = Database::open_in_memory().unwrap();
    let c = Classification::default();
    db.with_conn(|conn| c.set(conn, VSCODE, Some("Code"), Category::Learn))
        .unwrap();
    db.with_conn(|conn| c.set(conn, FIREFOX, Some("Firefox"), Category::Explore))
        .unwrap();
    assert_eq!(c.category_for(VSCODE), Category::Learn);
    assert_eq!(c.category_for(FIREFOX), Category::Explore);

    let reloaded = db.with_conn(Classification::load).unwrap();
    assert_eq!(reloaded.category_for(VSCODE), Category::Learn);
    assert_eq!(reloaded.category_for(FIREFOX), Category::Explore);
}

#[test]
fn resetting_falls_back_to_the_default() {
    let db = Database::open_in_memory().unwrap();
    let c = Classification::default();
    db.with_conn(|conn| c.set(conn, VSCODE, None, Category::Think))
        .unwrap();
    db.with_conn(|conn| c.reset(conn, VSCODE)).unwrap();
    assert_eq!(c.category_for(VSCODE), Category::Create);
    assert_eq!(
        db.with_conn(Classification::load)
            .unwrap()
            .category_for(VSCODE),
        Category::Create
    );
}

#[test]
fn a_user_can_explicitly_mark_a_default_app_as_unknown() {
    let db = Database::open_in_memory().unwrap();
    let c = Classification::default();
    db.with_conn(|conn| c.set(conn, VSCODE, None, Category::Unknown))
        .unwrap();
    assert_eq!(c.category_for(VSCODE), Category::Unknown);
}

#[test]
fn the_mapping_list_shows_seen_apps_and_saved_choices_with_their_source() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|conn| {
        activity_events::insert(conn, &active("2026-03-02T09:00:00Z", VSCODE, "Code"))?;
        activity_events::insert(conn, &active("2026-03-02T09:10:00Z", FIREFOX, "Firefox"))?;
        // Renamed later: the most recent name wins.
        activity_events::insert(
            conn,
            &active("2026-03-02T09:20:00Z", VSCODE, "Visual Studio Code"),
        )?;
        // Not seen recently, but the user classified it and we stored its name.
        Classification::default().set(conn, "com.apple.Notes", Some("Notes"), Category::Think)
    })
    .unwrap();

    let c = db.with_conn(Classification::load).unwrap();
    let list = db.with_conn(|conn| c.list_mappings(conn)).unwrap();
    let summary: Vec<_> = list
        .iter()
        .map(|m| (m.application_name.as_str(), m.category, m.source))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("Firefox", Category::Unknown, MappingSource::Unset),
            ("Notes", Category::Think, MappingSource::User),
            (
                "Visual Studio Code",
                Category::Create,
                MappingSource::Default
            ),
        ]
    );
}

#[test]
fn remapping_an_app_changes_the_category_of_the_session_in_progress() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let classification = Arc::new(db.with_conn(Classification::load).unwrap());
    let classifier = classification.clone();
    let service = SessionService::new(
        Sessionizer::with_classifier(
            SessionConfig::default(),
            Box::new(move |app| classifier.category_for(app)),
        ),
        db.clone(),
        SharedSnapshot::default(),
    );
    db.with_conn(|c| {
        activity_events::insert(c, &active("2026-03-02T09:00:00Z", VSCODE, "Code")).map(|_| ())
    })
    .unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 2, 9, 5, 0).unwrap();

    assert_eq!(
        service.refresh(now).unwrap().unwrap().category,
        Category::Create
    );

    db.with_conn(|c| classification.set(c, VSCODE, Some("Code"), Category::Learn))
        .unwrap();
    let current = service.refresh(now).unwrap().unwrap();
    assert_eq!(current.category, Category::Learn);
    let stored = db
        .with_conn(|c| sessions::get(c, &current.id))
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.category,
        Category::Learn,
        "the new category is persisted too"
    );
}
