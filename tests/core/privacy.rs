//! Milestone 12: the user can inspect and delete what is stored, and retention
//! is honoured immediately.

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::{
    activity_events, daily_summaries, interests, interventions, pauses, sessions,
};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::privacy::inventory::{overview, recent_events, MAX_RECENT};
use personal_rhythm_assistant_lib::privacy::retention::{self, RetentionPolicy};
use personal_rhythm_assistant_lib::sensors::event::ActivityEvent;
use personal_rhythm_assistant_lib::sessions::classification::Classification;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};

fn at(day: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, day, h, 0, 0).unwrap()
}
fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn tz() -> FixedOffset {
    FixedOffset::west_opt(3 * 3600).unwrap()
}

fn event(t: DateTime<Utc>, bundle: &str) -> ActivityEvent {
    ActivityEvent::ActiveApplication {
        timestamp: iso(t),
        bundle_id: bundle.into(),
        application_name: bundle.into(),
    }
}
fn session(t: DateTime<Utc>, active: f64) -> Session {
    Session {
        id: format!("session-{}", iso(t)),
        started_at: iso(t),
        ended_at: Some(iso(t + Duration::minutes(active as i64))),
        active_minutes: active,
        idle_minutes: 0.0,
        context_switches: 1,
        category: Category::Create,
        application_ids: vec![],
        project_id: None,
    }
}

// ---- overview -----------------------------------------------------------------------

#[test]
fn an_empty_database_reports_nothing_stored() {
    let db = Database::open_in_memory().unwrap();
    let o = db.with_conn(overview).unwrap();
    for stats in [
        &o.activity_events,
        &o.sessions,
        &o.check_ins,
        &o.pauses,
        &o.daily_summaries,
        &o.interests,
    ] {
        assert_eq!(
            (
                stats.count,
                stats.oldest.as_deref(),
                stats.newest.as_deref()
            ),
            (0, None, None)
        );
    }
    assert_eq!(o.app_category_overrides, 0);
    assert!(o.database_bytes > 0, "an empty database still has a size");
}

#[test]
fn the_overview_counts_and_dates_everything_that_is_stored() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        activity_events::insert(c, &event(at(2, 9), "a"))?;
        activity_events::insert(c, &event(at(4, 9), "b"))?;
        activity_events::insert(c, &event(at(3, 9), "c"))?;
        sessions::upsert(c, &session(at(2, 9), 60.0))?;
        interventions::insert(
            c,
            &interventions::NewIntervention {
                id: "i1",
                shown_at: &iso(at(3, 10)),
                is_retry: false,
                active_minutes: 95.0,
                reasons: &[],
            },
        )?;
        pauses::insert(c, "p1", &iso(at(3, 11)), "silence", 300)?;
        daily_summaries::set_reflection(c, "2026-03-03", Some("hello"))?;
        interests::insert(c, "Learn WebGPU", &iso(at(2, 8)))?;
        Classification::default().set(c, "com.example", Some("Example"), Category::Learn)
    })
    .unwrap();

    let o = db.with_conn(overview).unwrap();
    assert_eq!(o.activity_events.count, 3);
    assert_eq!(
        o.activity_events.oldest.as_deref(),
        Some("2026-03-02T09:00:00.000Z")
    );
    assert_eq!(
        o.activity_events.newest.as_deref(),
        Some("2026-03-04T09:00:00.000Z")
    );
    assert_eq!(o.sessions.count, 1);
    assert_eq!(o.check_ins.count, 1);
    assert_eq!(o.pauses.count, 1);
    assert_eq!(o.daily_summaries.count, 1);
    assert_eq!(o.daily_summaries.newest.as_deref(), Some("2026-03-03"));
    assert_eq!(o.interests.count, 1);
    assert_eq!(o.app_category_overrides, 1);
}

#[test]
fn the_overview_contains_counts_and_dates_never_content() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        activity_events::insert(c, &event(at(2, 9), "com.secret.sentinel"))?;
        interests::insert(c, "my private thought", &iso(at(2, 8))).map(|_| ())
    })
    .unwrap();
    let json = serde_json::to_string(&db.with_conn(overview).unwrap()).unwrap();
    assert!(!json.contains("sentinel"));
    assert!(!json.contains("private thought"));
}

// ---- recent events -----------------------------------------------------------------------

#[test]
fn the_newest_events_come_first() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        for (h, b) in [(9, "old"), (11, "newest"), (10, "middle")] {
            activity_events::insert(c, &event(at(2, h), b))?;
        }
        Ok(())
    })
    .unwrap();
    let names: Vec<String> = db
        .with_conn(|c| recent_events(c, 10))
        .unwrap()
        .into_iter()
        .map(|e| match e {
            ActivityEvent::ActiveApplication { bundle_id, .. } => bundle_id,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(names, ["newest", "middle", "old"]);
}

#[test]
fn the_number_of_recent_events_is_bounded() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        for i in 0..(MAX_RECENT as i64 + 25) {
            activity_events::insert(c, &event(at(2, 0) + Duration::seconds(i), "a"))?;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        db.with_conn(|c| recent_events(c, 10_000)).unwrap().len(),
        MAX_RECENT as usize
    );
    assert_eq!(
        db.with_conn(|c| recent_events(c, 0)).unwrap().len(),
        1,
        "at least one"
    );
    assert_eq!(db.with_conn(|c| recent_events(c, 5)).unwrap().len(), 5);
}

#[test]
fn every_kind_of_event_can_be_inspected() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        activity_events::insert(c, &event(at(2, 9), "a"))?;
        activity_events::insert(
            c,
            &ActivityEvent::Idle {
                timestamp: iso(at(2, 10)),
                seconds: 300,
            },
        )?;
        activity_events::insert(
            c,
            &ActivityEvent::ApplicationSwitch {
                timestamp: iso(at(2, 11)),
                from_bundle_id: "a".into(),
                to_bundle_id: "b".into(),
            },
        )
        .map(|_| ())
    })
    .unwrap();
    assert_eq!(db.with_conn(|c| recent_events(c, 10)).unwrap().len(), 3);
}

// ---- deleting raw data ---------------------------------------------------------------------

#[test]
fn deleting_raw_activity_keeps_everything_derived_from_it() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        activity_events::insert(c, &event(at(2, 9), "a"))?;
        sessions::upsert(c, &session(at(2, 9), 60.0))?;
        interests::insert(c, "keep me", &iso(at(2, 8)))?;
        daily_summaries::set_reflection(c, "2026-03-02", Some("keep me too"))?;
        pauses::insert(c, "p1", &iso(at(2, 11)), "silence", 300)
    })
    .unwrap();

    assert_eq!(db.delete_raw_data().unwrap(), 1);

    let o = db.with_conn(overview).unwrap();
    assert_eq!(o.activity_events.count, 0);
    assert_eq!(
        (
            o.sessions.count,
            o.interests.count,
            o.daily_summaries.count,
            o.pauses.count
        ),
        (1, 1, 1, 1)
    );
}

#[test]
fn deleting_raw_activity_restarts_the_id_counter_and_the_app_keeps_working() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| activity_events::insert(c, &event(at(2, 9), "a")).map(|_| ()))
        .unwrap();
    db.delete_raw_data().unwrap();
    assert_eq!(
        db.with_conn(|c| activity_events::insert(c, &event(at(3, 9), "b")))
            .unwrap(),
        1
    );
}

#[test]
fn deleting_raw_activity_when_there_is_none_is_fine() {
    assert_eq!(
        Database::open_in_memory()
            .unwrap()
            .delete_raw_data()
            .unwrap(),
        0
    );
}

#[test]
fn deleted_raw_activity_does_not_linger_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    let db = Database::open(&path).unwrap();
    db.with_conn(|c| {
        activity_events::insert(c, &event(at(2, 9), "com.example.sentinel-7c1e")).map(|_| ())
    })
    .unwrap();
    db.delete_raw_data().unwrap();
    drop(db);
    let bytes = std::fs::read(&path).unwrap();
    let needle = b"sentinel-7c1e";
    assert!(!bytes.windows(needle.len()).any(|w| w == needle));
}

// ---- delete all: memory too ---------------------------------------------------------------

#[test]
fn deleting_everything_also_forgets_category_choices_held_in_memory() {
    let db = Database::open_in_memory().unwrap();
    let classification = db.with_conn(Classification::load).unwrap();
    db.with_conn(|c| classification.set(c, "com.microsoft.VSCode", None, Category::Learn))
        .unwrap();
    assert_eq!(
        classification.category_for("com.microsoft.VSCode"),
        Category::Learn
    );

    db.delete_all_local_data().unwrap();
    // Without a reload the cache would keep applying a choice that no longer exists.
    assert_eq!(
        classification.category_for("com.microsoft.VSCode"),
        Category::Learn,
        "stale until reloaded"
    );
    db.with_conn(|c| classification.reload(c)).unwrap();
    assert_eq!(
        classification.category_for("com.microsoft.VSCode"),
        Category::Create,
        "back to the default"
    );
}

#[test]
fn the_overview_after_deleting_everything_is_empty() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        sessions::upsert(c, &session(at(2, 9), 60.0))?;
        interests::insert(c, "x", &iso(at(2, 8))).map(|_| ())
    })
    .unwrap();
    db.delete_all_local_data().unwrap();
    let o = db.with_conn(overview).unwrap();
    assert_eq!(
        (o.sessions.count, o.interests.count, o.activity_events.count),
        (0, 0, 0)
    );
}

// ---- retention --------------------------------------------------------------------------------

#[test]
fn a_finished_day_is_summarised_before_its_sessions_are_pruned() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| sessions::upsert(c, &session(at(2, 15), 90.0)))
        .unwrap();

    // Long after the 30 day session retention: pruning alone would lose the day.
    let now = at(2, 15) + Duration::days(45);
    let report = db.with_conn(|c| retention::maintain(c, now, tz())).unwrap();

    assert_eq!(report.sessions_deleted, 1);
    let kept = db
        .with_conn(|c| daily_summaries::get(c, "2026-03-02"))
        .unwrap()
        .unwrap();
    let payload = kept
        .payload
        .expect("the summary was written before the pruning");
    assert!(payload.contains("\"activeMinutes\":90"), "{payload}");
}

#[test]
fn a_shorter_policy_removes_what_it_no_longer_covers_straight_away() {
    let db = Database::open_in_memory().unwrap();
    let now = at(10, 12);
    db.with_conn(|c| {
        activity_events::insert(c, &event(now - Duration::days(6), "old"))?;
        activity_events::insert(c, &event(now - Duration::hours(2), "recent"))
    })
    .unwrap();
    // Default is 7 days: nothing to remove.
    assert_eq!(
        db.with_conn(|c| retention::maintain(c, now, tz()))
            .unwrap()
            .activity_events_deleted,
        0
    );

    let report = db
        .with_conn(|c| {
            retention::save(
                c,
                &RetentionPolicy {
                    activity_events_days: 2,
                    ..Default::default()
                },
            )?;
            retention::maintain(c, now, tz())
        })
        .unwrap();
    assert_eq!(report.activity_events_deleted, 1);
    assert_eq!(db.with_conn(activity_events::count).unwrap(), 1);
}

#[test]
fn maintenance_is_repeatable() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| sessions::upsert(c, &session(at(2, 15), 90.0)))
        .unwrap();
    let now = at(2, 15) + Duration::days(45);
    db.with_conn(|c| retention::maintain(c, now, tz())).unwrap();
    let again = db.with_conn(|c| retention::maintain(c, now, tz())).unwrap();
    assert_eq!(again, Default::default(), "nothing left to do");
    assert!(db
        .with_conn(|c| daily_summaries::get(c, "2026-03-02"))
        .unwrap()
        .is_some());
}
