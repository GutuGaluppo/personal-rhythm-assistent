//! Milestone 7: the check-in flow. The window itself is faked; what is tested is
//! when a check-in appears, what each answer does, and how it ends.

use chrono::{DateTime, Duration, TimeZone, Utc};
use personal_rhythm_assistant_lib::context::engine::{assess, ContextConfig, ContextInput};
use personal_rhythm_assistant_lib::context::signals::ContextAssessment;
use personal_rhythm_assistant_lib::interventions::manager::{
    InterventionConfig, InterventionError, InterventionManager, Presenter,
};
use personal_rhythm_assistant_lib::interventions::model::{
    Action, Answer, Energy, InterventionView, Step,
};
use personal_rhythm_assistant_lib::interventions::pause::{
    PauseKind, PausePresenter, PauseService,
};
use personal_rhythm_assistant_lib::persistence::repositories::{interventions, settings};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::policy::engine::PolicyState;
use personal_rhythm_assistant_lib::policy::rules::{PolicyConfig, PolicyDecision};
use personal_rhythm_assistant_lib::policy::service::PolicyService;
use personal_rhythm_assistant_lib::privacy::retention;
use personal_rhythm_assistant_lib::sessions::model::{Category, Session};
use std::sync::{Arc, Mutex};

fn t(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 4, h, m, 0).unwrap()
}
fn day_start() -> DateTime<Utc> {
    t(0, 0)
}

#[derive(Default)]
struct Log {
    shown: Vec<InterventionView>,
    closed: usize,
}
struct FakePresenter(Arc<Mutex<Log>>);
impl Presenter for FakePresenter {
    fn show(&self, view: &InterventionView) -> Result<(), String> {
        self.0.lock().unwrap().shown.push(view.clone());
        Ok(())
    }
    fn close(&self) {
        self.0.lock().unwrap().closed += 1;
    }
}

#[derive(Default)]
struct PauseLog {
    shown: usize,
    closed: usize,
}
struct FakePausePresenter(Arc<Mutex<PauseLog>>);
impl PausePresenter for FakePausePresenter {
    fn show(&self) {
        self.0.lock().unwrap().shown += 1;
    }
    fn close(&self) {
        self.0.lock().unwrap().closed += 1;
    }
}

struct Rig {
    manager: InterventionManager,
    pause: Arc<PauseService>,
    pause_log: Arc<Mutex<PauseLog>>,
    policy: Arc<PolicyService>,
    db: Arc<Database>,
    log: Arc<Mutex<Log>>,
}
fn rig() -> Rig {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let policy = Arc::new(PolicyService::new(db.clone(), PolicyConfig::default()));
    let log = Arc::new(Mutex::new(Log::default()));
    let pause_log = Arc::new(Mutex::new(PauseLog::default()));
    let pause = Arc::new(PauseService::new(
        policy.clone(),
        Box::new(FakePausePresenter(pause_log.clone())),
    ));
    let manager = InterventionManager::new(
        db.clone(),
        policy.clone(),
        Box::new(FakePresenter(log.clone())),
        pause.clone(),
        InterventionConfig::default(),
    );
    Rig {
        manager,
        pause,
        pause_log,
        policy,
        db,
        log,
    }
}

fn session(active: f64, idle: f64) -> Session {
    Session {
        id: "s".into(),
        started_at: "2026-03-04T08:00:00.000Z".into(),
        ended_at: None,
        active_minutes: active,
        idle_minutes: idle,
        context_switches: 3,
        category: Category::Create,
        application_ids: vec![],
        project_id: None,
    }
}
fn assessment_at(now: DateTime<Utc>, s: Option<&Session>) -> ContextAssessment {
    assess(
        &ContextConfig::default(),
        ContextInput {
            now,
            current_session: s,
            recent_sessions: &[],
        },
    )
}
/// Two signals (long + no idle): a candidate.
fn candidate(now: DateTime<Utc>) -> ContextAssessment {
    assessment_at(now, Some(&session(96.0, 0.0)))
}
/// One signal only.
fn weak(now: DateTime<Utc>) -> ContextAssessment {
    assessment_at(now, Some(&session(95.0, 20.0)))
}

fn show(r: &Rig, now: DateTime<Utc>) -> InterventionView {
    r.manager
        .consider(now, day_start(), &candidate(now), true)
        .unwrap()
        .expect("a check-in")
}
fn decision(r: &Rig, now: DateTime<Utc>) -> PolicyDecision {
    r.policy.decide(now, day_start(), 2).unwrap()
}
fn state(r: &Rig) -> PolicyState {
    r.policy.state().unwrap()
}
fn energy(v: Energy) -> Answer {
    Answer::Energy { value: v }
}
fn action(v: Action) -> Answer {
    Answer::Action { value: v }
}
fn feedback(r: &Rig, id: &str) -> Vec<(String, String)> {
    r.db.with_conn(|c| interventions::feedback_for(c, id))
        .unwrap()
}

// ---- when a check-in appears --------------------------------------------------------

#[test]
fn a_real_candidate_shows_the_check_in_with_its_reason() {
    let r = rig();
    let v = show(&r, t(10, 0));

    assert_eq!(v.step, Step::Energy);
    assert_eq!(v.headline, "You've been active for 96 minutes.");
    assert_eq!(v.question, "How is your energy right now?");
    assert_eq!(r.log.lock().unwrap().shown, vec![v.clone()]);
    // The reason is visible: the evidence, restated.
    assert_eq!(
        v.reasons,
        vec![
            "Active for 96 minutes in this session.".to_string(),
            "0 minutes without input in a 96-minute session.".to_string(),
        ]
    );
    assert_eq!(r.manager.current(), Some(v));
}

#[test]
fn showing_counts_toward_the_daily_ceiling_and_is_stored() {
    let r = rig();
    let v = show(&r, t(10, 0));
    assert_eq!(state(&r).shown_today(day_start()), 1);
    assert_eq!(r.db.with_conn(interventions::count).unwrap(), 1);
    let reasons =
        r.db.with_conn(|c| interventions::reasons_of(c, &v.id))
            .unwrap();
    assert_eq!(reasons, v.reasons);
}

#[test]
fn one_signal_is_not_enough() {
    let r = rig();
    assert!(r
        .manager
        .consider(t(10, 0), day_start(), &weak(t(10, 0)), true)
        .unwrap()
        .is_none());
    assert!(r.log.lock().unwrap().shown.is_empty());
}

#[test]
fn nothing_appears_while_the_user_is_away() {
    let r = rig();
    assert!(r
        .manager
        .consider(t(10, 0), day_start(), &candidate(t(10, 0)), false)
        .unwrap()
        .is_none());
}

#[test]
fn nothing_appears_while_the_policy_says_silent() {
    let r = rig();
    r.policy.set_silent(true).unwrap();
    assert!(r
        .manager
        .consider(t(10, 0), day_start(), &candidate(t(10, 0)), true)
        .unwrap()
        .is_none());
}

#[test]
fn only_one_check_in_at_a_time() {
    let r = rig();
    show(&r, t(10, 0));
    assert!(r
        .manager
        .consider(t(10, 1), day_start(), &candidate(t(10, 1)), true)
        .unwrap()
        .is_none());
    assert_eq!(r.log.lock().unwrap().shown.len(), 1);
}

// ---- the flow ------------------------------------------------------------------------

#[test]
fn good_then_continue_starts_a_60_minute_cooldown() {
    let r = rig();
    let v = show(&r, t(10, 0));

    let next = r
        .manager
        .answer(t(10, 0), &v.id, energy(Energy::Good))
        .unwrap()
        .unwrap();
    assert_eq!(next.step, Step::Positive);
    assert!(r
        .manager
        .answer(t(10, 0), &v.id, action(Action::Continue))
        .unwrap()
        .is_none());

    assert!(r.manager.current().is_none());
    assert_eq!(r.log.lock().unwrap().closed, 1);
    assert!(matches!(
        decision(&r, t(10, 59)),
        PolicyDecision::Silent { .. }
    ));
    assert_eq!(
        decision(&r, t(11, 0)),
        PolicyDecision::AskCheckin { is_retry: false }
    );
    assert_eq!(
        feedback(&r, &v.id),
        vec![
            ("energy".into(), "good".into()),
            ("action".into(), "continue".into())
        ]
    );
}

#[test]
fn okay_follows_the_same_path_as_good() {
    let r = rig();
    let v = show(&r, t(10, 0));
    let next = r
        .manager
        .answer(t(10, 0), &v.id, energy(Energy::Okay))
        .unwrap()
        .unwrap();
    assert_eq!(next.step, Step::Positive);
}

#[test]
fn taking_a_break_is_an_accepted_break() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager
        .answer(t(10, 0), &v.id, energy(Energy::Good))
        .unwrap();
    r.manager
        .answer(t(10, 1), &v.id, action(Action::TakeBreak))
        .unwrap();
    assert!(matches!(
        decision(&r, t(10, 30)),
        PolicyDecision::Silent { .. }
    ));
    assert_eq!(state(&r).refusal_streak, 0);
}

#[test]
fn low_offers_move_meditate_or_do_nothing_and_none_is_a_refusal() {
    // "Do nothing" is a silent pause (Pause Mode's "nothing"), not a refusal.
    for (a, refusal) in [
        (Action::Move, 0),
        (Action::Meditate, 0),
        (Action::DoNothing, 0),
    ] {
        let r = rig();
        let v = show(&r, t(10, 0));
        let next = r
            .manager
            .answer(t(10, 0), &v.id, energy(Energy::Low))
            .unwrap()
            .unwrap();
        assert_eq!(next.step, Step::Low);
        assert!(r
            .manager
            .answer(t(10, 1), &v.id, action(a))
            .unwrap()
            .is_none());
        assert_eq!(state(&r).refusal_streak, refusal, "{a:?}");
        assert!(matches!(
            decision(&r, t(10, 30)),
            PolicyDecision::Silent { .. }
        ));
    }
}

#[test]
fn on_fire_can_be_chosen_from_the_check_in() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager
        .answer(t(10, 0), &v.id, energy(Energy::Good))
        .unwrap();
    r.manager
        .answer(t(10, 1), &v.id, action(Action::OnFire))
        .unwrap();
    assert!(state(&r).on_fire_active(t(11, 0)));
}

#[test]
fn leave_me_alone_works_on_every_step_and_cools_down_for_an_hour() {
    for steps in [
        vec![],
        vec![energy(Energy::Good)],
        vec![energy(Energy::Low)],
    ] {
        let r = rig();
        let v = show(&r, t(10, 0));
        for a in steps {
            r.manager.answer(t(10, 0), &v.id, a).unwrap();
        }
        assert!(r
            .manager
            .answer(t(10, 5), &v.id, Answer::LeaveMeAlone)
            .unwrap()
            .is_none());
        assert!(matches!(
            decision(&r, t(11, 4)),
            PolicyDecision::Silent { .. }
        ));
        assert_eq!(
            decision(&r, t(11, 5)),
            PolicyDecision::AskCheckin { is_retry: false }
        );
        assert_eq!(feedback(&r, &v.id).last().unwrap().1, "leave_me_alone");
    }
}

#[test]
fn answers_must_fit_the_step_being_shown() {
    let r = rig();
    let v = show(&r, t(10, 0));
    // Actions are not available before the energy question is answered.
    assert!(matches!(
        r.manager.answer(t(10, 0), &v.id, action(Action::Continue)),
        Err(InterventionError::WrongStep {
            shown: Step::Energy
        })
    ));
    r.manager
        .answer(t(10, 0), &v.id, energy(Energy::Good))
        .unwrap();
    // "Move" belongs to the Low step, not the Good one.
    assert!(matches!(
        r.manager.answer(t(10, 0), &v.id, action(Action::Move)),
        Err(InterventionError::WrongStep {
            shown: Step::Positive
        })
    ));
    // The energy question cannot be answered twice.
    assert!(matches!(
        r.manager.answer(t(10, 0), &v.id, energy(Energy::Low)),
        Err(InterventionError::WrongStep { .. })
    ));
    assert!(
        r.manager.current().is_some(),
        "a wrong answer does not end the check-in"
    );
}

#[test]
fn a_stale_or_unknown_check_in_id_is_refused() {
    let r = rig();
    let v = show(&r, t(10, 0));
    assert!(matches!(
        r.manager
            .answer(t(10, 0), "intervention-old", Answer::LeaveMeAlone),
        Err(InterventionError::NotActive)
    ));
    r.manager
        .answer(t(10, 0), &v.id, Answer::LeaveMeAlone)
        .unwrap();
    assert!(matches!(
        r.manager.dismiss(t(10, 1), &v.id),
        Err(InterventionError::NotActive)
    ));
}

// ---- dismissing, timing out, retrying -----------------------------------------------

#[test]
fn dismissing_is_one_call_and_counts_as_ignored() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager.dismiss(t(10, 0), &v.id).unwrap();

    assert!(r.manager.current().is_none());
    assert_eq!(r.log.lock().unwrap().closed, 1);
    assert_eq!(
        feedback(&r, &v.id),
        vec![("action".into(), "ignored".into())]
    );
    assert!(state(&r).retry.is_some());
}

#[test]
fn an_untouched_check_in_expires_after_20_seconds() {
    let r = rig();
    show(&r, t(10, 0));
    assert!(!r.manager.expire(t(10, 0) + Duration::seconds(19)).unwrap());
    assert!(r.manager.current().is_some());
    assert!(r.manager.expire(t(10, 0) + Duration::seconds(20)).unwrap());
    assert!(r.manager.current().is_none());
    assert_eq!(r.log.lock().unwrap().closed, 1);
}

#[test]
fn answering_a_step_gives_the_user_another_20_seconds() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager
        .answer(
            t(10, 0) + Duration::seconds(15),
            &v.id,
            energy(Energy::Good),
        )
        .unwrap();
    assert!(!r.manager.expire(t(10, 0) + Duration::seconds(30)).unwrap());
    assert!(r.manager.expire(t(10, 0) + Duration::seconds(35)).unwrap());
}

#[test]
fn a_window_closed_from_outside_counts_as_dismissed() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager.window_closed(t(10, 0)).unwrap();
    assert!(r.manager.current().is_none());
    assert_eq!(
        feedback(&r, &v.id),
        vec![("action".into(), "ignored".into())]
    );
    // ...and closing it again (our own destroy event) does nothing more.
    r.manager.window_closed(t(10, 0)).unwrap();
    assert_eq!(feedback(&r, &v.id).len(), 1);
}

#[test]
fn an_ignored_check_in_is_retried_once_45_minutes_later() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager.dismiss(t(10, 0), &v.id).unwrap();

    assert!(r
        .manager
        .consider(t(10, 44), day_start(), &candidate(t(10, 44)), true)
        .unwrap()
        .is_none());
    let retry = show(&r, t(10, 45));
    assert_ne!(retry.id, v.id);
    let stored: bool =
        r.db.with_conn(|c| {
            Ok(c.query_row(
                "SELECT is_retry FROM interventions WHERE id = ?1",
                [&retry.id],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert!(stored, "the retry is recorded as a retry");

    // Ignored again: no third attempt, a plain 60 minute cooldown.
    r.manager.dismiss(t(10, 45), &retry.id).unwrap();
    assert!(r
        .manager
        .consider(t(11, 30), day_start(), &candidate(t(11, 30)), true)
        .unwrap()
        .is_none());
    assert!(state(&r).retry.is_none());
}

#[test]
fn the_daily_ceiling_holds_across_check_ins() {
    let r = rig();
    for h in [8, 10, 12, 14] {
        let v = show(&r, t(h, 0));
        r.manager
            .answer(t(h, 0), &v.id, Answer::LeaveMeAlone)
            .unwrap();
    }
    assert!(r
        .manager
        .consider(t(18, 0), day_start(), &candidate(t(18, 0)), true)
        .unwrap()
        .is_none());
    assert_eq!(state(&r).shown_today(day_start()), 4);
}

// ---- wording ----------------------------------------------------------------------------

#[test]
fn wording_never_repeats_back_to_back() {
    let r = rig();
    let mut previous: Option<(String, String)> = None;
    for i in 0..9 {
        let v = r
            .manager
            .preview(t(10, 0) + Duration::minutes(i), &candidate(t(10, 0)))
            .unwrap()
            .unwrap();
        let this = (v.headline.replace("96", "N"), v.question.clone());
        assert_ne!(
            Some(this.clone()),
            previous,
            "presentation {i} repeated the previous wording"
        );
        previous = Some(this);
        r.manager.dismiss(t(10, 0), &v.id).unwrap();
    }
}

#[test]
fn wording_rotation_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rhythm.sqlite");
    let mut headlines = vec![];
    for _ in 0..2 {
        let db = Arc::new(Database::open(&path).unwrap());
        let policy = Arc::new(PolicyService::new(db.clone(), PolicyConfig::default()));
        let manager = InterventionManager::new(
            db,
            policy,
            Box::new(FakePresenter(Arc::new(Mutex::new(Log::default())))),
            Arc::new(PauseService::new(
                Arc::new(PolicyService::new(
                    Arc::new(Database::open_in_memory().unwrap()),
                    PolicyConfig::default(),
                )),
                Box::new(FakePausePresenter(Arc::new(
                    Mutex::new(PauseLog::default()),
                ))),
            )),
            InterventionConfig::default(),
        );
        headlines.push(
            manager
                .preview(t(10, 0), &candidate(t(10, 0)))
                .unwrap()
                .unwrap()
                .headline,
        );
    }
    assert_ne!(headlines[0], headlines[1]);
    let saved: Option<usize> = Database::open(&path)
        .unwrap()
        .with_conn(|c| settings::get_json(c, "last_intervention_wording"))
        .unwrap();
    assert_eq!(saved, Some(1));
}

#[test]
fn no_wording_carries_urgency_or_judgement() {
    let r = rig();
    let banned = [
        "!",
        "urgent",
        "now!",
        "warning",
        "should",
        "must",
        "too long",
        "too much",
        "overwork",
        "tired",
        "stress",
        "burn",
        "hurry",
        "last chance",
        "streak",
        "score",
    ];
    for i in 0..6 {
        let v = r
            .manager
            .preview(t(10, 0), &candidate(t(10, 0)))
            .unwrap()
            .unwrap();
        let text = format!("{} {} {}", v.headline, v.question, v.reasons.join(" ")).to_lowercase();
        for word in banned {
            assert!(
                !text.contains(word),
                "{word:?} in {text:?} (presentation {i})"
            );
        }
        r.manager.dismiss(t(10, 0), &v.id).unwrap();
    }
}

// ---- developer preview ----------------------------------------------------------------

#[test]
fn a_preview_leaves_no_trace_and_does_not_touch_the_policy() {
    let r = rig();
    let v = r
        .manager
        .preview(t(10, 0), &candidate(t(10, 0)))
        .unwrap()
        .unwrap();
    r.manager
        .answer(t(10, 0), &v.id, Answer::LeaveMeAlone)
        .unwrap();

    assert_eq!(r.db.with_conn(interventions::count).unwrap(), 0);
    assert_eq!(state(&r), PolicyState::default(), "no cooldown, no count");
}

// ---- storage ---------------------------------------------------------------------------

#[test]
fn old_check_ins_and_their_feedback_are_removed_by_retention() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager
        .answer(t(10, 0), &v.id, Answer::LeaveMeAlone)
        .unwrap();

    let report =
        r.db.with_conn(|c| retention::apply(c, t(10, 0) + Duration::days(40)))
            .unwrap();
    assert_eq!(report.interventions_deleted, 1);
    assert!(
        feedback(&r, &v.id).is_empty(),
        "feedback goes with its check-in"
    );
}

#[test]
fn deleting_all_local_data_removes_check_ins_too() {
    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager
        .answer(t(10, 0), &v.id, Answer::LeaveMeAlone)
        .unwrap();
    r.db.delete_all_local_data().unwrap();
    assert_eq!(r.db.with_conn(interventions::count).unwrap(), 0);
    assert!(feedback(&r, &v.id).is_empty());
}

// ---- Pause Mode hand-off ------------------------------------------------------------

fn finish_with(r: &Rig, first: Option<Energy>, last: Action) {
    let v = show(r, t(10, 0));
    if let Some(e) = first {
        r.manager.answer(t(10, 0), &v.id, energy(e)).unwrap();
    }
    r.manager.answer(t(10, 1), &v.id, action(last)).unwrap();
}

#[test]
fn accepted_actions_open_the_pause_window_with_a_matching_suggestion() {
    for (first, last, expected) in [
        (Energy::Good, Action::TakeBreak, PauseKind::Silence),
        (Energy::Low, Action::Meditate, PauseKind::Meditation),
        (Energy::Low, Action::Move, PauseKind::Walking),
        (Energy::Low, Action::DoNothing, PauseKind::Silence),
    ] {
        let r = rig();
        finish_with(&r, Some(first), last);
        assert_eq!(r.pause_log.lock().unwrap().shown, 1, "{last:?}");
        assert_eq!(r.pause.view(t(10, 1)).unwrap().kind, expected, "{last:?}");
    }
}

#[test]
fn continuing_or_leaving_me_alone_never_opens_a_pause() {
    let r = rig();
    finish_with(&r, Some(Energy::Good), Action::Continue);
    assert_eq!(r.pause_log.lock().unwrap().shown, 0);

    let r = rig();
    let v = show(&r, t(10, 0));
    r.manager
        .answer(t(10, 0), &v.id, Answer::LeaveMeAlone)
        .unwrap();
    assert_eq!(r.pause_log.lock().unwrap().shown, 0);
}

#[test]
fn on_fire_does_not_open_a_pause() {
    let r = rig();
    finish_with(&r, Some(Energy::Good), Action::OnFire);
    assert_eq!(r.pause_log.lock().unwrap().shown, 0);
}

#[test]
fn nothing_interrupts_a_pause_in_progress() {
    let r = rig();
    r.pause.start(PauseKind::Meditation, 5, t(10, 0)).unwrap();
    // Even long after the policy cooldown a check-in must wait for the pause to end.
    let later = t(12, 0);
    assert!(r
        .manager
        .consider(later, day_start(), &candidate(later), true)
        .unwrap()
        .is_none());

    r.pause.end();
    // (the accepted break started a 60 minute cooldown at 10:00; by 12:00 it is over)
    assert!(r
        .manager
        .consider(later, day_start(), &candidate(later), true)
        .unwrap()
        .is_some());
}
