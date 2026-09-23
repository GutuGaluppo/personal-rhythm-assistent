//! Milestone 4: My Day is built from real local sessions and shows facts only.

use chrono::{DateTime, Local, TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::activity_events;
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::privacy::toggles::{self, PrivacyToggles};
use personal_rhythm_assistant_lib::reports::my_day::{self, build, Inputs, MyDay};
use personal_rhythm_assistant_lib::sensors::event::ActivityEvent;
use personal_rhythm_assistant_lib::sensors::service::SharedSnapshot;
use personal_rhythm_assistant_lib::sensors::system_state::ActivityState;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};
use personal_rhythm_assistant_lib::sessions::service::SessionService;
use personal_rhythm_assistant_lib::sessions::sessionizer::{SessionConfig, Sessionizer};
use std::sync::Arc;

fn at(day: u32, h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, day, h, m, 0).unwrap()
}
fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn session(
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    active: f64,
    idle: f64,
    switches: u32,
) -> Session {
    Session {
        id: format!("session-{}", iso(start)),
        started_at: iso(start),
        ended_at: end.map(iso),
        active_minutes: active,
        idle_minutes: idle,
        context_switches: switches,
        category: Category::Create,
        application_ids: vec!["a".into()],
        project_id: None,
    }
}

fn inputs<'a>(
    now: DateTime<Utc>,
    sessions: &'a [Session],
    current: Option<&'a Session>,
) -> Inputs<'a> {
    Inputs {
        now,
        day_start: at(2, 0, 0),
        sessions,
        current,
        last_break: None,
        live_idle_since: None,
        break_gap_secs: 900,
        state: ActivityState::Active,
        frontmost_application: Some("Code".into()),
    }
}

fn empty_day(now: DateTime<Utc>) -> MyDay {
    build(inputs(now, &[], None))
}

#[test]
fn an_empty_day_is_just_empty() {
    let d = empty_day(at(2, 9, 0));
    assert!(d.tracking_enabled);
    assert!(d.current_session.is_none());
    assert_eq!((d.active_minutes_today, d.context_switches_today), (0.0, 0));
    assert!(d.last_break.is_none());
}

#[test]
fn totals_add_up_todays_sessions_and_the_one_in_progress() {
    let done = session(at(2, 8, 0), Some(at(2, 9, 0)), 55.0, 5.0, 4);
    let current = session(at(2, 9, 30), None, 28.0, 2.0, 1);
    let d = build(inputs(
        at(2, 10, 0),
        std::slice::from_ref(&done),
        Some(&current),
    ));

    assert_eq!(d.active_minutes_today, 83.0);
    assert_eq!(d.context_switches_today, 5);
    let cur = d.current_session.unwrap();
    assert_eq!(cur.elapsed_minutes, 30.0);
    assert_eq!(cur.active_minutes, 28.0);
    assert_eq!(cur.category, Category::Create);
    assert_eq!(cur.context_switches, 1);
}

#[test]
fn the_stored_copy_of_the_open_session_is_not_counted_twice() {
    let stored_open = session(at(2, 9, 30), None, 10.0, 0.0, 1); // stale copy
    let live = session(at(2, 9, 30), None, 28.0, 2.0, 3);
    let d = build(inputs(
        at(2, 10, 0),
        std::slice::from_ref(&stored_open),
        Some(&live),
    ));
    assert_eq!(d.active_minutes_today, 28.0);
    assert_eq!(d.context_switches_today, 3);
}

#[test]
fn yesterday_does_not_count() {
    let yesterday = session(at(1, 9, 0), Some(at(1, 12, 0)), 170.0, 10.0, 9);
    let d = build(inputs(at(2, 9, 0), std::slice::from_ref(&yesterday), None));
    assert_eq!((d.active_minutes_today, d.context_switches_today), (0.0, 0));
}

#[test]
fn a_session_across_midnight_counts_only_the_part_inside_today() {
    // 23:00 -> 01:00 (120 min, 100 active): half of it is today.
    let overnight = session(at(1, 23, 0), Some(at(2, 1, 0)), 100.0, 20.0, 6);
    let d = build(inputs(at(2, 9, 0), std::slice::from_ref(&overnight), None));
    assert!((d.active_minutes_today - 50.0).abs() < 1e-9);
    assert_eq!(
        d.context_switches_today, 0,
        "switches belong to the day the session started"
    );
}

#[test]
fn a_finished_break_reports_its_length_and_how_long_ago_it_ended() {
    let mut i = inputs(at(2, 10, 0), &[], None);
    i.last_break = Some((at(2, 9, 0), 1800)); // 09:00 - 09:30
    let b = build(i).last_break.unwrap();
    assert_eq!(b.minutes, 30.0);
    assert_eq!(b.ended_minutes_ago, Some(30.0));
}

#[test]
fn a_break_in_progress_has_no_end_yet() {
    let mut i = inputs(at(2, 10, 0), &[], None);
    i.live_idle_since = Some(at(2, 9, 30));
    i.last_break = Some((at(2, 8, 0), 1200));
    let b = build(i).last_break.unwrap();
    assert_eq!(b.minutes, 30.0);
    assert_eq!(b.ended_minutes_ago, None);
}

#[test]
fn a_short_idle_in_progress_is_not_a_break() {
    let mut i = inputs(at(2, 10, 0), &[], None);
    i.live_idle_since = Some(at(2, 9, 55)); // 5 min < 15
    i.last_break = Some((at(2, 8, 0), 1200));
    assert_eq!(build(i).last_break.unwrap().ended_minutes_ago, Some(100.0));
}

#[test]
fn my_day_shows_no_score_field() {
    let json = serde_json::to_value(empty_day(at(2, 9, 0))).unwrap();
    let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
    for k in &keys {
        for banned in ["score", "streak", "goal", "rating", "grade"] {
            assert!(!k.to_lowercase().contains(banned), "unexpected field {k}");
        }
    }
}

#[test]
fn local_day_start_is_local_midnight_within_the_last_day() {
    let now = Utc::now();
    let start = my_day::local_day_start(now);
    assert!(start <= now && now - start <= chrono::Duration::hours(25));
    assert_eq!(start.with_timezone(&Local).time(), chrono::NaiveTime::MIN);
}

// ---- from the database ----------------------------------------------------------

fn active_event(ts: DateTime<Utc>, bundle: &str) -> ActivityEvent {
    ActivityEvent::ActiveApplication {
        timestamp: iso(ts),
        bundle_id: bundle.into(),
        application_name: bundle.into(),
    }
}

#[test]
fn load_builds_my_day_from_stored_events() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let snapshot = SharedSnapshot::default();
    let service = SessionService::new(
        Sessionizer::new(SessionConfig::default()),
        db.clone(),
        snapshot.clone(),
    );
    db.with_conn(|c| {
        activity_events::insert(c, &active_event(at(2, 9, 0), "a"))?;
        activity_events::insert(c, &active_event(at(2, 9, 5), "a"))?;
        // A 20 minute break starting at 09:07, then back at 09:27.
        activity_events::insert(
            c,
            &ActivityEvent::Idle {
                timestamp: iso(at(2, 9, 7)),
                seconds: 1200,
            },
        )?;
        activity_events::insert(c, &active_event(at(2, 9, 32), "a"))?;
        Ok(())
    })
    .unwrap();

    let d = my_day::load(&db, &service, &snapshot, at(2, 9, 40), at(2, 0, 0), 900).unwrap();

    assert!(d.tracking_enabled);
    // 09:00-09:07 (7 min) ended by the break; the new session starts at 09:27 -> 13 min so far.
    assert!(
        (d.active_minutes_today - 20.0).abs() < 1e-9,
        "got {}",
        d.active_minutes_today
    );
    let cur = d.current_session.unwrap();
    assert_eq!(cur.started_at, iso(at(2, 9, 27)));
    assert_eq!(cur.elapsed_minutes, 13.0);
    let b = d.last_break.unwrap();
    assert_eq!((b.minutes, b.ended_minutes_ago), (20.0, Some(13.0)));
}

#[test]
fn load_reports_that_tracking_is_off_and_shows_nothing() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let snapshot = SharedSnapshot::default();
    let service = SessionService::new(
        Sessionizer::new(SessionConfig::default()),
        db.clone(),
        snapshot.clone(),
    );
    db.with_conn(|c| {
        activity_events::insert(c, &active_event(at(2, 9, 0), "a")).map(|_| ())?;
        toggles::save(
            c,
            PrivacyToggles {
                active_time: false,
                ..PrivacyToggles::default()
            },
        )
        .map(|_| ())
    })
    .unwrap();

    let d = my_day::load(&db, &service, &snapshot, at(2, 9, 10), at(2, 0, 0), 900).unwrap();
    assert!(!d.tracking_enabled);
    assert!(d.current_session.is_none());
    assert_eq!(d.active_minutes_today, 0.0);
}
