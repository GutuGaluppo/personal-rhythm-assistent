//! Milestone 8: the pause timer is a deadline kept in the core, so it survives
//! anything that happens to a window; and the menu bar shows the state in words.

use chrono::{DateTime, TimeZone, Utc};
use personal_rhythm_assistant_lib::app::tray::{tooltip, tray_state, TrayState};
use personal_rhythm_assistant_lib::interventions::pause::{
    PauseError, PauseKind, PausePhase, PausePresenter, PauseService, MAX_MINUTES, MIN_MINUTES,
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
    let policy = Arc::new(PolicyService::new(db, PolicyConfig::default()));
    let log = Arc::new(Mutex::new(Log::default()));
    (
        PauseService::new(policy.clone(), Box::new(Fake(log.clone()))),
        policy,
        log,
    )
}

// ---- the timer -------------------------------------------------------------------

#[test]
fn a_pause_runs_for_the_chosen_time_then_is_done() {
    let (svc, _, _) = rig();
    let v = svc.start(PauseKind::Meditation, 5, t(10, 0, 0)).unwrap();
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
    svc.start(PauseKind::Walking, 10, t(10, 0, 0)).unwrap();

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
            svc.start(PauseKind::Silence, minutes, t(10, 0, 0)).is_ok(),
            "{minutes}"
        );
    }
}

#[test]
fn absurd_lengths_are_refused() {
    let (svc, _, _) = rig();
    for minutes in [0, MAX_MINUTES + 1, 24 * 60] {
        assert!(matches!(
            svc.start(PauseKind::Silence, minutes, t(10, 0, 0)),
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
    svc.start(PauseKind::Silence, 5, t(10, 0, 0)).unwrap();
    assert!(matches!(
        svc.start(PauseKind::Meditation, 3, t(10, 1, 0)),
        Err(PauseError::AlreadyRunning)
    ));
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().kind, PauseKind::Silence);
}

// ---- the return flow ----------------------------------------------------------------

#[test]
fn ending_returns_the_user_and_closes_the_window() {
    let (svc, _, log) = rig();
    svc.start(PauseKind::Silence, 5, t(10, 0, 0)).unwrap();
    svc.end();
    assert!(svc.view(t(10, 1, 0)).is_none());
    assert!(!svc.is_active());
    assert_eq!(log.lock().unwrap().closed, 1);
}

#[test]
fn a_pause_can_be_ended_early_without_consequence() {
    let (svc, policy, _) = rig();
    svc.start(PauseKind::Silence, 10, t(10, 0, 0)).unwrap();
    svc.end();
    // Nothing punishes leaving early: no refusal is recorded.
    assert_eq!(policy.state().unwrap().refusal_streak, 0);
}

#[test]
fn a_new_pause_can_start_after_coming_back() {
    let (svc, _, _) = rig();
    svc.start(PauseKind::Silence, 3, t(10, 0, 0)).unwrap();
    svc.end();
    assert!(svc.start(PauseKind::Stretching, 3, t(10, 10, 0)).is_ok());
}

#[test]
fn the_done_screen_stays_until_the_user_comes_back() {
    let (svc, _, _) = rig();
    svc.start(PauseKind::Silence, 3, t(10, 0, 0)).unwrap();
    assert_eq!(svc.view(t(11, 0, 0)).unwrap().phase, PausePhase::Done);
    assert!(svc.is_active(), "still a pause until the user returns");
    svc.end();
    assert!(svc.view(t(11, 0, 0)).is_none());
}

#[test]
fn closing_the_window_from_outside_leaves_the_pause() {
    let (svc, _, log) = rig();
    svc.start(PauseKind::Silence, 5, t(10, 0, 0)).unwrap();
    svc.window_closed();
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
    svc.start(PauseKind::Meditation, 5, t(10, 0, 0)).unwrap();
    svc.offer(PauseKind::Walking);
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().phase, PausePhase::Running);
    assert_eq!(svc.view(t(10, 1, 0)).unwrap().kind, PauseKind::Meditation);
    assert_eq!(log.lock().unwrap().shown, 0);
}

#[test]
fn the_setup_can_lead_to_any_pause_kind() {
    let (svc, _, _) = rig();
    svc.offer(PauseKind::Silence);
    let v = svc.start(PauseKind::Stretching, 5, t(10, 0, 0)).unwrap();
    assert_eq!(v.kind, PauseKind::Stretching);
}

// ---- policy ----------------------------------------------------------------------------

#[test]
fn starting_a_pause_is_an_accepted_break_whichever_way_it_began() {
    let (svc, policy, _) = rig();
    // Started from the menu bar, not from a check-in: the policy still cools down.
    svc.start(PauseKind::Silence, 5, t(10, 0, 0)).unwrap();
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
    let _ = svc.start(PauseKind::Silence, 0, t(10, 0, 0));
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
    // Distinct pictures, so no two states can look the same.
    let all: std::collections::BTreeSet<Vec<u8>> = ["default", "pause", "on_fire", "silent"]
        .iter()
        .map(|n| std::fs::read(dir.join(format!("{n}.png"))).unwrap())
        .collect();
    assert_eq!(all.len(), 4);
}
