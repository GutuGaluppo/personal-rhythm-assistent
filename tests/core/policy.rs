//! Milestone 6 exit criteria: Continue -> 60 min cooldown, Leave me alone -> 60 min,
//! max 4/day, "I'm on fire" suppresses, and silence overrides everything.

use chrono::{DateTime, Duration, TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::policy::engine::{PolicyState, Response};
use personal_rhythm_assistant_lib::policy::rules::{
    PolicyConfig, PolicyDecision, SilentReason, HARD_DAILY_CEILING,
};
use personal_rhythm_assistant_lib::policy::service::PolicyService;
use std::sync::Arc;

fn t(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 4, h, m, 0).unwrap()
}
fn day_start() -> DateTime<Utc> {
    t(0, 0)
}
fn cfg() -> PolicyConfig {
    PolicyConfig::default()
}
fn mins(m: i64) -> Duration {
    Duration::minutes(m)
}

/// Decision with a strong candidate (2 signals) unless stated otherwise.
fn ask(s: &PolicyState, now: DateTime<Utc>) -> PolicyDecision {
    s.decide(&cfg(), now, day_start(), 2)
}
fn is_silent(d: &PolicyDecision) -> bool {
    matches!(d, PolicyDecision::Silent { .. })
}
const ASK: PolicyDecision = PolicyDecision::AskCheckin { is_retry: false };

#[test]
fn a_fresh_state_lets_a_real_candidate_through() {
    assert_eq!(ask(&PolicyState::default(), t(10, 0)), ASK);
}

#[test]
fn not_enough_signals_means_observe_never_ask() {
    let s = PolicyState::default();
    assert_eq!(
        s.decide(&cfg(), t(10, 0), day_start(), 1),
        PolicyDecision::Observe
    );
    assert_eq!(
        s.decide(&cfg(), t(10, 0), day_start(), 0),
        PolicyDecision::Observe
    );
}

// ---- cooldowns ------------------------------------------------------------------

fn assert_cooldown_of_60(response: Response) {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), response);
    assert!(is_silent(&ask(&s, t(10, 1))));
    assert_eq!(
        ask(&s, t(10, 59)),
        PolicyDecision::Silent {
            reason: SilentReason::Cooldown { until: t(11, 0) }
        }
    );
    assert_eq!(ask(&s, t(11, 0)), ASK, "the cooldown is exactly 60 minutes");
}

#[test]
fn continue_starts_a_60_minute_cooldown() {
    assert_cooldown_of_60(Response::Continue);
}

#[test]
fn leave_me_alone_starts_a_60_minute_cooldown() {
    assert_cooldown_of_60(Response::LeaveMeAlone);
}

#[test]
fn an_accepted_break_is_not_followed_up_immediately() {
    assert_cooldown_of_60(Response::AcceptedBreak);
}

#[test]
fn do_nothing_also_cools_down() {
    assert_cooldown_of_60(Response::DoNothing);
}

#[test]
fn a_shorter_cooldown_never_shortens_a_longer_one() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), Response::Continue); // until 11:00
    s.respond(&cfg(), t(10, 5), Response::Ignored { is_retry: false }); // retry due 10:50
    assert!(
        is_silent(&ask(&s, t(10, 55))),
        "still cooling down until 11:00"
    );
}

// ---- daily ceiling ---------------------------------------------------------------

#[test]
fn at_most_four_check_ins_per_day() {
    let mut s = PolicyState::default();
    for h in [8, 10, 12] {
        s.record_shown(t(h, 0));
        assert_eq!(ask(&s, t(h, 30)), ASK, "{} shown so far", h);
    }
    s.record_shown(t(14, 0));
    assert_eq!(
        ask(&s, t(16, 0)),
        PolicyDecision::Silent {
            reason: SilentReason::DailyCeiling { limit: 4 }
        }
    );
}

#[test]
fn the_ceiling_resets_with_the_new_day() {
    let mut s = PolicyState::default();
    for h in [8, 10, 12, 14] {
        s.record_shown(t(h, 0));
    }
    assert!(is_silent(&ask(&s, t(16, 0))));
    let tomorrow = day_start() + Duration::days(1);
    assert_eq!(s.decide(&cfg(), tomorrow + mins(600), tomorrow, 2), ASK);
}

#[test]
fn the_user_cannot_raise_the_ceiling_above_four() {
    let mut s = PolicyState {
        max_per_day: 10,
        ..Default::default()
    };
    assert_eq!(s.effective_ceiling(), HARD_DAILY_CEILING);
    for h in [8, 10, 12, 14] {
        s.record_shown(t(h, 0));
    }
    assert!(is_silent(&ask(&s, t(16, 0))));
}

#[test]
fn old_history_is_pruned() {
    let mut s = PolicyState::default();
    s.record_shown(t(10, 0) - Duration::days(30));
    s.record_shown(t(10, 0));
    assert_eq!(s.shown.len(), 1);
}

// ---- I'm on fire ------------------------------------------------------------------

#[test]
fn on_fire_suppresses_for_120_minutes() {
    let mut s = PolicyState::default();
    s.activate_on_fire(&cfg(), t(10, 0));
    assert_eq!(
        ask(&s, t(11, 59)),
        PolicyDecision::Silent {
            reason: SilentReason::OnFire { until: t(12, 0) }
        }
    );
    assert_eq!(ask(&s, t(12, 0)), ASK);
}

#[test]
fn on_fire_can_be_chosen_from_a_check_in() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), Response::OnFire);
    assert!(is_silent(&ask(&s, t(11, 0))));
    assert!(s.on_fire_active(t(11, 0)));
}

#[test]
fn on_fire_can_be_switched_off_early() {
    let mut s = PolicyState::default();
    s.activate_on_fire(&cfg(), t(10, 0));
    s.clear_on_fire();
    assert_eq!(ask(&s, t(10, 5)), ASK);
}

#[test]
fn on_fire_is_not_a_refusal() {
    let mut s = PolicyState::default();
    for _ in 0..5 {
        s.respond(&cfg(), t(10, 0), Response::OnFire);
    }
    assert!(!s.frequency_prompt_due(&cfg()));
}

// ---- silence ---------------------------------------------------------------------

#[test]
fn silence_overrides_everything() {
    let mut s = PolicyState::default();
    s.set_silent(true);
    assert_eq!(
        s.decide(&cfg(), t(10, 0), day_start(), 4),
        PolicyDecision::Silent {
            reason: SilentReason::SilentMode
        }
    );
    // ...even when a cooldown, the ceiling and on-fire are all active too.
    s.respond(&cfg(), t(10, 0), Response::Continue);
    s.activate_on_fire(&cfg(), t(10, 0));
    for h in [6, 7, 8, 9] {
        s.record_shown(t(h, 0));
    }
    assert_eq!(
        s.decide(&cfg(), t(10, 30), day_start(), 4),
        PolicyDecision::Silent {
            reason: SilentReason::SilentMode
        }
    );
}

#[test]
fn turning_silence_off_restores_normal_behaviour() {
    let mut s = PolicyState::default();
    s.set_silent(true);
    s.set_silent(false);
    assert_eq!(ask(&s, t(10, 0)), ASK);
}

#[test]
fn reasons_are_reported_in_priority_order() {
    let mut s = PolicyState::default();
    s.activate_on_fire(&cfg(), t(10, 0));
    for h in [6, 7, 8, 9] {
        s.record_shown(t(h, 0));
    }
    // ceiling beats on-fire
    assert!(matches!(
        ask(&s, t(10, 5)),
        PolicyDecision::Silent {
            reason: SilentReason::DailyCeiling { .. }
        }
    ));
    // cooldown beats ceiling
    s.respond(&cfg(), t(10, 5), Response::Continue);
    assert!(matches!(
        ask(&s, t(10, 10)),
        PolicyDecision::Silent {
            reason: SilentReason::Cooldown { .. }
        }
    ));
}

#[test]
fn suppression_wins_over_missing_evidence() {
    // Same order as the spec: a silenced app says "silent", not "observe".
    let mut s = PolicyState::default();
    s.set_silent(true);
    assert!(is_silent(&s.decide(&cfg(), t(10, 0), day_start(), 0)));
}

// ---- retry ------------------------------------------------------------------------

#[test]
fn an_ignored_check_in_is_retried_once_after_45_minutes() {
    let mut s = PolicyState::default();
    s.record_shown(t(10, 0));
    s.respond(&cfg(), t(10, 0), Response::Ignored { is_retry: false });

    assert!(
        is_silent(&ask(&s, t(10, 44))),
        "nothing before the retry is due"
    );
    assert_eq!(
        ask(&s, t(10, 45)),
        PolicyDecision::AskCheckin { is_retry: true }
    );
}

#[test]
fn the_retry_delay_is_within_30_to_60_minutes() {
    let d = cfg().retry_delay_minutes;
    assert!((30..=60).contains(&d));
}

#[test]
fn a_retry_only_fires_if_the_evidence_is_still_there() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), Response::Ignored { is_retry: false });
    assert_eq!(
        s.decide(&cfg(), t(10, 50), day_start(), 1),
        PolicyDecision::Observe
    );
}

#[test]
fn a_retry_that_is_ignored_too_is_not_retried_again() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), Response::Ignored { is_retry: false });
    s.record_shown(t(10, 45));
    s.respond(&cfg(), t(10, 45), Response::Ignored { is_retry: true });

    assert!(s.retry.is_none());
    assert!(is_silent(&ask(&s, t(11, 30))), "60 minute cooldown");
    assert_eq!(
        ask(&s, t(11, 45)),
        ASK,
        "afterwards it is a normal, non-retry check-in"
    );
}

#[test]
fn a_stale_retry_lapses() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), Response::Ignored { is_retry: false }); // due 10:45, lapses 11:45
    assert_eq!(ask(&s, t(12, 0)), ASK, "no longer flagged as a retry");
}

#[test]
fn answering_settles_a_pending_retry() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(10, 0), Response::Ignored { is_retry: false });
    s.respond(&cfg(), t(10, 5), Response::Continue);
    assert!(s.retry.is_none());
}

// ---- 3 refusals -> ask about frequency ---------------------------------------------

#[test]
fn three_refusals_in_a_row_ask_whether_to_reduce_the_frequency() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(9, 0), Response::Continue);
    s.respond(&cfg(), t(10, 0), Response::LeaveMeAlone);
    assert!(!s.frequency_prompt_due(&cfg()));
    s.respond(&cfg(), t(11, 0), Response::DoNothing);
    assert!(s.frequency_prompt_due(&cfg()));
}

#[test]
fn accepting_a_break_resets_the_refusal_count() {
    let mut s = PolicyState::default();
    s.respond(&cfg(), t(9, 0), Response::Continue);
    s.respond(&cfg(), t(10, 0), Response::Continue);
    s.respond(&cfg(), t(11, 0), Response::AcceptedBreak);
    s.respond(&cfg(), t(12, 0), Response::Continue);
    assert!(!s.frequency_prompt_due(&cfg()));
}

#[test]
fn ignoring_is_not_a_refusal() {
    let mut s = PolicyState::default();
    for h in [9, 10, 11, 12] {
        s.respond(&cfg(), t(h, 0), Response::Ignored { is_retry: false });
    }
    assert!(!s.frequency_prompt_due(&cfg()));
}

#[test]
fn choosing_to_reduce_lowers_the_daily_limit() {
    let mut s = PolicyState {
        refusal_streak: 3,
        ..Default::default()
    };
    s.answer_frequency_prompt(&cfg(), true);
    assert_eq!(s.effective_ceiling(), 2);
    assert!(!s.frequency_prompt_due(&cfg()));
    s.record_shown(t(8, 0));
    s.record_shown(t(10, 0));
    assert!(matches!(
        ask(&s, t(14, 0)),
        PolicyDecision::Silent {
            reason: SilentReason::DailyCeiling { limit: 2 }
        }
    ));
}

#[test]
fn choosing_to_keep_the_frequency_changes_nothing_but_settles_the_question() {
    let mut s = PolicyState {
        refusal_streak: 3,
        ..Default::default()
    };
    s.answer_frequency_prompt(&cfg(), false);
    assert_eq!(s.effective_ceiling(), 4);
    assert!(!s.frequency_prompt_due(&cfg()));
}

// ---- service: persistence -----------------------------------------------------------

fn service() -> (Arc<Database>, PolicyService) {
    let db = Arc::new(Database::open_in_memory().unwrap());
    (db.clone(), PolicyService::new(db, cfg()))
}

#[test]
fn state_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    {
        let svc = PolicyService::new(Arc::new(Database::open(&path).unwrap()), cfg());
        svc.set_silent(true).unwrap();
        svc.respond(t(10, 0), Response::Continue).unwrap();
        svc.set_on_fire(true, t(10, 0)).unwrap();
        svc.record_shown(t(10, 0)).unwrap();
    }
    let svc = PolicyService::new(Arc::new(Database::open(&path).unwrap()), cfg());
    let s = svc.state().unwrap();
    assert!(s.silent);
    assert_eq!(s.cooldown_until, Some(t(11, 0)));
    assert_eq!(s.on_fire_until, Some(t(12, 0)));
    assert_eq!(s.shown, vec![t(10, 0)]);
}

#[test]
fn the_service_applies_the_same_rules() {
    let (_db, svc) = service();
    assert_eq!(svc.decide(t(10, 0), day_start(), 2).unwrap(), ASK);
    svc.respond(t(10, 0), Response::LeaveMeAlone).unwrap();
    assert!(is_silent(&svc.decide(t(10, 30), day_start(), 2).unwrap()));
    assert_eq!(svc.decide(t(11, 0), day_start(), 2).unwrap(), ASK);
}

#[test]
fn the_view_reports_what_is_in_effect() {
    let (_db, svc) = service();
    svc.record_shown(t(9, 0)).unwrap();
    svc.set_on_fire(true, t(10, 0)).unwrap();
    svc.respond(t(10, 0), Response::Continue).unwrap();
    let v = svc.view(t(10, 30), day_start()).unwrap();
    assert_eq!((v.shown_today, v.daily_limit), (1, 4));
    assert_eq!(v.on_fire_until, Some(t(12, 0)));
    assert_eq!(v.cooldown_until, Some(t(11, 0)));
    // Expired states are not reported as in effect.
    let later = svc.view(t(13, 0), day_start()).unwrap();
    assert_eq!((later.on_fire_until, later.cooldown_until), (None, None));
}

#[test]
fn deleting_all_local_data_resets_the_policy() {
    let (db, svc) = service();
    svc.set_silent(true).unwrap();
    svc.respond(t(10, 0), Response::Continue).unwrap();
    db.delete_all_local_data().unwrap();
    assert_eq!(svc.state().unwrap(), PolicyState::default());
}

#[test]
fn concurrent_updates_do_not_lose_each_other() {
    let (_db, svc) = service();
    let svc = Arc::new(svc);
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let svc = svc.clone();
            std::thread::spawn(move || svc.record_shown(t(8, 0) + mins(i)).unwrap())
        })
        .collect();
    handles.into_iter().for_each(|h| h.join().unwrap());
    assert_eq!(svc.state().unwrap().shown.len(), 8);
}
