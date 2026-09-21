//! Milestone 10: the summary comes from persisted sessions, in the user's local
//! day, and it never scores or judges.
//!
//! Tests run in UTC-3 so that "local day" and "UTC day" genuinely differ.

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use personal_rhythm_assistant_lib::interventions::pause::{
    PauseKind, PausePresenter, PauseService,
};
use personal_rhythm_assistant_lib::persistence::repositories::{daily_summaries, pauses, sessions};
use personal_rhythm_assistant_lib::persistence::{Database, PersistenceError};
use personal_rhythm_assistant_lib::policy::rules::PolicyConfig;
use personal_rhythm_assistant_lib::policy::service::PolicyService;
use personal_rhythm_assistant_lib::privacy::retention::{self, RetentionPolicy};
use personal_rhythm_assistant_lib::reports::daily::{
    self, build, day_bounds, finalize_past_days, list_days, save_reflection, summary_for,
    DailySummary, Inputs, MAX_REFLECTION_CHARS, REFLECTIVE_QUESTION,
};
use personal_rhythm_assistant_lib::sensors::service::SharedSnapshot;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};
use personal_rhythm_assistant_lib::sessions::service::SessionService;
use personal_rhythm_assistant_lib::sessions::sessionizer::{SessionConfig, Sessionizer};
use std::sync::Arc;

fn tz() -> FixedOffset {
    FixedOffset::west_opt(3 * 3600).unwrap()
}
fn utc(day: u32, h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, day, h, m, 0).unwrap()
}
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, day).unwrap()
}
fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn session(
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    active: f64,
    switches: u32,
    category: Category,
) -> Session {
    Session {
        id: format!("session-{}", iso(start)),
        started_at: iso(start),
        ended_at: end.map(iso),
        active_minutes: active,
        idle_minutes: 0.0,
        context_switches: switches,
        category,
        application_ids: vec!["a".into()],
        project_id: None,
    }
}

// ---- the pure summary --------------------------------------------------------------

fn summary_of(
    day: u32,
    sessions: &[Session],
    current: Option<&Session>,
    pauses: u32,
    now: DateTime<Utc>,
) -> DailySummary {
    let (day_start, day_end) = day_bounds(date(day), tz());
    build(Inputs {
        date: date(day),
        day_start,
        day_end,
        now,
        sessions,
        current,
        pauses_taken: pauses,
    })
}

/// A UTC-3 "March 4th" is 03:00Z on the 4th to 03:00Z on the 5th.
fn s4(h_utc: u32, m: u32) -> DateTime<Utc> {
    utc(4, h_utc, m)
}

#[test]
fn a_day_with_nothing_in_it_is_just_empty() {
    let d = summary_of(4, &[], None, 0, utc(4, 20, 0));
    assert_eq!(d.date, "2026-03-04");
    assert_eq!(
        (
            d.active_minutes,
            d.longest_session_minutes,
            d.context_switches,
            d.pauses_taken
        ),
        (0.0, 0.0, 0, 0)
    );
    assert!(d.category_distribution.is_empty());
    assert_eq!(d.reflective_question, "How did today's rhythm feel to you?");
    assert_eq!(d.reflection, None);
    assert!(!d.is_final);
}

#[test]
fn it_reports_active_time_longest_session_switches_and_pauses() {
    let sessions = [
        session(s4(12, 0), Some(s4(13, 30)), 80.0, 12, Category::Create),
        session(s4(15, 0), Some(s4(15, 50)), 45.0, 3, Category::Learn),
        session(s4(17, 0), Some(s4(17, 30)), 30.0, 9, Category::Create),
    ];
    let d = summary_of(4, &sessions, None, 2, utc(4, 23, 0));
    assert_eq!(d.active_minutes, 155.0);
    assert_eq!(d.longest_session_minutes, 80.0);
    assert_eq!(d.context_switches, 24);
    assert_eq!(d.pauses_taken, 2);
}

#[test]
fn the_category_distribution_is_largest_first_and_adds_up() {
    let sessions = [
        session(s4(12, 0), Some(s4(13, 0)), 60.0, 0, Category::Learn),
        session(s4(14, 0), Some(s4(16, 0)), 100.0, 0, Category::Create),
        session(s4(17, 0), Some(s4(17, 40)), 40.0, 0, Category::Recover),
    ];
    let d = summary_of(4, &sessions, None, 0, utc(4, 23, 0));
    let order: Vec<_> = d.category_distribution.iter().map(|c| c.category).collect();
    assert_eq!(
        order,
        [Category::Create, Category::Learn, Category::Recover]
    );
    assert_eq!(d.category_distribution[0].minutes, 100.0);
    assert!((d.category_distribution[0].share - 0.5).abs() < 1e-9);
    let total: f64 = d.category_distribution.iter().map(|c| c.share).sum();
    assert!((total - 1.0).abs() < 1e-9);
}

#[test]
fn categories_with_no_time_are_left_out_and_ties_are_stable() {
    let sessions = [
        session(s4(12, 0), Some(s4(13, 0)), 0.0, 0, Category::Move),
        session(s4(14, 0), Some(s4(15, 0)), 30.0, 0, Category::Think),
        session(s4(16, 0), Some(s4(17, 0)), 30.0, 0, Category::Create),
    ];
    let d = summary_of(4, &sessions, None, 0, utc(4, 23, 0));
    let order: Vec<_> = d.category_distribution.iter().map(|c| c.category).collect();
    assert_eq!(
        order,
        [Category::Create, Category::Think],
        "equal time: alphabetical, and no zero-minute Move"
    );
}

#[test]
fn a_session_across_local_midnight_is_shared_between_the_two_days() {
    // 21:00 local on the 4th (00:00Z on the 5th) to 01:00 local on the 5th (04:00Z): 4 hours, 200 active.
    let overnight = session(utc(5, 0, 0), Some(utc(5, 4, 0)), 200.0, 8, Category::Create);
    // Local midnight is 03:00Z on the 5th, so 3 of the 4 hours belong to the 4th.
    let on_4th = summary_of(4, std::slice::from_ref(&overnight), None, 0, utc(6, 0, 0));
    let on_5th = summary_of(5, std::slice::from_ref(&overnight), None, 0, utc(6, 0, 0));
    assert!((on_4th.active_minutes - 150.0).abs() < 1e-9);
    assert!((on_5th.active_minutes - 50.0).abs() < 1e-9);
    assert!(
        (on_4th.active_minutes + on_5th.active_minutes - 200.0).abs() < 1e-9,
        "nothing lost, nothing doubled"
    );
    assert_eq!(
        (on_4th.context_switches, on_5th.context_switches),
        (8, 0),
        "switches belong to the day it began"
    );
    assert!((on_4th.longest_session_minutes - 150.0).abs() < 1e-9);
}

#[test]
fn sessions_of_other_days_do_not_count() {
    let before = session(
        utc(3, 12, 0),
        Some(utc(3, 14, 0)),
        100.0,
        5,
        Category::Create,
    );
    let after = session(
        utc(6, 12, 0),
        Some(utc(6, 14, 0)),
        100.0,
        5,
        Category::Create,
    );
    let d = summary_of(4, &[before, after], None, 0, utc(7, 0, 0));
    assert_eq!(d.active_minutes, 0.0);
}

#[test]
fn the_session_in_progress_replaces_its_stored_copy() {
    let stale = session(s4(12, 0), None, 10.0, 1, Category::Create);
    let live = session(s4(12, 0), None, 60.0, 4, Category::Create);
    let d = summary_of(4, std::slice::from_ref(&stale), Some(&live), 0, s4(13, 0));
    assert_eq!((d.active_minutes, d.context_switches), (60.0, 4));
}

#[test]
fn an_open_session_ends_at_now_for_a_past_day_too() {
    let open = session(s4(20, 0), None, 60.0, 0, Category::Create);
    // "now" is 01:00 local the next day; only the hours before local midnight count.
    let d = summary_of(4, std::slice::from_ref(&open), None, 0, utc(5, 4, 0)); // 20:00Z..04:00Z = 8h, 7h before 03:00Z
    assert!((d.active_minutes - 60.0 * 7.0 / 8.0).abs() < 1e-9);
}

#[test]
fn there_is_no_score_grade_goal_or_verdict_anywhere() {
    let sessions = [session(
        s4(12, 0),
        Some(s4(20, 0)),
        400.0,
        50,
        Category::Create,
    )];
    let d = summary_of(4, &sessions, None, 0, utc(5, 4, 0));
    let json = serde_json::to_string(&d).unwrap().to_lowercase();
    for banned in [
        "score",
        "grade",
        "rating",
        "goal",
        "target",
        "streak",
        "productiv",
        "efficien",
        "focus time",
        "good",
        "bad",
        "great",
        "poor",
        "well done",
        "too much",
        "too little",
        "should",
    ] {
        assert!(!json.contains(banned), "{banned:?} in {json}");
    }
    assert_eq!(d.reflective_question, REFLECTIVE_QUESTION);
}

// ---- days ----------------------------------------------------------------------------

#[test]
fn a_local_day_is_not_a_utc_day() {
    let (start, end) = day_bounds(date(4), tz());
    assert_eq!(start, utc(4, 3, 0));
    assert_eq!(end, utc(5, 3, 0));
    assert_eq!(
        daily::today(utc(5, 2, 59), tz()),
        date(4),
        "02:59Z is still the 4th at UTC-3"
    );
    assert_eq!(daily::today(utc(5, 3, 0), tz()), date(5));
}

#[test]
fn dates_must_look_like_dates() {
    assert!(daily::parse_date("2026-03-04").is_ok());
    for bad in ["", "yesterday", "2026-13-01", "04/03/2026", "2026-3-4x"] {
        assert!(daily::parse_date(bad).is_err(), "{bad:?}");
    }
}

// ---- from the database -----------------------------------------------------------------

struct Rig {
    db: Arc<Database>,
    service: SessionService,
}
fn rig() -> Rig {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let service = SessionService::new(
        Sessionizer::new(SessionConfig::default()),
        db.clone(),
        SharedSnapshot::default(),
    );
    Rig { db, service }
}
fn store(r: &Rig, s: &Session) {
    r.db.with_conn(|c| sessions::upsert(c, s)).unwrap();
}
fn summary(r: &Rig, now: DateTime<Utc>, day: u32) -> DailySummary {
    summary_for(&r.db, &r.service, now, tz(), date(day)).unwrap()
}
fn stored_payload(r: &Rig, day: &str) -> Option<String> {
    r.db.with_conn(|c| daily_summaries::get(c, day))
        .unwrap()
        .and_then(|s| s.payload)
}

#[test]
fn todays_summary_is_live_and_is_not_stored_until_the_day_is_over() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    let d = summary(&r, s4(20, 0), 4);
    assert_eq!(d.active_minutes, 60.0);
    assert!(!d.is_final);
    assert_eq!(stored_payload(&r, "2026-03-04"), None);
}

#[test]
fn a_finished_day_is_stored_the_first_time_it_is_asked_for() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    let d = summary(&r, utc(6, 12, 0), 4);
    assert!(d.is_final);
    assert!(stored_payload(&r, "2026-03-04").is_some());
}

#[test]
fn a_stored_day_stays_readable_after_its_sessions_are_pruned() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    let before = summary(&r, utc(6, 12, 0), 4);

    // Sessions are kept for 30 days; the summary is kept much longer.
    r.db.with_conn(|c| sessions::delete_started_before(c, "2027-01-01T00:00:00Z"))
        .unwrap();
    let after = summary(&r, utc(6, 12, 0), 4);
    assert_eq!(before, after);
    assert_eq!(after.active_minutes, 60.0);
}

#[test]
fn an_empty_past_day_is_not_written_down() {
    let r = rig();
    let d = summary(&r, utc(6, 12, 0), 4);
    assert_eq!(d.active_minutes, 0.0);
    assert_eq!(stored_payload(&r, "2026-03-04"), None);
}

#[test]
fn a_day_that_has_not_happened_is_refused() {
    let r = rig();
    let err = summary_for(&r.db, &r.service, utc(4, 12, 0), tz(), date(9)).unwrap_err();
    assert!(matches!(err, PersistenceError::Invalid(_)));
}

#[test]
fn pauses_are_counted_in_the_local_day_they_started() {
    let r = rig();
    let insert = |id: &str, at: DateTime<Utc>| {
        r.db.with_conn(|c| pauses::insert(c, id, &iso(at), "silence", 300))
            .unwrap()
    };
    insert("late-3rd", utc(4, 2, 59)); // 23:59 local on the 3rd
    insert("early-4th", utc(4, 3, 1)); // 00:01 local on the 4th
    insert("noon-4th", utc(4, 15, 0));
    insert("late-4th", utc(5, 2, 59)); // 23:59 local on the 4th
    insert("early-5th", utc(5, 3, 0)); // 00:00 local on the 5th
    assert_eq!(summary(&r, utc(7, 12, 0), 4).pauses_taken, 3);
    assert_eq!(summary(&r, utc(7, 12, 0), 3).pauses_taken, 1);
}

// ---- reflection ---------------------------------------------------------------------------

fn reflect(r: &Rig, day: u32, text: &str, now: DateTime<Utc>) -> Result<(), PersistenceError> {
    r.db.with_conn(|c| save_reflection(c, date(day), text, now, tz()))
}

#[test]
fn a_reflection_is_saved_trimmed_and_shown_with_the_summary() {
    let r = rig();
    reflect(&r, 4, "  Steady, mostly.  ", utc(4, 20, 0)).unwrap();
    assert_eq!(
        summary(&r, utc(4, 21, 0), 4).reflection.as_deref(),
        Some("Steady, mostly.")
    );
}

#[test]
fn an_empty_reflection_clears_it() {
    let r = rig();
    reflect(&r, 4, "something", utc(4, 20, 0)).unwrap();
    reflect(&r, 4, "   ", utc(4, 20, 5)).unwrap();
    assert_eq!(summary(&r, utc(4, 21, 0), 4).reflection, None);
}

#[test]
fn a_reflection_is_optional_and_bounded() {
    let r = rig();
    assert_eq!(summary(&r, utc(4, 21, 0), 4).reflection, None);
    let long = "x".repeat(MAX_REFLECTION_CHARS + 1);
    assert!(matches!(
        reflect(&r, 4, &long, utc(4, 20, 0)),
        Err(PersistenceError::Invalid(_))
    ));
    assert!(reflect(&r, 4, &"x".repeat(MAX_REFLECTION_CHARS), utc(4, 20, 0)).is_ok());
}

#[test]
fn a_reflection_cannot_be_written_for_a_day_that_has_not_begun() {
    let r = rig();
    assert!(matches!(
        reflect(&r, 9, "hello", utc(4, 20, 0)),
        Err(PersistenceError::Invalid(_))
    ));
}

#[test]
fn a_reflection_survives_the_day_being_finalized_and_finalizing_survives_a_reflection() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    reflect(&r, 4, "quiet day", utc(4, 20, 0)).unwrap();

    let d = summary(&r, utc(6, 12, 0), 4); // finalizes
    assert!(d.is_final);
    assert_eq!(d.reflection.as_deref(), Some("quiet day"));

    reflect(&r, 4, "changed my mind", utc(6, 13, 0)).unwrap();
    let again = summary(&r, utc(6, 14, 0), 4);
    assert_eq!(again.reflection.as_deref(), Some("changed my mind"));
    assert_eq!(
        again.active_minutes, 60.0,
        "the stored figures are untouched"
    );
}

#[test]
fn a_reflection_on_a_day_with_no_activity_is_kept() {
    let r = rig();
    reflect(&r, 3, "day off", utc(3, 20, 0)).unwrap();
    assert_eq!(
        summary(&r, utc(6, 12, 0), 3).reflection.as_deref(),
        Some("day off")
    );
}

// ---- finalizing and listing ----------------------------------------------------------------

#[test]
fn finished_days_are_stored_in_one_pass_and_today_is_left_alone() {
    let r = rig();
    store(
        &r,
        &session(
            utc(3, 15, 0),
            Some(utc(3, 16, 0)),
            40.0,
            1,
            Category::Create,
        ),
    ); // the 3rd
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Learn),
    ); // the 4th
    store(
        &r,
        &session(
            utc(6, 15, 0),
            Some(utc(6, 16, 0)),
            20.0,
            0,
            Category::Create,
        ),
    ); // the 6th = today

    let stored =
        r.db.with_conn(|c| finalize_past_days(c, utc(6, 18, 0), tz()))
            .unwrap();
    assert_eq!(stored, ["2026-03-03", "2026-03-04"]);
    assert_eq!(stored_payload(&r, "2026-03-06"), None);
}

#[test]
fn finalizing_twice_changes_nothing() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    r.db.with_conn(|c| finalize_past_days(c, utc(6, 18, 0), tz()))
        .unwrap();
    let first = stored_payload(&r, "2026-03-04");
    let second =
        r.db.with_conn(|c| finalize_past_days(c, utc(7, 18, 0), tz()))
            .unwrap();
    assert!(second.is_empty());
    assert_eq!(stored_payload(&r, "2026-03-04"), first);
}

#[test]
fn a_session_across_midnight_finalizes_both_days() {
    let r = rig();
    store(
        &r,
        &session(utc(5, 0, 0), Some(utc(5, 4, 0)), 200.0, 8, Category::Create),
    );
    let stored =
        r.db.with_conn(|c| finalize_past_days(c, utc(8, 12, 0), tz()))
            .unwrap();
    assert_eq!(stored, ["2026-03-04", "2026-03-05"]);
}

#[test]
fn the_list_of_days_is_newest_first_without_duplicates_and_always_has_today() {
    let r = rig();
    store(
        &r,
        &session(
            utc(3, 15, 0),
            Some(utc(3, 16, 0)),
            40.0,
            1,
            Category::Create,
        ),
    );
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    store(
        &r,
        &session(s4(15, 0), Some(s4(16, 0)), 60.0, 2, Category::Create),
    ); // same day again
    r.db.with_conn(|c| finalize_past_days(c, utc(6, 18, 0), tz()))
        .unwrap();

    let days =
        r.db.with_conn(|c| list_days(c, utc(6, 18, 0), tz()))
            .unwrap();
    assert_eq!(days, ["2026-03-06", "2026-03-04", "2026-03-03"]);
}

#[test]
fn a_summary_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    {
        let db = Arc::new(Database::open(&path).unwrap());
        let service = SessionService::new(
            Sessionizer::new(SessionConfig::default()),
            db.clone(),
            SharedSnapshot::default(),
        );
        db.with_conn(|c| {
            sessions::upsert(
                c,
                &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
            )
        })
        .unwrap();
        summary_for(&db, &service, utc(6, 12, 0), tz(), date(4)).unwrap();
        db.with_conn(|c| save_reflection(c, date(4), "kept", utc(6, 12, 0), tz()))
            .unwrap();
    }
    let db = Arc::new(Database::open(&path).unwrap());
    let service = SessionService::new(
        Sessionizer::new(SessionConfig::default()),
        db.clone(),
        SharedSnapshot::default(),
    );
    let d = summary_for(&db, &service, utc(7, 12, 0), tz(), date(4)).unwrap();
    assert_eq!(
        (d.active_minutes, d.reflection.as_deref()),
        (60.0, Some("kept"))
    );
}

// ---- pauses are recorded ---------------------------------------------------------------------

struct Silent;
impl PausePresenter for Silent {
    fn show(&self) {}
    fn close(&self) {}
}
fn pause_service(db: &Arc<Database>) -> PauseService {
    let policy = Arc::new(PolicyService::new(db.clone(), PolicyConfig::default()));
    PauseService::new(policy, db.clone(), Box::new(Silent))
}
fn pause_rows(db: &Database) -> Vec<(String, Option<String>)> {
    db.with_conn(|c| {
        let mut stmt = c.prepare("SELECT kind, ended_at FROM pauses ORDER BY started_at")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
    .unwrap()
}

#[test]
fn every_pause_that_starts_is_recorded_whatever_started_it() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let svc = pause_service(&db);
    svc.start(PauseKind::Meditation, 5, utc(4, 15, 0)).unwrap();
    assert_eq!(pause_rows(&db), [("meditation".to_string(), None)]);

    svc.end(utc(4, 15, 3));
    assert_eq!(
        pause_rows(&db),
        [(
            "meditation".to_string(),
            Some("2026-03-04T15:03:00.000Z".to_string())
        )]
    );
}

#[test]
fn a_setup_screen_that_was_never_started_is_not_a_pause() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let svc = pause_service(&db);
    svc.offer(PauseKind::Silence);
    svc.end(utc(4, 15, 3));
    assert!(pause_rows(&db).is_empty());
}

#[test]
fn a_refused_pause_leaves_no_record() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let svc = pause_service(&db);
    assert!(svc.start(PauseKind::Silence, 0, utc(4, 15, 0)).is_err());
    assert!(pause_rows(&db).is_empty());
}

#[test]
fn closing_the_window_also_records_when_the_pause_ended() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let svc = pause_service(&db);
    svc.start(PauseKind::Walking, 10, utc(4, 15, 0)).unwrap();
    svc.window_closed(utc(4, 15, 4));
    assert_eq!(
        pause_rows(&db)[0].1.as_deref(),
        Some("2026-03-04T15:04:00.000Z")
    );
}

// ---- retention and deletion --------------------------------------------------------------------

#[test]
fn summaries_are_kept_indefinitely_by_default_and_pauses_follow_sessions() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    r.db.with_conn(|c| pauses::insert(c, "p1", &iso(s4(12, 30)), "silence", 300))
        .unwrap();
    summary(&r, utc(6, 12, 0), 4);

    let far = utc(4, 12, 0) + chrono::Duration::days(400);
    let report = r.db.with_conn(|c| retention::apply(c, far)).unwrap();
    assert_eq!(report.daily_summaries_deleted, 0, "default: indefinite");
    assert_eq!(report.pauses_deleted, 1);
    assert!(stored_payload(&r, "2026-03-04").is_some());
}

#[test]
fn a_chosen_limit_removes_older_summaries_only() {
    let r = rig();
    store(
        &r,
        &session(
            utc(2, 15, 0),
            Some(utc(2, 16, 0)),
            30.0,
            0,
            Category::Create,
        ),
    );
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    r.db.with_conn(|c| finalize_past_days(c, utc(6, 12, 0), tz()))
        .unwrap();
    r.db.with_conn(|c| {
        retention::save(
            c,
            &RetentionPolicy {
                daily_summaries_days: Some(30),
                ..Default::default()
            },
        )
    })
    .unwrap();

    // 31 days after the 2nd: the 2nd is out, the 4th is still inside 30 days.
    let report =
        r.db.with_conn(|c| retention::apply(c, utc(2, 12, 0) + chrono::Duration::days(31)))
            .unwrap();
    assert_eq!(report.daily_summaries_deleted, 1);
    assert_eq!(stored_payload(&r, "2026-03-02"), None);
    assert!(stored_payload(&r, "2026-03-04").is_some());
}

#[test]
fn deleting_all_local_data_removes_summaries_reflections_and_pauses() {
    let r = rig();
    store(
        &r,
        &session(s4(12, 0), Some(s4(13, 0)), 60.0, 2, Category::Create),
    );
    r.db.with_conn(|c| pauses::insert(c, "p1", &iso(s4(12, 30)), "silence", 300))
        .unwrap();
    reflect(&r, 4, "private", utc(6, 12, 0)).unwrap();
    summary(&r, utc(6, 12, 0), 4);

    r.db.delete_all_local_data().unwrap();
    assert_eq!(
        r.db.with_conn(|c| daily_summaries::get(c, "2026-03-04"))
            .unwrap(),
        None
    );
    assert!(pause_rows(&r.db).is_empty());
}
