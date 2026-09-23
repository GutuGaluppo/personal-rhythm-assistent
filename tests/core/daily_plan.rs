//! Post-v0.1: Today's plan, a free-text checklist scoped to a local day.

use chrono::{DateTime, TimeZone, Utc};
use personal_rhythm_assistant_lib::daily_plan::{self, MAX_CHARS};
use personal_rhythm_assistant_lib::persistence::{Database, PersistenceError};

fn t(day: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, day, h, 0, 0).unwrap()
}
fn db() -> Database {
    Database::open_in_memory().unwrap()
}
fn add(db: &Database, day: &str, text: &str, now: DateTime<Utc>) -> i64 {
    db.with_conn(|c| daily_plan::add(c, day, text, now))
        .unwrap()
        .id
}
fn texts(db: &Database, day: &str) -> Vec<String> {
    db.with_conn(|c| daily_plan::list(c, day))
        .unwrap()
        .into_iter()
        .map(|i| i.text)
        .collect()
}

// ---- add ---------------------------------------------------------------------------

#[test]
fn an_item_has_the_fields_the_spec_lists() {
    let db = db();
    let item = db
        .with_conn(|c| daily_plan::add(c, "2026-03-02", "Ship the release notes", t(2, 9)))
        .unwrap();
    assert_eq!(item.day, "2026-03-02");
    assert_eq!(item.text, "Ship the release notes");
    assert_eq!(item.created_at, "2026-03-02T09:00:00.000Z");
    assert_eq!(item.done_at, None);
    assert!(item.id > 0);
}

#[test]
fn surrounding_and_inner_whitespace_is_tidied() {
    let db = db();
    let item = db
        .with_conn(|c| daily_plan::add(c, "2026-03-02", "  write   the   report  ", t(2, 9)))
        .unwrap();
    assert_eq!(item.text, "write the report");
}

#[test]
fn empty_or_blank_text_is_refused() {
    let db = db();
    for text in ["", "   "] {
        assert!(matches!(
            db.with_conn(|c| daily_plan::add(c, "2026-03-02", text, t(2, 9))),
            Err(PersistenceError::Invalid(_))
        ));
    }
    assert!(texts(&db, "2026-03-02").is_empty());
}

#[test]
fn text_longer_than_the_limit_is_refused() {
    let db = db();
    let long = "a".repeat(MAX_CHARS + 1);
    assert!(matches!(
        db.with_conn(|c| daily_plan::add(c, "2026-03-02", &long, t(2, 9))),
        Err(PersistenceError::Invalid(_))
    ));
}

#[test]
fn text_at_the_limit_is_accepted() {
    let db = db();
    let exact = "a".repeat(MAX_CHARS);
    assert!(db
        .with_conn(|c| daily_plan::add(c, "2026-03-02", &exact, t(2, 9)))
        .is_ok());
}

// ---- list ---------------------------------------------------------------------------

#[test]
fn items_are_scoped_to_their_day_oldest_first() {
    let db = db();
    add(&db, "2026-03-02", "first", t(2, 8));
    add(&db, "2026-03-02", "second", t(2, 9));
    add(&db, "2026-03-03", "a different day", t(3, 8));

    assert_eq!(texts(&db, "2026-03-02"), vec!["first", "second"]);
    assert_eq!(texts(&db, "2026-03-03"), vec!["a different day"]);
}

#[test]
fn a_day_with_nothing_planned_is_just_empty() {
    let db = db();
    assert!(texts(&db, "2026-03-02").is_empty());
}

// ---- toggle and delete ----------------------------------------------------------------

#[test]
fn toggling_marks_done_then_undone() {
    let db = db();
    let id = add(&db, "2026-03-02", "write tests", t(2, 9));

    let done = db
        .with_conn(|c| daily_plan::toggle(c, id, t(2, 10)))
        .unwrap()
        .unwrap();
    assert_eq!(done.done_at.as_deref(), Some("2026-03-02T10:00:00.000Z"));

    let undone = db
        .with_conn(|c| daily_plan::toggle(c, id, t(2, 11)))
        .unwrap()
        .unwrap();
    assert_eq!(undone.done_at, None);
}

#[test]
fn toggling_something_that_does_not_exist_reports_nothing() {
    let db = db();
    assert_eq!(
        db.with_conn(|c| daily_plan::toggle(c, 999, t(2, 9)))
            .unwrap(),
        None
    );
}

#[test]
fn deleting_removes_the_item() {
    let db = db();
    let id = add(&db, "2026-03-02", "write tests", t(2, 9));
    assert!(db.with_conn(|c| daily_plan::delete(c, id)).unwrap());
    assert!(texts(&db, "2026-03-02").is_empty());
    assert!(
        !db.with_conn(|c| daily_plan::delete(c, id)).unwrap(),
        "already gone"
    );
}

// ---- persistence ----------------------------------------------------------------------

#[test]
fn items_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    {
        let db = Database::open(&path).unwrap();
        add(&db, "2026-03-02", "keep me", t(2, 9));
    }
    let reopened = Database::open(&path).unwrap();
    assert_eq!(texts(&reopened, "2026-03-02"), vec!["keep me"]);
}

#[test]
fn deleting_all_local_data_empties_the_plan() {
    let db = db();
    add(&db, "2026-03-02", "keep me", t(2, 9));
    db.delete_all_local_data().unwrap();
    assert!(texts(&db, "2026-03-02").is_empty());
}
