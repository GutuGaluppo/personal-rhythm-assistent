//! Milestone 1 exit criteria: migrations run automatically, data can be
//! inserted/read/deleted, and delete-all is covered.

use chrono::{TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::{
    activity_events, sessions, settings,
};
use personal_rhythm_assistant_lib::persistence::{migrations, Database, PersistenceError};
use personal_rhythm_assistant_lib::privacy::retention::{self, RetentionPolicy};
use personal_rhythm_assistant_lib::sensors::event::ActivityEvent;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};
use rusqlite::Connection;

fn active(ts: &str, bundle: &str) -> ActivityEvent {
    ActivityEvent::ActiveApplication {
        timestamp: ts.into(),
        bundle_id: bundle.into(),
        application_name: "App".into(),
    }
}

fn session(id: &str, started_at: &str) -> Session {
    Session {
        id: id.into(),
        started_at: started_at.into(),
        ended_at: None,
        active_minutes: 12.5,
        idle_minutes: 1.5,
        context_switches: 3,
        category: Category::Create,
        application_ids: vec!["com.microsoft.VSCode".into(), "com.apple.Terminal".into()],
        project_id: None,
    }
}

fn table_counts(db: &Database) -> Vec<(String, i64)> {
    db.with_conn(|conn| {
        let names: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            )?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(names
            .into_iter()
            .map(|n| {
                let c = conn
                    .query_row(&format!("SELECT COUNT(*) FROM \"{n}\""), [], |r| r.get(0))
                    .unwrap();
                (n, c)
            })
            .collect())
    })
    .unwrap()
}

// ---- migrations -----------------------------------------------------------

#[test]
fn migrations_run_automatically_on_open() {
    let db = Database::open_in_memory().unwrap();
    let (version, tables) = db
        .with_conn(|conn| {
            let v: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
            let t: Vec<String> = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?
                .query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            Ok((v, t))
        })
        .unwrap();
    assert_eq!(version, migrations::latest_version());
    for expected in ["activity_events", "sessions", "settings"] {
        assert!(
            tables.iter().any(|t| t == expected),
            "missing table {expected}"
        );
    }
}

#[test]
fn reopening_a_file_database_keeps_data_and_does_not_remigrate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("rhythm.sqlite");

    let db = Database::open(&path).unwrap();
    db.with_conn(|c| settings::set_json(c, "k", &"v")).unwrap();
    drop(db);

    let db = Database::open(&path).unwrap();
    let v: Option<String> = db.with_conn(|c| settings::get_json(c, "k")).unwrap();
    assert_eq!(v.as_deref(), Some("v"));
}

#[test]
fn refuses_a_database_from_a_newer_app_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.pragma_update(None, "user_version", migrations::latest_version() + 1)
        .unwrap();
    drop(conn);

    match Database::open(&path) {
        Err(PersistenceError::SchemaTooNew { .. }) => {}
        other => panic!("expected SchemaTooNew, got {:?}", other.err()),
    }
}

// ---- settings -------------------------------------------------------------

#[test]
fn settings_round_trip_overwrite_and_delete() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        assert_eq!(settings::get_json::<bool>(c, "silent")?, None);
        settings::set_json(c, "silent", &true)?;
        assert_eq!(settings::get_json::<bool>(c, "silent")?, Some(true));
        settings::set_json(c, "silent", &false)?;
        assert_eq!(settings::get_json::<bool>(c, "silent")?, Some(false));
        assert!(settings::delete(c, "silent")?);
        assert_eq!(settings::get_json::<bool>(c, "silent")?, None);
        Ok(())
    })
    .unwrap();
}

// ---- activity events ------------------------------------------------------

#[test]
fn every_event_kind_round_trips() {
    let db = Database::open_in_memory().unwrap();
    let events = vec![
        active("2026-03-02T09:00:00.000Z", "com.microsoft.VSCode"),
        ActivityEvent::ApplicationSwitch {
            timestamp: "2026-03-02T09:30:00.000Z".into(),
            from_bundle_id: "com.microsoft.VSCode".into(),
            to_bundle_id: "com.apple.Terminal".into(),
        },
        ActivityEvent::Idle {
            timestamp: "2026-03-02T09:45:00.000Z".into(),
            seconds: 300,
        },
    ];
    db.with_conn(|c| {
        for e in &events {
            activity_events::insert(c, e)?;
        }
        let read = activity_events::list_range(c, "2026-03-02T00:00:00Z", "2026-03-03T00:00:00Z")?;
        assert_eq!(read, events);
        assert_eq!(activity_events::count(c)?, 3);
        Ok(())
    })
    .unwrap();
}

#[test]
fn event_timestamps_are_normalized_to_utc_so_ordering_is_chronological() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        // 10:00+02:00 == 08:00Z, which is *earlier* than 09:00Z despite the larger clock digits.
        activity_events::insert(c, &active("2026-03-02T09:00:00Z", "b.late"))?;
        activity_events::insert(c, &active("2026-03-02T10:00:00+02:00", "b.early"))?;
        let read = activity_events::list_range(c, "2026-03-02T00:00:00Z", "2026-03-03T00:00:00Z")?;
        let bundles: Vec<_> = read
            .iter()
            .map(|e| match e {
                ActivityEvent::ActiveApplication { bundle_id, .. } => bundle_id.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(bundles, ["b.early", "b.late"]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn list_range_is_half_open_and_delete_before_is_strict() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        for h in [8, 9, 10] {
            activity_events::insert(c, &active(&format!("2026-03-02T{h:02}:00:00Z"), "b"))?;
        }
        let r = activity_events::list_range(c, "2026-03-02T09:00:00Z", "2026-03-02T10:00:00Z")?;
        assert_eq!(r.len(), 1);
        assert_eq!(
            activity_events::delete_before(c, "2026-03-02T09:00:00Z")?,
            1
        );
        assert_eq!(activity_events::count(c)?, 2);
        assert_eq!(activity_events::delete_all(c)?, 2);
        Ok(())
    })
    .unwrap();
}

#[test]
fn invalid_timestamps_are_rejected() {
    let db = Database::open_in_memory().unwrap();
    let err = db
        .with_conn(|c| activity_events::insert(c, &active("yesterday-ish", "b")))
        .unwrap_err();
    assert!(matches!(err, PersistenceError::Invalid(_)));
}

// ---- sessions -------------------------------------------------------------

#[test]
fn sessions_upsert_read_update_and_delete() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        let mut s = session("s1", "2026-03-02T09:00:00Z");
        sessions::upsert(c, &s)?;

        let stored = sessions::get(c, "s1")?.unwrap();
        assert_eq!(stored.started_at, "2026-03-02T09:00:00.000Z");
        assert_eq!(stored.application_ids, s.application_ids);
        assert_eq!(stored.category, Category::Create);

        // The Sessionizer updates the open session in place.
        s.ended_at = Some("2026-03-02T10:30:00Z".into());
        s.active_minutes = 88.0;
        s.category = Category::Learn;
        sessions::upsert(c, &s)?;
        assert_eq!(sessions::count(c)?, 1);
        let updated = sessions::get(c, "s1")?.unwrap();
        assert_eq!(
            updated.ended_at.as_deref(),
            Some("2026-03-02T10:30:00.000Z")
        );
        assert_eq!(updated.active_minutes, 88.0);
        assert_eq!(updated.category, Category::Learn);

        assert!(sessions::get(c, "missing")?.is_none());
        sessions::upsert(c, &session("s2", "2026-03-03T09:00:00Z"))?;
        let day1 =
            sessions::list_started_between(c, "2026-03-02T00:00:00Z", "2026-03-03T00:00:00Z")?;
        assert_eq!(day1.len(), 1);
        assert_eq!(
            sessions::delete_started_before(c, "2026-03-03T00:00:00Z")?,
            1
        );
        assert_eq!(sessions::count(c)?, 1);
        Ok(())
    })
    .unwrap();
}

#[test]
fn every_category_can_be_stored() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        for (i, cat) in Category::ALL.into_iter().enumerate() {
            let mut s = session(&format!("s{i}"), "2026-03-02T09:00:00Z");
            s.category = cat;
            sessions::upsert(c, &s)?;
            assert_eq!(sessions::get(c, &s.id)?.unwrap().category, cat);
        }
        Ok(())
    })
    .unwrap();
}

// ---- retention ------------------------------------------------------------

#[test]
fn retention_defaults_match_the_spec_and_can_be_changed() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        let d = retention::load(c)?;
        assert_eq!(
            (
                d.activity_events_days,
                d.sessions_days,
                d.daily_summaries_days
            ),
            (7, 30, None)
        );

        let p = RetentionPolicy {
            activity_events_days: 3,
            sessions_days: 14,
            daily_summaries_days: Some(365),
        };
        retention::save(c, &p)?;
        assert_eq!(retention::load(c)?, p);

        let zero = RetentionPolicy {
            activity_events_days: 0,
            ..p
        };
        assert!(matches!(
            retention::save(c, &zero),
            Err(PersistenceError::Invalid(_))
        ));
        assert_eq!(
            retention::load(c)?,
            p,
            "a rejected policy must not overwrite the stored one"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn applying_retention_deletes_only_expired_data() {
    let db = Database::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap();
    db.with_conn(|c| {
        // Defaults: events 7 days, sessions 30 days.
        activity_events::insert(c, &active("2026-03-20T12:00:00Z", "old"))?; // 11 days
        activity_events::insert(c, &active("2026-03-28T12:00:00Z", "recent"))?; // 3 days
        sessions::upsert(c, &session("old", "2026-02-20T12:00:00Z"))?; // 39 days
        sessions::upsert(c, &session("mid", "2026-03-20T12:00:00Z"))?; // 11 days: past events window, inside sessions window

        let report = retention::apply(c, now)?;
        assert_eq!(report.activity_events_deleted, 1);
        assert_eq!(report.sessions_deleted, 1);
        assert_eq!(activity_events::count(c)?, 1);
        assert!(sessions::get(c, "old")?.is_none());
        assert!(sessions::get(c, "mid")?.is_some());
        Ok(())
    })
    .unwrap();
}

// ---- delete all local data ------------------------------------------------

#[test]
fn delete_all_empties_every_table_and_the_app_keeps_working() {
    let db = Database::open_in_memory().unwrap();
    db.with_conn(|c| {
        settings::set_json(c, "k", &1)?;
        retention::save(
            c,
            &RetentionPolicy {
                activity_events_days: 2,
                ..Default::default()
            },
        )?;
        activity_events::insert(c, &active("2026-03-02T09:00:00Z", "b"))?;
        sessions::upsert(c, &session("s1", "2026-03-02T09:00:00Z"))
    })
    .unwrap();
    assert!(table_counts(&db).iter().any(|(_, n)| *n > 0));

    db.delete_all_local_data().unwrap();

    let counts = table_counts(&db);
    assert!(!counts.is_empty(), "schema must be kept");
    assert!(
        counts.iter().all(|(_, n)| *n == 0),
        "tables not emptied: {counts:?}"
    );

    // Defaults are back, AUTOINCREMENT restarts, and writes still work.
    db.with_conn(|c| {
        assert_eq!(retention::load(c)?, RetentionPolicy::default());
        assert_eq!(
            activity_events::insert(c, &active("2026-03-03T09:00:00Z", "b"))?,
            1
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn deleted_content_does_not_linger_in_the_database_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    let db = Database::open(&path).unwrap();
    db.with_conn(|c| {
        activity_events::insert(
            c,
            &active("2026-03-02T09:00:00Z", "com.example.sentinel-9f3a"),
        )
    })
    .unwrap();
    db.delete_all_local_data().unwrap();
    drop(db);

    let mut bytes = std::fs::read(&path).unwrap();
    for sidecar in ["rhythm.sqlite-wal", "rhythm.sqlite-journal"] {
        if let Ok(extra) = std::fs::read(dir.path().join(sidecar)) {
            bytes.extend(extra);
        }
    }
    let needle = b"sentinel-9f3a";
    assert!(!bytes.windows(needle.len()).any(|w| w == needle));
}
