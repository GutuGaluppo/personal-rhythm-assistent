//! Milestone 9: capturing is quick and items persist locally; a simple,
//! deterministic suggestion brings forgotten ones back.

use chrono::{DateTime, TimeZone, Utc};
use personal_rhythm_assistant_lib::interest_inbox::{self, MAX_CHARS};
use personal_rhythm_assistant_lib::persistence::repositories::interests;
use personal_rhythm_assistant_lib::persistence::{Database, PersistenceError};

fn t(day: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, day, h, 0, 0).unwrap()
}
fn db() -> Database {
    Database::open_in_memory().unwrap()
}
fn add(db: &Database, text: &str, now: DateTime<Utc>) -> i64 {
    db.with_conn(|c| interest_inbox::add(c, text, now))
        .unwrap()
        .id
}
fn texts(db: &Database, archived: bool) -> Vec<String> {
    db.with_conn(|c| interest_inbox::list(c, archived))
        .unwrap()
        .into_iter()
        .map(|i| i.text)
        .collect()
}
fn suggestion(db: &Database, day: &str) -> Option<String> {
    db.with_conn(|c| interest_inbox::suggestion_for(c, day))
        .unwrap()
        .map(|i| i.text)
}

// ---- add ---------------------------------------------------------------------------

#[test]
fn an_interest_has_the_fields_the_spec_lists() {
    let db = db();
    let i = db
        .with_conn(|c| interest_inbox::add(c, "Learn WebGPU", t(2, 9)))
        .unwrap();
    assert_eq!(i.text, "Learn WebGPU");
    assert_eq!(i.created_at, "2026-03-02T09:00:00.000Z");
    assert_eq!(i.archived_at, None);
    assert!(i.id > 0);
}

#[test]
fn the_spec_examples_can_be_added() {
    let db = db();
    for text in [
        "Learn WebGPU",
        "Restore bicycle frame",
        "Read about granular synthesis",
    ] {
        add(&db, text, t(2, 9));
    }
    assert_eq!(texts(&db, false).len(), 3);
}

#[test]
fn text_is_tidied_but_otherwise_kept_as_written() {
    let db = db();
    let i = db
        .with_conn(|c| interest_inbox::add(c, "  Learn \n\t  WebGPU  ", t(2, 9)))
        .unwrap();
    assert_eq!(i.text, "Learn WebGPU");

    let odd = "Ünïcode ✨ — 'quotes'; DROP TABLE interests;--";
    let j = db
        .with_conn(|c| interest_inbox::add(c, odd, t(2, 10)))
        .unwrap();
    assert_eq!(j.text, odd, "stored literally, never interpreted");
    assert_eq!(db.with_conn(interests::count).unwrap(), 2);
}

#[test]
fn empty_or_blank_text_is_refused() {
    let db = db();
    for text in ["", "   ", "\n\t "] {
        let err = db
            .with_conn(|c| interest_inbox::add(c, text, t(2, 9)))
            .unwrap_err();
        assert!(matches!(err, PersistenceError::Invalid(_)), "{text:?}");
    }
    assert_eq!(db.with_conn(interests::count).unwrap(), 0);
}

#[test]
fn the_length_limit_counts_characters_not_bytes() {
    let db = db();
    let ok = "é".repeat(MAX_CHARS);
    assert!(db
        .with_conn(|c| interest_inbox::add(c, &ok, t(2, 9)))
        .is_ok());
    let too_long = "é".repeat(MAX_CHARS + 1);
    assert!(matches!(
        db.with_conn(|c| interest_inbox::add(c, &too_long, t(2, 9))),
        Err(PersistenceError::Invalid(_))
    ));
}

#[test]
fn adding_the_same_thing_twice_keeps_one_entry() {
    let db = db();
    let first = add(&db, "Learn WebGPU", t(2, 9));
    let again = add(&db, "  learn   webgpu ", t(3, 9));
    assert_eq!(first, again);
    assert_eq!(texts(&db, false), ["Learn WebGPU"]);
}

#[test]
fn an_archived_interest_can_be_added_again_as_a_new_one() {
    let db = db();
    let first = add(&db, "Learn WebGPU", t(2, 9));
    db.with_conn(|c| interest_inbox::archive(c, first, t(2, 10)))
        .unwrap();
    let again = add(&db, "Learn WebGPU", t(3, 9));
    assert_ne!(first, again);
}

// ---- list / archive / delete ---------------------------------------------------------

#[test]
fn the_newest_interest_comes_first() {
    let db = db();
    add(&db, "first", t(2, 9));
    add(&db, "second", t(2, 10));
    add(&db, "third", t(2, 10)); // same instant: the later insert still wins
    assert_eq!(texts(&db, false), ["third", "second", "first"]);
}

#[test]
fn archiving_moves_an_interest_out_of_the_list_and_restoring_brings_it_back() {
    let db = db();
    let id = add(&db, "Learn WebGPU", t(2, 9));
    add(&db, "Restore bicycle frame", t(2, 10));

    assert!(db
        .with_conn(|c| interest_inbox::archive(c, id, t(3, 9)))
        .unwrap());
    assert_eq!(texts(&db, false), ["Restore bicycle frame"]);
    assert_eq!(texts(&db, true), ["Learn WebGPU"]);
    let archived = db.with_conn(|c| interests::get(c, id)).unwrap().unwrap();
    assert_eq!(
        archived.archived_at.as_deref(),
        Some("2026-03-03T09:00:00.000Z")
    );

    assert!(db.with_conn(|c| interest_inbox::restore(c, id)).unwrap());
    assert_eq!(texts(&db, true), Vec::<String>::new());
    assert_eq!(texts(&db, false).len(), 2);
}

#[test]
fn deleting_is_permanent() {
    let db = db();
    let id = add(&db, "Learn WebGPU", t(2, 9));
    assert!(db.with_conn(|c| interest_inbox::delete(c, id)).unwrap());
    assert!(texts(&db, false).is_empty());
    assert!(db.with_conn(|c| interests::get(c, id)).unwrap().is_none());
}

#[test]
fn an_archived_interest_can_be_deleted_too() {
    let db = db();
    let id = add(&db, "Learn WebGPU", t(2, 9));
    db.with_conn(|c| interest_inbox::archive(c, id, t(2, 10)))
        .unwrap();
    assert!(db.with_conn(|c| interest_inbox::delete(c, id)).unwrap());
    assert!(texts(&db, true).is_empty());
}

#[test]
fn acting_on_something_that_does_not_exist_is_reported_not_fatal() {
    let db = db();
    assert!(!db
        .with_conn(|c| interest_inbox::archive(c, 999, t(2, 9)))
        .unwrap());
    assert!(!db.with_conn(|c| interest_inbox::restore(c, 999)).unwrap());
    assert!(!db.with_conn(|c| interest_inbox::delete(c, 999)).unwrap());
}

// ---- persistence ------------------------------------------------------------------------

#[test]
fn interests_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    {
        let db = Database::open(&path).unwrap();
        add(&db, "Learn WebGPU", t(2, 9));
        let id = add(&db, "Restore bicycle frame", t(2, 10));
        db.with_conn(|c| interest_inbox::archive(c, id, t(2, 11)))
            .unwrap();
    }
    let db = Database::open(&path).unwrap();
    assert_eq!(texts(&db, false), ["Learn WebGPU"]);
    assert_eq!(texts(&db, true), ["Restore bicycle frame"]);
}

#[test]
fn deleting_all_local_data_empties_the_inbox() {
    let db = db();
    add(&db, "Learn WebGPU", t(2, 9));
    db.delete_all_local_data().unwrap();
    assert_eq!(db.with_conn(interests::count).unwrap(), 0);
}

// ---- suggestion -------------------------------------------------------------------------

#[test]
fn nothing_to_suggest_from_an_empty_inbox() {
    assert_eq!(suggestion(&db(), "2026-03-05"), None);
}

#[test]
fn the_oldest_never_offered_interest_is_suggested_first() {
    let db = db();
    add(&db, "oldest", t(2, 9));
    add(&db, "newer", t(3, 9));
    assert_eq!(suggestion(&db, "2026-03-05").as_deref(), Some("oldest"));
}

#[test]
fn the_suggestion_is_stable_for_the_whole_day() {
    let db = db();
    add(&db, "a", t(2, 9));
    add(&db, "b", t(3, 9));
    let first = suggestion(&db, "2026-03-05");
    for _ in 0..5 {
        assert_eq!(suggestion(&db, "2026-03-05"), first);
    }
}

#[test]
fn each_new_day_brings_the_one_that_has_waited_longest() {
    let db = db();
    add(&db, "a", t(2, 9));
    add(&db, "b", t(3, 9));
    add(&db, "c", t(4, 9));

    let days = ["2026-03-05", "2026-03-06", "2026-03-07"];
    let picked: Vec<_> = days.iter().map(|d| suggestion(&db, d).unwrap()).collect();
    assert_eq!(
        picked,
        ["a", "b", "c"],
        "nothing repeats until everything has had a turn"
    );

    // Then it starts over with the one offered longest ago.
    assert_eq!(suggestion(&db, "2026-03-08").as_deref(), Some("a"));
    assert_eq!(suggestion(&db, "2026-03-09").as_deref(), Some("b"));
}

#[test]
fn a_forgotten_interest_beats_a_recently_added_one() {
    let db = db();
    add(&db, "old idea", t(2, 9));
    assert_eq!(suggestion(&db, "2026-03-05").as_deref(), Some("old idea"));
    add(&db, "fresh idea", t(5, 12));
    // Tomorrow the never-offered one is due, not the one offered yesterday.
    assert_eq!(suggestion(&db, "2026-03-06").as_deref(), Some("fresh idea"));
}

#[test]
fn archived_interests_are_never_suggested() {
    let db = db();
    let id = add(&db, "done with this", t(2, 9));
    add(&db, "still curious", t(3, 9));
    db.with_conn(|c| interest_inbox::archive(c, id, t(4, 9)))
        .unwrap();
    assert_eq!(
        suggestion(&db, "2026-03-05").as_deref(),
        Some("still curious")
    );
}

#[test]
fn archiving_todays_suggestion_replaces_it_the_same_day() {
    let db = db();
    let a = add(&db, "a", t(2, 9));
    add(&db, "b", t(3, 9));
    assert_eq!(suggestion(&db, "2026-03-05").as_deref(), Some("a"));
    db.with_conn(|c| interest_inbox::archive(c, a, t(5, 9)))
        .unwrap();
    assert_eq!(suggestion(&db, "2026-03-05").as_deref(), Some("b"));
    assert_eq!(
        suggestion(&db, "2026-03-05").as_deref(),
        Some("b"),
        "and that one is then stable"
    );
}

#[test]
fn a_single_interest_is_suggested_every_day() {
    let db = db();
    add(&db, "only one", t(2, 9));
    for day in ["2026-03-05", "2026-03-06", "2026-03-07"] {
        assert_eq!(suggestion(&db, day).as_deref(), Some("only one"));
    }
}

#[test]
fn a_restored_interest_can_be_suggested_again() {
    let db = db();
    let id = add(&db, "back again", t(2, 9));
    db.with_conn(|c| interest_inbox::archive(c, id, t(2, 10)))
        .unwrap();
    assert_eq!(suggestion(&db, "2026-03-05"), None);
    db.with_conn(|c| interest_inbox::restore(c, id)).unwrap();
    assert_eq!(suggestion(&db, "2026-03-05").as_deref(), Some("back again"));
}
