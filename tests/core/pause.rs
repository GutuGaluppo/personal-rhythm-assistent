//! Milestone 8: the pause timer is a deadline kept in the core, so it survives
//! anything that happens to a window; and the menu bar shows the state in words.

use chrono::{DateTime, TimeZone, Utc};
use personal_rhythm_assistant_lib::app::tray::{tooltip, tray_state, TrayState};
use personal_rhythm_assistant_lib::interventions::pause::{
    PauseError, PauseKind, PausePhase, PausePresenter, PauseReason, PauseReasonKind, PauseService,
    MAX_MINUTES, MIN_MINUTES,
};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::policy::rules::{PolicyConfig, PolicyDecision};
use personal_rhythm_assistant_lib::policy::service::{PolicyService, PolicyView};
use std::sync::{Arc, Mutex};

fn t(h: u32, m: u32, s: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 4, h, m, s).unwrap()
}

#[derive(Default)]
struct Log {
    shown: usize,
    closed: usize,
}
struct Fake(Arc<Mutex<Log>>);
impl PausePresenter for Fake {
    fn show(&self) {
        self.0.lock().unwrap().shown += 1;
    }
    fn close(&self) {
        self.0.lock().unwrap().closed += 1;
    }
}

fn rig() -> (PauseService, Arc<PolicyService>, Arc<Mutex<Log>>) {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let policy = Arc::new(PolicyService::new(db.clone(), PolicyConfig::default()));
    let log = Arc::new(Mutex::new(Log::default()));
    (
        PauseService::new(policy.clone(), db, Box::new(Fake(log.clone()))),
        policy,
        log,
    )
}

// ---- the timer -------------------------------------------------------------------

#[test]
fn a_pause_runs_for_the_chosen_time_then_is_done() {
    let (svc, _, _) = rig();
    let v = svc
        .start(PauseKind::Meditation, 5, None, t(10, 0, 0))
        .unwrap();
    assert_eq!(v.phase, PausePhase::Running);
    assert_eq!(v.duration_seconds, Some(300));
    assert_eq!(v.started_at.as_deref(), Some("2026-03-04T10:00:00.000Z"));
    assert_eq!(v.ends_at.as_deref(), Some("2026-03-04T10:05:00.000Z"));

    assert_eq!(svc.view(t(10, 4, 59)).unwrap().phase, PausePhase::Running);
    assert_eq!(svc.view(t(10, 5, 0)).unwrap().phase, PausePhase::Done);
    assert!(svc.is_running(t(10, 4, 59)));
    assert!(!svc.is_running(t(10, 5, 0)));
}

#[test]
fn the_timer_is_a_deadline_so_time_passing_unobserved_changes_nothing() {
    // No window, no ticks, no focus: only the clock. Looking again after a long
    // gap (a window in the background, a sleeping laptop) gives the exact answer.
    let (svc, _, _) = rig();
    svc.start(PauseKind::Walking, 10, None, t(10, 0, 0))
        .unwrap();

    let a = svc.view(t(10, 3, 0)).unwrap();
    let b = svc.view(t(10, 7, 0)).unwrap();
    assert_eq!(
        (a.phase, b.phase),
        (PausePhase::Running, PausePhase::Running)
    );
    assert_eq!(a.ends_at, b.ends_at, "the end never moves");
    assert_eq!(svc.view(t(13, 0, 0)).unwrap().phase, PausePhase::Done);
}

#[test]
fn the_standard_lengths_and_custom_ones_within_bounds_are_accepted() {
    for minutes in [3, 5, 10, MIN_MINUTES, 17, MAX_MINUTES] {
        let (svc, _, _) = rig();
        assert!(
            svc.start(PauseKind::Silence, minutes, None, t(10, 0, 0))
                .is_ok(),
            "{minutes}"
        );
    }
}

#[test]
fn absurd_lengths_are_refused() {
    let (svc, _, _) = rig();
    for minutes in [0, MAX_MINUTES + 1, 24 * 60] {
        assert!(matches!(
            svc.start(PauseKind::Silence, minutes, None, t(10, 0, 0)),
            Err(PauseError::InvalidDuration { .. })
        ));
    }
    assert!(
        svc.view(t(10, 0, 0)).is_none(),
        "a refused start leaves nothing behind"
    );
}

#[test]
fn only_one_pause_at_a_time() {
    let (svc, _, _) = rig();
    svc.start(PauseKind::Silence, 5, None, t(10, 0, 0)).unwrap();
    assert!(matches!(
        svc.start(PauseKind::Meditation, 3, None, t(10, 1, 0)),
        Err(PauseError::AlreadyRunning)
    ));
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().kind, PauseKind::Silence);
}

// ---- the return flow ----------------------------------------------------------------

#[test]
fn ending_returns_the_user_and_closes_the_window() {
    let (svc, _, log) = rig();
    svc.start(PauseKind::Silence, 5, None, t(10, 0, 0)).unwrap();
    svc.end(t(10, 6, 0));
    assert!(svc.view(t(10, 1, 0)).is_none());
    assert!(!svc.is_active());
    assert_eq!(log.lock().unwrap().closed, 1);
}

#[test]
fn a_pause_can_be_ended_early_without_consequence() {
    let (svc, policy, _) = rig();
    svc.start(PauseKind::Silence, 10, None, t(10, 0, 0))
        .unwrap();
    svc.end(t(10, 6, 0));
    // Nothing punishes leaving early: no refusal is recorded.
    assert_eq!(policy.state().unwrap().refusal_streak, 0);
}

#[test]
fn a_new_pause_can_start_after_coming_back() {
    let (svc, _, _) = rig();
    svc.start(PauseKind::Silence, 3, None, t(10, 0, 0)).unwrap();
    svc.end(t(10, 6, 0));
    assert!(svc
        .start(PauseKind::Stretching, 3, None, t(10, 10, 0))
        .is_ok());
}

#[test]
fn the_done_screen_stays_until_the_user_comes_back() {
    let (svc, _, _) = rig();
    svc.start(PauseKind::Silence, 3, None, t(10, 0, 0)).unwrap();
    assert_eq!(svc.view(t(11, 0, 0)).unwrap().phase, PausePhase::Done);
    assert!(svc.is_active(), "still a pause until the user returns");
    svc.end(t(10, 6, 0));
    assert!(svc.view(t(11, 0, 0)).is_none());
}

#[test]
fn closing_the_window_from_outside_leaves_the_pause() {
    let (svc, _, log) = rig();
    svc.start(PauseKind::Silence, 5, None, t(10, 0, 0)).unwrap();
    svc.window_closed(t(10, 1, 0));
    assert!(svc.view(t(10, 1, 0)).is_none());
    assert_eq!(log.lock().unwrap().closed, 0, "nothing left to close");
}

// ---- setup ---------------------------------------------------------------------------

#[test]
fn offering_opens_the_setup_screen_with_the_suggested_kind() {
    let (svc, _, log) = rig();
    svc.offer(PauseKind::Meditation);
    let v = svc.view(t(10, 0, 0)).unwrap();
    assert_eq!(
        (v.phase, v.kind),
        (PausePhase::Setup, PauseKind::Meditation)
    );
    assert_eq!(v.ends_at, None);
    assert_eq!(log.lock().unwrap().shown, 1);
}

#[test]
fn a_setup_screen_is_not_yet_a_pause() {
    let (svc, _, _) = rig();
    svc.offer(PauseKind::Silence);
    assert!(
        !svc.is_active(),
        "check-ins are only held back once the pause has started"
    );
}

#[test]
fn offering_during_a_running_pause_changes_nothing() {
    let (svc, _, log) = rig();
    svc.start(PauseKind::Meditation, 5, None, t(10, 0, 0))
        .unwrap();
    svc.offer(PauseKind::Walking);
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().phase, PausePhase::Running);
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().kind, PauseKind::Meditation);
    assert_eq!(log.lock().unwrap().shown, 0);
}

#[test]
fn the_setup_can_lead_to_any_pause_kind() {
    let (svc, _, _) = rig();
    svc.offer(PauseKind::Silence);
    let v = svc
        .start(PauseKind::Stretching, 5, None, t(10, 0, 0))
        .unwrap();
    assert_eq!(v.kind, PauseKind::Stretching);
}

// ---- quick pause -------------------------------------------------------------------------

#[test]
fn a_quick_pause_skips_setup_and_opens_the_window_itself() {
    let (svc, _, log) = rig();
    let v = svc.start_quick(t(10, 0, 0)).unwrap();
    assert_eq!(v.phase, PausePhase::Running);
    assert_eq!(v.kind, PauseKind::Silence);
    assert_eq!(v.ends_at, None, "untimed: no deadline");
    assert_eq!(v.duration_seconds, None);
    assert_eq!(
        log.lock().unwrap().shown,
        1,
        "start_quick opens the window itself"
    );
}

#[test]
fn a_quick_pause_never_becomes_done_on_its_own() {
    let (svc, _, _) = rig();
    svc.start_quick(t(10, 0, 0)).unwrap();
    // A very long time passes with nobody touching it.
    assert_eq!(svc.view(t(23, 0, 0)).unwrap().phase, PausePhase::Running);
    assert!(
        !svc.is_running(t(23, 0, 0)),
        "is_running is only for deadline-based pauses"
    );
    assert!(svc.is_active(), "but it is still an active pause");
}

#[test]
fn only_one_pause_at_a_time_covers_quick_pauses_too() {
    let (svc, _, _) = rig();
    svc.start_quick(t(10, 0, 0)).unwrap();
    assert!(matches!(
        svc.start_quick(t(10, 1, 0)),
        Err(PauseError::AlreadyRunning)
    ));
    assert!(matches!(
        svc.start(PauseKind::Silence, 5, None, t(10, 1, 0)),
        Err(PauseError::AlreadyRunning)
    ));
}

#[test]
fn refocus_does_nothing_unless_a_pause_is_running() {
    let (svc, _, log) = rig();
    svc.refocus_if_active();
    assert_eq!(log.lock().unwrap().shown, 0);

    svc.start_quick(t(10, 0, 0)).unwrap();
    svc.refocus_if_active();
    assert_eq!(
        log.lock().unwrap().shown,
        2,
        "start_quick's own show, plus refocus"
    );
}

#[test]
fn ending_a_quick_pause_works_the_same_as_a_timed_one() {
    let (svc, _, log) = rig();
    svc.start_quick(t(10, 0, 0)).unwrap();
    svc.end(t(10, 20, 0));
    assert!(svc.view(t(10, 20, 0)).is_none());
    assert!(!svc.is_active());
    assert_eq!(log.lock().unwrap().closed, 1);
}

// ---- reason --------------------------------------------------------------------------

#[test]
fn a_preset_reason_is_attached_and_returned() {
    let (svc, _, _) = rig();
    let reason = PauseReason {
        kind: PauseReasonKind::Lunch,
        note: None,
    };
    let v = svc
        .start(PauseKind::Silence, 5, Some(reason.clone()), t(10, 0, 0))
        .unwrap();
    assert_eq!(v.reason, Some(reason));
}

#[test]
fn a_note_is_kept_only_for_other_and_is_trimmed() {
    let (svc, _, _) = rig();
    let reason = PauseReason {
        kind: PauseReasonKind::Other,
        note: Some("  waiting for a delivery  ".into()),
    };
    let v = svc
        .start(PauseKind::Silence, 5, Some(reason), t(10, 0, 0))
        .unwrap();
    assert_eq!(
        v.reason,
        Some(PauseReason {
            kind: PauseReasonKind::Other,
            note: Some("waiting for a delivery".into()),
        })
    );

    // A note on any other kind is dropped: it isn't shown for that kind.
    let (svc2, _, _) = rig();
    let v2 = svc2
        .start(
            PauseKind::Silence,
            5,
            Some(PauseReason {
                kind: PauseReasonKind::Call,
                note: Some("ignored".into()),
            }),
            t(10, 0, 0),
        )
        .unwrap();
    assert_eq!(
        v2.reason,
        Some(PauseReason {
            kind: PauseReasonKind::Call,
            note: None
        })
    );
}

#[test]
fn other_without_a_note_is_refused() {
    let (svc, _, _) = rig();
    let empty = PauseReason {
        kind: PauseReasonKind::Other,
        note: Some("   ".into()),
    };
    assert!(matches!(
        svc.start(PauseKind::Silence, 5, Some(empty), t(10, 0, 0)),
        Err(PauseError::InvalidReason(_))
    ));
    assert!(
        svc.view(t(10, 0, 0)).is_none(),
        "a refused start leaves nothing behind"
    );
}

#[test]
fn a_reason_can_be_attached_after_a_quick_pause_has_already_started() {
    let (svc, _, _) = rig();
    let v = svc.start_quick(t(10, 0, 0)).unwrap();
    assert_eq!(v.reason, None);

    let updated = svc
        .set_reason(
            Some(PauseReason {
                kind: PauseReasonKind::Breakfast,
                note: None,
            }),
            t(10, 1, 0),
        )
        .unwrap();
    assert_eq!(
        updated.reason,
        Some(PauseReason {
            kind: PauseReasonKind::Breakfast,
            note: None,
        })
    );
    // The live view reflects it too, not just the return value.
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().reason, updated.reason);
}

#[test]
fn a_reason_can_be_cleared_and_replaced() {
    let (svc, _, _) = rig();
    svc.start(
        PauseKind::Silence,
        5,
        Some(PauseReason {
            kind: PauseReasonKind::Lunch,
            note: None,
        }),
        t(10, 0, 0),
    )
    .unwrap();

    let cleared = svc.set_reason(None, t(10, 1, 0)).unwrap();
    assert_eq!(cleared.reason, None);
}

#[test]
fn setting_a_reason_with_nothing_running_is_refused() {
    let (svc, _, _) = rig();
    assert!(matches!(
        svc.set_reason(None, t(10, 0, 0)),
        Err(PauseError::NotRunning)
    ));
}

// ---- policy ----------------------------------------------------------------------------

#[test]
fn starting_a_pause_is_an_accepted_break_whichever_way_it_began() {
    let (svc, policy, _) = rig();
    // Started from the menu bar, not from a check-in: the policy still cools down.
    svc.start(PauseKind::Silence, 5, None, t(10, 0, 0)).unwrap();
    let d = policy.decide(t(10, 30, 0), t(0, 0, 0), 4).unwrap();
    assert!(matches!(d, PolicyDecision::Silent { .. }));
    assert_eq!(
        policy.decide(t(11, 0, 0), t(0, 0, 0), 4).unwrap(),
        PolicyDecision::AskCheckin { is_retry: false }
    );
}

#[test]
fn a_refused_start_does_not_touch_the_policy() {
    let (svc, policy, _) = rig();
    let _ = svc.start(PauseKind::Silence, 0, None, t(10, 0, 0));
    assert!(policy.state().unwrap().cooldown_until.is_none());
}

// ---- menu bar state ---------------------------------------------------------------------

fn view(silent: bool, fire: Option<DateTime<Utc>>) -> PolicyView {
    PolicyView {
        silent,
        on_fire_until: fire,
        cooldown_until: None,
        shown_today: 0,
        daily_limit: 4,
        retry_pending: false,
        frequency_prompt_due: false,
    }
}

#[test]
fn the_menu_bar_shows_one_state_in_order_of_precedence() {
    let fire = Some(t(12, 0, 0));
    assert_eq!(tray_state(&view(false, None), false), TrayState::Normal);
    assert_eq!(tray_state(&view(true, None), false), TrayState::Silent);
    assert_eq!(tray_state(&view(false, fire), false), TrayState::OnFire);
    assert_eq!(tray_state(&view(true, fire), false), TrayState::OnFire);
    assert_eq!(tray_state(&view(true, fire), true), TrayState::Paused);
}

#[test]
fn every_state_is_labelled_in_words() {
    let all = [
        (TrayState::Normal, "Personal Rhythm Assistant"),
        (TrayState::Paused, "In a pause"),
        (TrayState::OnFire, "I'm on fire until 12:00 UTC"),
        (TrayState::Silent, "Silent"),
    ];
    for (state, expected) in all {
        let text = tooltip(state, Some(t(12, 0, 0)));
        assert!(text.contains(expected), "{state:?}: {text}");
    }
    // The states are distinguishable without seeing the icon.
    let texts: std::collections::BTreeSet<_> = all
        .iter()
        .map(|(s, _)| tooltip(*s, Some(t(12, 0, 0))))
        .collect();
    assert_eq!(texts.len(), 4);
}

#[test]
fn the_on_fire_tooltip_has_no_urgency_or_flame_language() {
    let text = tooltip(TrayState::OnFire, Some(t(12, 0, 0))).to_lowercase();
    for banned in [
        "!", "urgent", "hurry", "burning", "flame", "alert", "warning",
    ] {
        assert!(!text.contains(banned), "{banned:?} in {text:?}");
    }
}

#[test]
fn the_state_icons_exist_and_are_real_pngs() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/tray");
    for name in ["default", "pause", "on_fire", "silent"] {
        let bytes = std::fs::read(dir.join(format!("{name}.png"))).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{name}");
    }
    // Pause intentionally reuses the Normal icon (no approved art of its own,
    // and there's no ambiguity: the tray tooltip always states the difference
    // in words). Every other state must still look distinct.
    let pause = std::fs::read(dir.join("pause.png")).unwrap();
    let default = std::fs::read(dir.join("default.png")).unwrap();
    assert_eq!(pause, default, "pause reuses the default icon on purpose");

    let all: std::collections::BTreeSet<Vec<u8>> = ["default", "on_fire", "silent"]
        .iter()
        .map(|n| std::fs::read(dir.join(format!("{n}.png"))).unwrap())
        .collect();
    assert_eq!(all.len(), 3);
}
