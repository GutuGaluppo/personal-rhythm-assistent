//! Milestone 11: the review works with no AI, and every observation can be
//! explained from stored data.
//!
//! The window is the seven local days ending on Sat 7 March 2026 (UTC-3).

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::interventions::{self, Outcome};
use personal_rhythm_assistant_lib::persistence::repositories::{daily_summaries, sessions};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::reports::daily::{CategoryShare, DailySummary};
use personal_rhythm_assistant_lib::reports::weekly::{
    build, format_duration, review, Inputs, WeeklyConfig, WeeklyReview, REFLECTIVE_QUESTION,
};
use personal_rhythm_assistant_lib::sensors::service::SharedSnapshot;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};
use personal_rhythm_assistant_lib::sessions::service::SessionService;
use personal_rhythm_assistant_lib::sessions::sessionizer::{SessionConfig, Sessionizer};
use std::sync::Arc;

fn tz() -> FixedOffset {
    FixedOffset::west_opt(3 * 3600).unwrap()
}
fn cfg() -> WeeklyConfig {
    WeeklyConfig::default()
}
fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// A day's summary. `d` is the day of March (use 0 or negatives via `feb`).
fn day(
    date: &str,
    active: f64,
    longest: f64,
    switches: u32,
    cats: &[(Category, f64)],
) -> DailySummary {
    let all: f64 = cats.iter().map(|(_, m)| m).sum();
    DailySummary {
        date: date.into(),
        is_final: true,
        active_minutes: active,
        category_distribution: cats
            .iter()
            .map(|(category, minutes)| CategoryShare {
                category: *category,
                minutes: *minutes,
                share: minutes / all,
            })
            .collect(),
        longest_session_minutes: longest,
        context_switches: switches,
        pauses_taken: 0,
        reflective_question: String::new(),
        reflection: None,
    }
}
fn quiet(date: &str) -> DailySummary {
    day(date, 0.0, 0.0, 0, &[])
}

const THIS_WEEK: [&str; 7] = [
    "2026-03-01",
    "2026-03-02",
    "2026-03-03",
    "2026-03-04",
    "2026-03-05",
    "2026-03-06",
    "2026-03-07",
];
const LAST_WEEK: [&str; 7] = [
    "2026-02-22",
    "2026-02-23",
    "2026-02-24",
    "2026-02-25",
    "2026-02-26",
    "2026-02-27",
    "2026-02-28",
];

fn quiet_week(dates: [&str; 7]) -> Vec<DailySummary> {
    dates.iter().map(|d| quiet(d)).collect()
}

/// A session that began at noon local time (15:00Z) on `date` ("2026-03-0N").
fn sess(date: &str, active: f64) -> Session {
    let start = format!("{date}T15:00:00.000Z");
    Session {
        id: format!("s-{date}-{active}"),
        started_at: start,
        ended_at: None,
        active_minutes: active,
        idle_minutes: 0.0,
        context_switches: 0,
        category: Category::Create,
        application_ids: vec![],
        project_id: None,
    }
}

fn outcome(action: Option<&str>) -> Outcome {
    Outcome {
        id: "i".into(),
        shown_at: "2026-03-05T15:00:00.000Z".into(),
        is_retry: false,
        action: action.map(String::from),
    }
}

fn run(
    days: &[DailySummary],
    previous: &[DailySummary],
    sessions: &[Session],
    outcomes: &[Outcome],
) -> WeeklyReview {
    build(
        &cfg(),
        Inputs {
            tz: tz(),
            days,
            previous_days: previous,
            sessions,
            outcomes,
        },
    )
}
fn ids(r: &WeeklyReview) -> Vec<&str> {
    r.observations.iter().map(|o| o.id.as_str()).collect()
}
fn observation<'a>(
    r: &'a WeeklyReview,
    id: &str,
) -> &'a personal_rhythm_assistant_lib::reports::weekly::Observation {
    r.observations
        .iter()
        .find(|o| o.id == id)
        .unwrap_or_else(|| panic!("no {id}: {:?}", ids(r)))
}

// ---- the figures --------------------------------------------------------------------------

#[test]
fn the_window_is_seven_days_oldest_first() {
    let r = run(&quiet_week(THIS_WEEK), &quiet_week(LAST_WEEK), &[], &[]);
    assert_eq!(
        (r.from.as_str(), r.to.as_str()),
        ("2026-03-01", "2026-03-07")
    );
    assert_eq!(r.days.len(), 7);
    assert_eq!(r.days[0].date, "2026-03-01");
    assert_eq!(r.days[6].date, "2026-03-07");
    assert_eq!(
        r.reflective_question,
        "Does anything here feel different from what you would like?"
    );
    assert_eq!(r.reflective_question, REFLECTIVE_QUESTION);
}

#[test]
fn days_carry_their_own_totals_and_sessions() {
    let mut days = quiet_week(THIS_WEEK);
    days[2] = day("2026-03-03", 150.0, 100.0, 30, &[(Category::Create, 150.0)]);
    let sessions = [sess("2026-03-03", 100.0), sess("2026-03-03", 50.0)];
    let r = run(&days, &quiet_week(LAST_WEEK), &sessions, &[]);
    let d = &r.days[2];
    assert_eq!(
        (
            d.active_minutes,
            d.longest_session_minutes,
            d.context_switches
        ),
        (150.0, 100.0, 30)
    );
    assert_eq!(d.sessions, 2);
    assert_eq!(d.average_session_minutes, Some(75.0));
    assert_eq!(
        r.days[0].average_session_minutes, None,
        "no sessions, no average"
    );
    assert_eq!(r.active_minutes, 150.0);
}

#[test]
fn the_category_distribution_adds_the_days_up() {
    let mut days = quiet_week(THIS_WEEK);
    days[0] = day(
        "2026-03-01",
        100.0,
        60.0,
        0,
        &[(Category::Create, 60.0), (Category::Learn, 40.0)],
    );
    days[1] = day(
        "2026-03-02",
        100.0,
        60.0,
        0,
        &[(Category::Create, 50.0), (Category::Recover, 50.0)],
    );
    let r = run(&days, &quiet_week(LAST_WEEK), &[], &[]);
    let got: Vec<_> = r
        .category_distribution
        .iter()
        .map(|c| (c.category, c.minutes))
        .collect();
    assert_eq!(
        got,
        [
            (Category::Create, 110.0),
            (Category::Recover, 50.0),
            (Category::Learn, 40.0)
        ]
    );
    let sum: f64 = r.category_distribution.iter().map(|c| c.share).sum();
    assert!((sum - 1.0).abs() < 1e-9);
}

#[test]
fn session_figures_count_average_and_flag_long_ones() {
    let sessions = [
        sess("2026-03-01", 89.9),
        sess("2026-03-02", 90.0),
        sess("2026-03-02", 120.0),
        sess("2026-03-05", 200.0),
    ];
    let r = run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &sessions,
        &[],
    );
    let s = &r.sessions;
    assert_eq!(s.count, 4);
    assert!((s.average_minutes - (89.9 + 90.0 + 120.0 + 200.0) / 4.0).abs() < 1e-9);
    assert_eq!(s.longest_minutes, 200.0);
    assert_eq!(s.long_sessions, 3, "90 minutes counts, 89.9 does not");
    assert_eq!(s.long_session_days, 2);
}

#[test]
fn a_week_with_no_sessions_has_zeros_not_nan() {
    let r = run(&quiet_week(THIS_WEEK), &quiet_week(LAST_WEEK), &[], &[]);
    assert_eq!(
        (
            r.sessions.count,
            r.sessions.average_minutes,
            r.sessions.longest_minutes
        ),
        (0, 0.0, 0.0)
    );
    assert_eq!(r.switching.per_active_hour, 0.0);
    assert!(r.switching.busiest_day.is_none());
    assert!(r.category_distribution.is_empty());
    // A NaN would serialise as null in a field that is never allowed to be null.
    let json = serde_json::to_value(&r).unwrap();
    assert!(json["sessions"]["averageMinutes"].is_number());
    assert!(json["switching"]["perActiveHour"].is_number());
    assert!(json["activeMinutes"].is_number());
}

#[test]
fn switching_is_a_rate_and_names_the_busiest_qualifying_day() {
    let mut days = quiet_week(THIS_WEEK);
    days[0] = day("2026-03-01", 120.0, 60.0, 60, &[(Category::Create, 120.0)]); // 30 / h
    days[1] = day("2026-03-02", 60.0, 60.0, 90, &[(Category::Create, 60.0)]); // 90 / h
    days[2] = day("2026-03-03", 30.0, 30.0, 300, &[(Category::Create, 30.0)]); // 600 / h, too short a day to rank
    let r = run(&days, &quiet_week(LAST_WEEK), &[], &[]);
    assert_eq!(r.switching.total, 450);
    assert!((r.switching.per_active_hour - 450.0 / 3.5).abs() < 1e-9);
    let busiest = r.switching.busiest_day.unwrap();
    assert_eq!(
        (busiest.date.as_str(), busiest.per_active_hour),
        ("2026-03-02", 90.0)
    );
}

#[test]
fn check_ins_are_sorted_into_exactly_one_bucket_each() {
    let outcomes = [
        outcome(Some("take_break")),
        outcome(Some("meditate")),
        outcome(Some("move")),
        outcome(Some("do_nothing")),
        outcome(Some("continue")),
        outcome(Some("leave_me_alone")),
        outcome(Some("ignored")),
        outcome(None),
        outcome(Some("on_fire")),
    ];
    let c = run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &[],
        &outcomes,
    )
    .check_ins;
    assert_eq!(
        (c.shown, c.accepted, c.declined, c.ignored, c.on_fire),
        (9, 4, 2, 2, 1)
    );
    assert_eq!(
        c.accepted + c.declined + c.ignored + c.on_fire,
        c.shown,
        "nothing lost, nothing double counted"
    );
}

// ---- observations --------------------------------------------------------------------------

#[test]
fn the_spec_examples_come_out_word_for_word() {
    let mut outcomes: Vec<_> = (0..4).map(|_| outcome(Some("take_break"))).collect();
    outcomes.extend((0..3).map(|_| outcome(Some("continue"))));
    let r = run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &[sess("2026-03-04", 194.0)],
        &outcomes,
    );
    assert_eq!(
        observation(&r, "longest_session").text,
        "Your longest session was 3h 14m."
    );
    assert_eq!(
        observation(&r, "check_ins").text,
        "You accepted 4 of 7 break suggestions."
    );
}

#[test]
fn the_largest_category_is_named_with_its_numbers() {
    let mut days = quiet_week(THIS_WEEK);
    days[0] = day(
        "2026-03-01",
        300.0,
        100.0,
        0,
        &[(Category::Create, 210.0), (Category::Learn, 90.0)],
    );
    let o = run(&days, &quiet_week(LAST_WEEK), &[], &[]);
    let o = observation(&o, "top_category");
    assert_eq!(o.text, "Create took the largest share of your active time.");
    assert_eq!(o.evidence, "Create: 3h 30m of 5h active this week (70%).");
}

fn weeks_with_shares(this: (f64, f64), last: (f64, f64)) -> (Vec<DailySummary>, Vec<DailySummary>) {
    let mut a = quiet_week(THIS_WEEK);
    a[0] = day(
        "2026-03-01",
        this.0 + this.1,
        60.0,
        0,
        &[(Category::Create, this.0), (Category::Learn, this.1)],
    );
    let mut b = quiet_week(LAST_WEEK);
    b[0] = day(
        "2026-02-22",
        last.0 + last.1,
        60.0,
        0,
        &[(Category::Create, last.0), (Category::Learn, last.1)],
    );
    (a, b)
}

#[test]
fn a_shift_of_at_least_ten_points_against_last_week_is_reported() {
    let (a, b) = weeks_with_shares((70.0 * 3.0, 30.0 * 3.0), (60.0 * 3.0, 40.0 * 3.0)); // 70% vs 60%
    let r = run(&a, &b, &[], &[]);
    let o = observation(&r, "category_change");
    assert_eq!(
        o.evidence,
        "Create: 70% of active time this week, 60% the week before."
    );
    assert!(
        o.text == "Create occupied more space this week."
            || o.text == "Learn occupied less space this week."
    );
}

#[test]
fn a_shift_is_worded_as_more_or_less_without_praise_or_blame() {
    // Two categories trading places: the one that grew is named.
    let (a, b) = weeks_with_shares((150.0, 150.0), (60.0, 240.0)); // Create 50% vs 20%
    assert_eq!(
        observation(&run(&a, &b, &[], &[]), "category_change").text,
        "Create occupied more space this week."
    );

    // Three categories: Create falls 20 points while the others each gain 10.
    let mut this = quiet_week(THIS_WEEK);
    this[0] = day(
        "2026-03-01",
        300.0,
        60.0,
        0,
        &[
            (Category::Create, 120.0),
            (Category::Learn, 90.0),
            (Category::Recover, 90.0),
        ],
    );
    let mut last = quiet_week(LAST_WEEK);
    last[0] = day(
        "2026-02-22",
        300.0,
        60.0,
        0,
        &[
            (Category::Create, 180.0),
            (Category::Learn, 60.0),
            (Category::Recover, 60.0),
        ],
    );
    let o = run(&this, &last, &[], &[]);
    let o = observation(&o, "category_change");
    assert_eq!(o.text, "Create occupied less space this week.");
    assert_eq!(
        o.evidence,
        "Create: 40% of active time this week, 60% the week before."
    );
}

#[test]
fn exactly_ten_points_counts_whichever_way_the_arithmetic_rounds() {
    // 0.7 - 0.6 and 0.3 - 0.4 differ in the last bit; both are a ten-point shift.
    for (this, last) in [
        ((210.0, 90.0), (180.0, 120.0)),
        ((90.0, 210.0), (120.0, 180.0)),
    ] {
        let (a, b) = weeks_with_shares(this, last);
        assert!(
            ids(&run(&a, &b, &[], &[])).contains(&"category_change"),
            "{this:?} vs {last:?}"
        );
    }
}

#[test]
fn a_shift_under_ten_points_is_not_reported() {
    let (a, b) = weeks_with_shares((59.0 * 3.0, 41.0 * 3.0), (50.0 * 3.0, 50.0 * 3.0)); // 59% vs 50%
    assert!(!ids(&run(&a, &b, &[], &[])).contains(&"category_change"));
}

#[test]
fn a_comparison_needs_enough_activity_in_both_weeks() {
    let (a, b) = weeks_with_shares((100.0, 0.0), (0.0, 100.0)); // total flip, but only 100 min each
    assert!(!ids(&run(&a, &b, &[], &[])).contains(&"category_change"));
    let (a, mut b) = weeks_with_shares((300.0, 0.0), (0.0, 300.0));
    b.clear(); // no previous week at all
    assert!(!ids(&run(&a, &b, &[], &[])).contains(&"category_change"));
}

#[test]
fn long_sessions_are_counted_with_the_days_they_fell_on() {
    let r = run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &[
            sess("2026-03-02", 100.0),
            sess("2026-03-02", 95.0),
            sess("2026-03-05", 120.0),
        ],
        &[],
    );
    let o = observation(&r, "long_sessions");
    assert_eq!(o.text, "3 sessions lasted 1h 30m or more, across 2 days.");
    let r = run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &[sess("2026-03-02", 100.0)],
        &[],
    );
    assert_eq!(
        observation(&r, "long_sessions").text,
        "1 session lasted 1h 30m or more, across 1 day."
    );
    let r = run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &[sess("2026-03-02", 60.0)],
        &[],
    );
    assert!(!ids(&r).contains(&"long_sessions"));
}

#[test]
fn switching_is_only_stated_with_enough_activity() {
    let mut days = quiet_week(THIS_WEEK);
    days[0] = day("2026-03-01", 119.0, 60.0, 60, &[(Category::Create, 119.0)]);
    assert!(!ids(&run(&days, &quiet_week(LAST_WEEK), &[], &[])).contains(&"switching"));

    days[0] = day("2026-03-01", 120.0, 60.0, 60, &[(Category::Create, 120.0)]);
    let r = run(&days, &quiet_week(LAST_WEEK), &[], &[]);
    let o = observation(&r, "switching");
    assert_eq!(
        o.text,
        "You switched apps about 30 times per active hour. The busiest day was Sunday (about 30)."
    );
    assert_eq!(o.evidence, "60 switches in 2h of active time.");
}

fn trend(early: &[f64], late: &[f64], middle: &[f64]) -> WeeklyReview {
    let mut sessions = vec![];
    for (i, m) in early.iter().enumerate() {
        sessions.push(sess(["2026-03-01", "2026-03-02", "2026-03-03"][i % 3], *m));
    }
    for (i, m) in late.iter().enumerate() {
        sessions.push(sess(["2026-03-05", "2026-03-06", "2026-03-07"][i % 3], *m));
    }
    for m in middle {
        sessions.push(sess("2026-03-04", *m));
    }
    run(
        &quiet_week(THIS_WEEK),
        &quiet_week(LAST_WEEK),
        &sessions,
        &[],
    )
}

#[test]
fn a_clear_difference_in_session_length_is_reported_either_way() {
    let r = trend(&[30.0, 30.0, 30.0], &[60.0, 60.0, 60.0], &[]);
    let o = observation(&r, "session_length_trend");
    assert_eq!(
        o.text,
        "Your sessions were longer in the later part of the week."
    );
    assert_eq!(o.evidence, "Average 30m across 3 sessions in the first three days, 1h across 3 sessions in the last three.");

    let r = trend(&[60.0, 60.0, 60.0], &[30.0, 30.0, 30.0], &[]);
    assert_eq!(
        observation(&r, "session_length_trend").text,
        "Your sessions were longer in the earlier part of the week."
    );
}

#[test]
fn a_small_or_thinly_supported_difference_in_session_length_is_not_reported() {
    // 30 -> 35: 5 minutes and 17%: under both bars
    assert!(!ids(&trend(&[30.0; 3], &[35.0; 3], &[])).contains(&"session_length_trend"));
    // 100 -> 110: 10 minutes but only 10%
    assert!(!ids(&trend(&[100.0; 3], &[110.0; 3], &[])).contains(&"session_length_trend"));
    // 20 -> 28: 40% but only 8 minutes
    assert!(!ids(&trend(&[20.0; 3], &[28.0; 3], &[])).contains(&"session_length_trend"));
    // big difference, but two sessions in a half is too few
    assert!(!ids(&trend(&[30.0, 30.0], &[90.0, 90.0, 90.0], &[])).contains(&"session_length_trend"));
}

#[test]
fn the_middle_day_belongs_to_neither_half() {
    // Without the middle day this is flat (40 vs 40); a huge session on day four must not tilt it.
    let r = trend(&[40.0; 3], &[40.0; 3], &[500.0]);
    assert!(!ids(&r).contains(&"session_length_trend"));
}

#[test]
fn a_week_without_check_ins_says_so() {
    let r = run(&quiet_week(THIS_WEEK), &quiet_week(LAST_WEEK), &[], &[]);
    assert_eq!(
        observation(&r, "check_ins").text,
        "There were no check-ins this week."
    );
}

#[test]
fn a_completely_empty_week_has_one_quiet_observation() {
    let r = run(&quiet_week(THIS_WEEK), &quiet_week(LAST_WEEK), &[], &[]);
    assert_eq!(ids(&r), ["check_ins"]);
}

#[test]
fn observations_come_in_a_fixed_order() {
    let mut days = quiet_week(THIS_WEEK);
    days[0] = day(
        "2026-03-01",
        300.0,
        200.0,
        300,
        &[(Category::Create, 300.0)],
    );
    let mut last = quiet_week(LAST_WEEK);
    last[0] = day("2026-02-22", 300.0, 60.0, 0, &[(Category::Learn, 300.0)]);
    let mut sessions = vec![
        sess("2026-03-01", 200.0),
        sess("2026-03-01", 40.0),
        sess("2026-03-02", 30.0),
        sess("2026-03-03", 30.0),
    ];
    sessions.extend([
        sess("2026-03-05", 100.0),
        sess("2026-03-06", 100.0),
        sess("2026-03-07", 100.0),
    ]);
    let r = run(&days, &last, &sessions, &[outcome(Some("take_break"))]);
    assert_eq!(
        ids(&r),
        [
            "top_category",
            "category_change",
            "longest_session",
            "long_sessions",
            "switching",
            "session_length_trend",
            "check_ins"
        ]
    );
}

// ---- explainable and never judging -------------------------------------------------------------

fn rich_week() -> WeeklyReview {
    let mut days = quiet_week(THIS_WEEK);
    for (i, d) in THIS_WEEK.iter().enumerate() {
        days[i] = day(
            d,
            240.0,
            130.0,
            200,
            &[(Category::Create, 180.0), (Category::Learn, 60.0)],
        );
    }
    let mut last = quiet_week(LAST_WEEK);
    last[0] = day("2026-02-22", 300.0, 60.0, 0, &[(Category::Learn, 300.0)]);
    let sessions: Vec<_> = THIS_WEEK
        .iter()
        .flat_map(|d| [sess(d, 130.0), sess(d, 40.0)])
        .collect();
    run(
        &days,
        &last,
        &sessions,
        &[
            outcome(Some("take_break")),
            outcome(Some("continue")),
            outcome(Some("ignored")),
        ],
    )
}

#[test]
fn every_observation_shows_the_numbers_it_came_from() {
    for r in [
        rich_week(),
        run(
            &quiet_week(THIS_WEEK),
            &quiet_week(LAST_WEEK),
            &[],
            &[outcome(None)],
        ),
    ] {
        for o in &r.observations {
            assert!(!o.evidence.trim().is_empty(), "{} has no evidence", o.id);
            assert!(
                o.evidence.chars().any(|c| c.is_ascii_digit()),
                "{}: {}",
                o.id,
                o.evidence
            );
        }
    }
}

const FORBIDDEN: &[&str] = &[
    "burnout",
    "anxi",
    "depress",
    "addict",
    "lazy",
    "disciplin",
    "overwork",
    "stress",
    "exhaust",
    "tired",
    "unhealthy",
    "productiv",
    "score",
    "goal",
    "target",
    "streak",
    "should",
    "must",
    "need to",
    "try to",
    "too much",
    "too little",
    "too long",
    "good",
    "bad",
    "great",
    "poor",
    "well done",
    "better",
    "worse",
    "improve",
    "warning",
    "concern",
];

#[test]
fn nothing_in_the_review_judges_advises_or_diagnoses() {
    let empty = run(&quiet_week(THIS_WEEK), &quiet_week(LAST_WEEK), &[], &[]);
    for r in [rich_week(), empty] {
        let text = serde_json::to_string(&r).unwrap().to_lowercase();
        for word in FORBIDDEN {
            assert!(!text.contains(word), "{word:?} in {text}");
        }
    }
}

#[test]
fn it_is_deterministic() {
    assert_eq!(rich_week(), rich_week());
}

#[test]
fn durations_read_the_way_the_rest_of_the_app_does() {
    for (m, s) in [
        (0.0, "0m"),
        (45.0, "45m"),
        (60.0, "1h"),
        (194.0, "3h 14m"),
        (90.4, "1h 30m"),
        (-3.0, "0m"),
    ] {
        assert_eq!(format_duration(m), s);
    }
}

// ---- from the database -----------------------------------------------------------------------------

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
fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 7, 18, 0, 0).unwrap() // 15:00 on Saturday 7th, local
}
fn store(r: &Rig, date: &str, hour_utc: u32, active: f64, category: Category, switches: u32) {
    let start = chrono::NaiveDateTime::parse_from_str(
        &format!("{date} {hour_utc}:00:00"),
        "%Y-%m-%d %H:%M:%S",
    )
    .unwrap()
    .and_utc();
    let end = start + chrono::Duration::minutes(active as i64);
    let s = Session {
        id: format!("session-{}", iso(start)),
        started_at: iso(start),
        ended_at: Some(iso(end)),
        active_minutes: active,
        idle_minutes: 0.0,
        context_switches: switches,
        category,
        application_ids: vec![],
        project_id: None,
    };
    r.db.with_conn(|c| sessions::upsert(c, &s)).unwrap();
}
fn check_in(r: &Rig, id: &str, at: &str, actions: &[(&str, &str)]) {
    r.db.with_conn(|c| {
        interventions::insert(
            c,
            &interventions::NewIntervention {
                id,
                shown_at: at,
                is_retry: false,
                active_minutes: 95.0,
                reasons: &[],
            },
        )?;
        for (kind, value) in actions {
            interventions::add_feedback(c, id, at, kind, value)?;
        }
        Ok(())
    })
    .unwrap();
}
fn weekly(r: &Rig) -> WeeklyReview {
    review(&r.db, &r.service, &cfg(), now(), tz()).unwrap()
}

#[test]
fn an_empty_database_gives_a_calm_empty_week() {
    let r = weekly(&rig());
    assert_eq!(
        (r.from.as_str(), r.to.as_str()),
        ("2026-03-01", "2026-03-07")
    );
    assert_eq!(r.days.len(), 7);
    assert_eq!(r.active_minutes, 0.0);
    assert_eq!(ids(&r), ["check_ins"]);
}

#[test]
fn the_review_is_built_from_stored_sessions_and_check_ins() {
    let r = rig();
    store(&r, "2026-03-03", 15, 100.0, Category::Create, 40);
    store(&r, "2026-03-05", 15, 60.0, Category::Learn, 10);
    check_in(
        &r,
        "c1",
        "2026-03-04T15:00:00.000Z",
        &[("energy", "good"), ("action", "take_break")],
    );
    check_in(
        &r,
        "c2",
        "2026-03-05T15:00:00.000Z",
        &[("action", "ignored")],
    );

    let w = weekly(&r);
    assert_eq!(w.active_minutes, 160.0);
    assert_eq!(w.sessions.count, 2);
    assert_eq!(w.sessions.longest_minutes, 100.0);
    assert_eq!(w.switching.total, 50);
    assert_eq!(
        (w.check_ins.shown, w.check_ins.accepted, w.check_ins.ignored),
        (2, 1, 1)
    );
    assert_eq!(
        observation(&w, "check_ins").text,
        "You accepted 1 of 2 break suggestions."
    );
    assert_eq!(
        observation(&w, "longest_session").text,
        "Your longest session was 1h 40m."
    );
}

#[test]
fn a_check_ins_last_recorded_action_is_the_one_that_counts() {
    let r = rig();
    check_in(
        &r,
        "c1",
        "2026-03-04T15:00:00.000Z",
        &[("energy", "low"), ("action", "meditate")],
    );
    let outcomes =
        r.db.with_conn(|c| {
            interventions::outcomes_between(c, "2026-03-04T00:00:00Z", "2026-03-05T00:00:00Z")
        })
        .unwrap();
    assert_eq!(
        outcomes[0].action.as_deref(),
        Some("meditate"),
        "the energy answer is not the ending"
    );
}

#[test]
fn things_outside_the_window_are_left_out() {
    let r = rig();
    store(&r, "2026-02-20", 15, 300.0, Category::Create, 99); // before the window
    check_in(
        &r,
        "old",
        "2026-02-27T15:00:00.000Z",
        &[("action", "take_break")],
    );
    check_in(
        &r,
        "future",
        "2026-03-08T15:00:00.000Z",
        &[("action", "take_break")],
    );
    let w = weekly(&r);
    assert_eq!(w.sessions.count, 0);
    assert_eq!(w.check_ins.shown, 0);
    assert_eq!(w.active_minutes, 0.0);
}

#[test]
fn the_week_before_is_compared_when_both_weeks_have_enough() {
    let r = rig();
    for d in ["2026-02-23", "2026-02-24", "2026-02-25"] {
        store(&r, d, 15, 100.0, Category::Create, 0); // last week: all Create (300 min)
    }
    for d in ["2026-03-02", "2026-03-03", "2026-03-04"] {
        store(&r, d, 15, 100.0, Category::Learn, 0); // this week: all Learn (300 min)
    }
    let w = weekly(&r);
    let change = observation(&w, "category_change");
    assert!(change.text.ends_with("space this week."), "{}", change.text);
    assert!(
        change.evidence.contains("100%") && change.evidence.contains("0%"),
        "{}",
        change.evidence
    );
}

#[test]
fn asking_for_the_review_stores_the_finished_days_it_used() {
    let r = rig();
    store(&r, "2026-03-03", 15, 100.0, Category::Create, 5);
    weekly(&r);
    let stored =
        r.db.with_conn(|c| daily_summaries::get(c, "2026-03-03"))
            .unwrap();
    assert!(stored.is_some_and(|s| s.payload.is_some()));
}

#[test]
fn it_survives_the_sessions_being_pruned_for_finished_days() {
    let r = rig();
    store(&r, "2026-03-03", 15, 100.0, Category::Create, 5);
    let before = weekly(&r);
    r.db.with_conn(|c| sessions::delete_started_before(c, "2026-03-05T00:00:00Z"))
        .unwrap();
    let after = weekly(&r);
    // The day figures come from the stored snapshot; only session-level detail is gone.
    assert_eq!(before.active_minutes, after.active_minutes);
    assert_eq!(before.category_distribution, after.category_distribution);
}
