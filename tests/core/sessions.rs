//! Milestone 3 exit criteria: deterministic fixtures produce the expected sessions,
//! long idle closes/splits sessions, and the switching count is stable.

use chrono::{DateTime, Duration, TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::{activity_events, sessions};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::privacy::toggles::{self, PrivacyToggles};
use personal_rhythm_assistant_lib::sensors::event::ActivityEvent;
use personal_rhythm_assistant_lib::sensors::service::SharedSnapshot;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};
use personal_rhythm_assistant_lib::sessions::service::SessionService;
use personal_rhythm_assistant_lib::sessions::sessionizer::{SessionConfig, Sessionizer};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ---- fixtures -------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    #[serde(skip)]
    name: String,
    now: String,
    live_idle_since: Option<String>,
    categories: Option<HashMap<String, Category>>,
    events: Vec<ActivityEvent>,
    expected: Vec<Session>,
}

impl Fixture {
    fn sessionizer(&self) -> Sessionizer {
        match self.categories.clone() {
            Some(map) => Sessionizer::with_classifier(
                SessionConfig::default(),
                Box::new(move |app| map.get(app).copied().unwrap_or(Category::Unknown)),
            ),
            None => Sessionizer::new(SessionConfig::default()),
        }
    }
}

fn parse(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn fixtures() -> Vec<Fixture> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/sessions");
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 9, "fixtures missing");
    paths
        .into_iter()
        .map(|p| {
            let mut f: Fixture = serde_json::from_str(&std::fs::read_to_string(&p).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            f.name = p.file_stem().unwrap().to_string_lossy().into_owned();
            f
        })
        .collect()
}

fn assert_same_sessions(context: &str, actual: &[Session], expected: &[Session]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: session count\n actual: {actual:#?}"
    );
    for (a, e) in actual.iter().zip(expected) {
        assert_eq!(a.id, e.id, "{context}");
        assert_eq!(a.started_at, e.started_at, "{context}: startedAt");
        assert_eq!(a.ended_at, e.ended_at, "{context}: endedAt of {}", a.id);
        assert!(
            (a.active_minutes - e.active_minutes).abs() < 1e-9,
            "{context}: active {} vs {}",
            a.active_minutes,
            e.active_minutes
        );
        assert!(
            (a.idle_minutes - e.idle_minutes).abs() < 1e-9,
            "{context}: idle {} vs {}",
            a.idle_minutes,
            e.idle_minutes
        );
        assert_eq!(
            a.context_switches, e.context_switches,
            "{context}: switches"
        );
        assert_eq!(a.category, e.category, "{context}: category");
        assert_eq!(a.application_ids, e.application_ids, "{context}: apps");
        assert_eq!(a.project_id, e.project_id, "{context}");
    }
}

#[test]
fn fixtures_produce_the_expected_sessions() {
    for f in fixtures() {
        let actual = f.sessionizer().run(
            &f.events,
            parse(&f.now),
            f.live_idle_since.as_deref().map(parse),
        );
        assert_same_sessions(&f.name, &actual, &f.expected);
    }
}

#[test]
fn input_order_does_not_matter() {
    for f in fixtures() {
        let mut reversed = f.events.clone();
        reversed.reverse();
        let s = f.sessionizer();
        let now = parse(&f.now);
        let live = f.live_idle_since.as_deref().map(parse);
        // Reversing breaks ties between equal timestamps (switch before its active_application),
        // which never changes a session, so the outcome must be identical.
        assert_same_sessions(
            &f.name,
            &s.run(&reversed, now, live),
            &s.run(&f.events, now, live),
        );
    }
}

/// Live == batch: replaying the events in the order they would really arrive
/// (an idle event only arrives once the idle period is over) must never produce
/// a closed session that the final computation disagrees with.
#[test]
fn sessions_closed_while_streaming_match_the_final_result() {
    for f in fixtures() {
        let s = f.sessionizer();
        let final_run = s.run(
            &f.events,
            parse(&f.now),
            f.live_idle_since.as_deref().map(parse),
        );

        let arrival = |e: &ActivityEvent| match e {
            ActivityEvent::Idle { timestamp, seconds } => {
                parse(timestamp) + Duration::seconds((*seconds).into())
            }
            other => parse(other.timestamp()),
        };
        let mut arrivals: Vec<&ActivityEvent> = f.events.iter().collect();
        arrivals.sort_by_key(|e| arrival(e));

        for i in 0..arrivals.len() {
            let seen: Vec<ActivityEvent> = arrivals[..=i].iter().map(|e| (*e).clone()).collect();
            for closed in s
                .run(&seen, arrival(arrivals[i]), None)
                .iter()
                .filter(|s| s.ended_at.is_some())
            {
                let same = final_run.iter().find(|s| s.id == closed.id);
                assert_eq!(
                    same,
                    Some(closed),
                    "{}: session closed early differs from final",
                    f.name
                );
            }
        }
    }
}

// ---- rules not worth a fixture ------------------------------------------------

fn active(ts: &str, bundle: &str) -> ActivityEvent {
    ActivityEvent::ActiveApplication {
        timestamp: ts.into(),
        bundle_id: bundle.into(),
        application_name: bundle.into(),
    }
}
fn at(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, h, m, 0).unwrap()
}

#[test]
fn no_events_no_sessions() {
    assert!(Sessionizer::new(SessionConfig::default())
        .run(&[], at(10, 0), None)
        .is_empty());
}

#[test]
fn a_switch_burst_is_counted_exactly_once_each() {
    let mut events = vec![active("2026-03-02T09:00:00.000Z", "a")];
    let apps = ["b", "a", "b", "a", "b"];
    let mut from = "a";
    for (i, to) in apps.iter().enumerate() {
        let ts = format!("2026-03-02T09:0{}:00.000Z", i + 1);
        events.push(ActivityEvent::ApplicationSwitch {
            timestamp: ts.clone(),
            from_bundle_id: from.into(),
            to_bundle_id: (*to).into(),
        });
        events.push(active(&ts, to));
        from = to;
    }
    let s = Sessionizer::new(SessionConfig::default()).run(&events, at(9, 10), None);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].context_switches, 5);
    assert_eq!(s[0].application_ids, ["a", "b"]);
}

// ---- service: persistence ---------------------------------------------------

struct Rig {
    db: Arc<Database>,
    service: SessionService,
    snapshot: SharedSnapshot,
}
fn rig() -> Rig {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let snapshot = SharedSnapshot::default();
    let service = SessionService::new(
        Sessionizer::new(SessionConfig::default()),
        db.clone(),
        snapshot.clone(),
    );
    Rig {
        db,
        service,
        snapshot,
    }
}
fn insert(db: &Database, events: &[ActivityEvent]) {
    db.with_conn(|c| {
        events
            .iter()
            .try_for_each(|e| activity_events::insert(c, e).map(|_| ()))
    })
    .unwrap();
}
fn stored(db: &Database) -> Vec<Session> {
    db.with_conn(|c| {
        sessions::list_started_between(c, "1970-01-01T00:00:00Z", "9999-01-01T00:00:00Z")
    })
    .unwrap()
}

#[test]
fn refresh_persists_sessions_and_returns_the_one_in_progress() {
    let r = rig();
    insert(
        &r.db,
        &[
            active("2026-03-02T09:00:00.000Z", "a"),
            active("2026-03-02T09:05:00.000Z", "a"),
        ],
    );
    let current = r.service.refresh(at(9, 8)).unwrap().expect("open session");
    assert_eq!(current.started_at, "2026-03-02T09:00:00.000Z");
    assert!((current.active_minutes - 8.0).abs() < 1e-9);
    assert_eq!(stored(&r.db), vec![current]);
}

#[test]
fn refresh_is_idempotent() {
    let r = rig();
    insert(
        &r.db,
        &[
            active("2026-03-02T09:00:00.000Z", "a"),
            active("2026-03-02T09:05:00.000Z", "a"),
        ],
    );
    r.service.refresh(at(9, 8)).unwrap();
    let first = stored(&r.db);
    r.service.refresh(at(9, 8)).unwrap();
    assert_eq!(stored(&r.db), first);
}

#[test]
fn an_open_session_keeps_growing_across_refreshes_without_duplicating() {
    let r = rig();
    insert(&r.db, &[active("2026-03-02T09:00:00.000Z", "a")]);
    r.service.refresh(at(9, 3)).unwrap();
    insert(&r.db, &[active("2026-03-02T09:05:00.000Z", "a")]);
    r.service.refresh(at(9, 9)).unwrap();
    let all = stored(&r.db);
    assert_eq!(all.len(), 1);
    assert!((all[0].active_minutes - 9.0).abs() < 1e-9);
}

#[test]
fn live_idle_from_the_sensor_is_not_counted_as_active() {
    let r = rig();
    insert(
        &r.db,
        &[
            active("2026-03-02T09:00:00.000Z", "a"),
            active("2026-03-02T09:05:00.000Z", "a"),
        ],
    );
    r.snapshot.lock().unwrap().idle_since = Some("2026-03-02T09:06:00.000Z".into());
    let current = r.service.refresh(at(9, 10)).unwrap().unwrap();
    assert!((current.idle_minutes - 4.0).abs() < 1e-9);
    assert!((current.active_minutes - 6.0).abs() < 1e-9);
}

#[test]
fn sessions_before_the_latest_stored_one_are_final() {
    let r = rig();
    // A session from before the retained events, with details the events can no longer reproduce.
    let old = Session {
        id: "session-2026-01-01T08:00:00.000Z".into(),
        started_at: "2026-01-01T08:00:00.000Z".into(),
        ended_at: Some("2026-01-01T12:00:00.000Z".into()),
        active_minutes: 240.0,
        idle_minutes: 0.0,
        context_switches: 42,
        category: Category::Create,
        application_ids: vec!["x".into()],
        project_id: None,
    };
    r.db.with_conn(|c| sessions::upsert(c, &old)).unwrap();
    // Events exist only from later on, so recomputation would see a different (truncated) history.
    insert(&r.db, &[active("2026-03-02T09:00:00.000Z", "a")]);
    r.service.refresh(at(9, 3)).unwrap();

    let all = stored(&r.db);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], old, "an older session must never be rewritten");
}

#[test]
fn an_open_session_that_can_no_longer_be_recomputed_is_closed_at_its_last_refresh() {
    let r = rig();
    let stale = Session {
        id: "session-2026-01-01T08:00:00.000Z".into(),
        started_at: "2026-01-01T08:00:00.000Z".into(),
        ended_at: None,
        active_minutes: 50.0,
        idle_minutes: 10.0,
        context_switches: 3,
        category: Category::Unknown,
        application_ids: vec!["x".into()],
        project_id: None,
    };
    r.db.with_conn(|c| sessions::upsert(c, &stale)).unwrap();
    insert(&r.db, &[active("2026-03-02T09:00:00.000Z", "a")]);
    r.service.refresh(at(9, 3)).unwrap();

    let closed =
        r.db.with_conn(|c| sessions::get(c, &stale.id))
            .unwrap()
            .unwrap();
    assert_eq!(closed.ended_at.as_deref(), Some("2026-01-01T09:00:00.000Z"));
    assert!(r
        .db
        .with_conn(sessions::list_open)
        .unwrap()
        .iter()
        .all(|s| s.id != stale.id));
}

#[test]
fn nothing_is_built_while_active_time_is_off() {
    let r = rig();
    insert(&r.db, &[active("2026-03-02T09:00:00.000Z", "a")]);
    r.db.with_conn(|c| {
        toggles::save(
            c,
            PrivacyToggles {
                active_time: false,
                ..PrivacyToggles::default()
            },
        )
    })
    .unwrap();
    assert_eq!(r.service.refresh(at(9, 3)).unwrap(), None);
    assert!(stored(&r.db).is_empty());

    r.db.with_conn(|c| toggles::save(c, PrivacyToggles::default()))
        .unwrap();
    assert!(r.service.refresh(at(9, 3)).unwrap().is_some());
}
