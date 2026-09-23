use super::model::{Action, Answer, Energy, InterventionView, Step};
use super::pause::{PauseKind, PauseService};
use crate::context::signals::{ContextAssessment, SignalKind};
use crate::persistence::error::PersistenceError;
use crate::persistence::repositories::{interventions, settings};
use crate::persistence::{time, Database};
use crate::policy::engine::Response;
use crate::policy::rules::PolicyDecision;
use crate::policy::service::PolicyService;
use chrono::{DateTime, Duration, Utc};
use std::fmt;
use std::sync::{Arc, Mutex};

/// Puts the check-in in front of the user. Implemented by the Tauri window.
pub trait Presenter: Send + Sync {
    fn show(&self, view: &InterventionView) -> std::result::Result<(), String>;
    fn close(&self);
}

#[derive(Debug)]
pub enum InterventionError {
    Persistence(PersistenceError),
    /// No check-in is showing, or the answer is for an older one.
    NotActive,
    /// The answer does not belong to the step being shown.
    WrongStep {
        shown: Step,
    },
    Presenter(String),
}

pub type Result<T> = std::result::Result<T, InterventionError>;

impl fmt::Display for InterventionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(e) => write!(f, "{e}"),
            Self::NotActive => write!(f, "that check-in is no longer showing"),
            Self::WrongStep { shown } => write!(f, "that answer does not fit the {shown:?} step"),
            Self::Presenter(e) => write!(f, "could not show the check-in: {e}"),
        }
    }
}
impl std::error::Error for InterventionError {}
impl From<PersistenceError> for InterventionError {
    fn from(e: PersistenceError) -> Self {
        Self::Persistence(e)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InterventionConfig {
    /// "Visible roughly 20 seconds"; restarts after each answered step.
    pub visible_seconds: i64,
}

impl Default for InterventionConfig {
    fn default() -> Self {
        Self {
            visible_seconds: 20,
        }
    }
}

/// Never the same wording twice in a row (IMPLEMENTATION.md §12). Plain statements,
/// no urgency, no judgement.
const WORDINGS: [(&str, &str); 3] = [
    (
        "You've been active for {n} minutes.",
        "How is your energy right now?",
    ),
    (
        "About {n} minutes of activity so far.",
        "How's your energy?",
    ),
    ("{n} minutes in.", "How are you feeling, energy-wise?"),
];
const LAST_WORDING_KEY: &str = "last_intervention_wording";

struct Active {
    view: InterventionView,
    is_retry: bool,
    deadline: DateTime<Utc>,
    /// False for a developer preview: nothing is recorded and the policy is untouched.
    counted: bool,
}

pub struct InterventionManager {
    db: Arc<Database>,
    policy: Arc<PolicyService>,
    presenter: Box<dyn Presenter>,
    pause: Arc<PauseService>,
    cfg: InterventionConfig,
    active: Mutex<Option<Active>>,
}

impl InterventionManager {
    pub fn new(
        db: Arc<Database>,
        policy: Arc<PolicyService>,
        presenter: Box<dyn Presenter>,
        pause: Arc<PauseService>,
        cfg: InterventionConfig,
    ) -> Self {
        Self {
            db,
            policy,
            presenter,
            pause,
            cfg,
            active: Mutex::new(None),
        }
    }

    pub fn current(&self) -> Option<InterventionView> {
        self.active.lock().unwrap().as_ref().map(|a| a.view.clone())
    }

    /// Shows a check-in if, and only if, the Policy Engine allows it, the evidence is
    /// there, the user is at the machine and nothing is showing already.
    pub fn consider(
        &self,
        now: DateTime<Utc>,
        day_start: DateTime<Utc>,
        assessment: &ContextAssessment,
        user_present: bool,
    ) -> Result<Option<InterventionView>> {
        if !user_present || self.pause.is_active() || self.active.lock().unwrap().is_some() {
            return Ok(None);
        }
        let decision = self
            .policy
            .decide(now, day_start, assessment.active_signal_count)?;
        let PolicyDecision::AskCheckin { is_retry } = decision else {
            return Ok(None);
        };
        self.present(now, assessment, is_retry, true)
    }

    /// Developer preview: shows the window without consulting or affecting the policy.
    pub fn preview(
        &self,
        now: DateTime<Utc>,
        assessment: &ContextAssessment,
    ) -> Result<Option<InterventionView>> {
        self.present(now, assessment, false, false)
    }

    fn present(
        &self,
        now: DateTime<Utc>,
        assessment: &ContextAssessment,
        is_retry: bool,
        counted: bool,
    ) -> Result<Option<InterventionView>> {
        let mut active = self.active.lock().unwrap();
        if active.is_some() {
            return Ok(None);
        }

        let minutes = assessment
            .signals
            .iter()
            .find(|s| s.kind == SignalKind::ContinuousActivity)
            .map_or(0.0, |s| s.evidence.measured);
        let reasons: Vec<String> = assessment
            .signals
            .iter()
            .filter(|s| s.active)
            .map(|s| s.evidence.explanation.clone())
            .collect();

        let (headline, question) = self.next_wording(minutes)?;
        let view = InterventionView {
            id: format!("intervention-{}", time::format(now)),
            headline,
            question,
            reasons,
            step: Step::Energy,
        };

        if counted {
            self.db.with_conn(|c| {
                interventions::insert(
                    c,
                    &interventions::NewIntervention {
                        id: &view.id,
                        shown_at: &time::format(now),
                        is_retry,
                        active_minutes: minutes,
                        reasons: &view.reasons,
                    },
                )
            })?;
            self.policy.record_shown(now)?;
        }
        self.presenter
            .show(&view)
            .map_err(InterventionError::Presenter)?;

        *active = Some(Active {
            view: view.clone(),
            is_retry,
            deadline: now + Duration::seconds(self.cfg.visible_seconds),
            counted,
        });
        Ok(Some(view))
    }

    fn next_wording(&self, minutes: f64) -> Result<(String, String)> {
        let last: Option<usize> = self
            .db
            .with_conn(|c| settings::get_json(c, LAST_WORDING_KEY))?;
        let next = last.map_or(0, |i| (i + 1) % WORDINGS.len());
        self.db
            .with_conn(|c| settings::set_json(c, LAST_WORDING_KEY, &next))?;
        let (headline, question) = WORDINGS[next];
        Ok((
            headline.replace("{n}", &format!("{}", minutes.round())),
            question.to_string(),
        ))
    }

    /// Applies an answer. Returns the next screen, or `None` when the check-in is over.
    pub fn answer(
        &self,
        now: DateTime<Utc>,
        id: &str,
        answer: Answer,
    ) -> Result<Option<InterventionView>> {
        let mut guard = self.active.lock().unwrap();
        let active = guard
            .as_mut()
            .filter(|a| a.view.id == id)
            .ok_or(InterventionError::NotActive)?;

        let response = match answer {
            Answer::Energy { value } => {
                if active.view.step != Step::Energy {
                    return Err(InterventionError::WrongStep {
                        shown: active.view.step,
                    });
                }
                self.record(active, now, "energy", value.as_str())?;
                active.view.step = if value == Energy::Low {
                    Step::Low
                } else {
                    Step::Positive
                };
                active.deadline = now + Duration::seconds(self.cfg.visible_seconds);
                return Ok(Some(active.view.clone()));
            }
            Answer::Action { value } => {
                if value.offered_on() != active.view.step {
                    return Err(InterventionError::WrongStep {
                        shown: active.view.step,
                    });
                }
                self.record(active, now, "action", value.as_str())?;
                match value {
                    Action::Continue => Response::Continue,
                    // "Do nothing" is a silent pause, not a refusal.
                    Action::TakeBreak | Action::Move | Action::Meditate | Action::DoNothing => {
                        Response::AcceptedBreak
                    }
                    Action::OnFire => Response::OnFire,
                }
            }
            Answer::LeaveMeAlone => {
                self.record(active, now, "action", "leave_me_alone")?;
                Response::LeaveMeAlone
            }
        };

        let finished = guard.take().expect("checked above");
        drop(guard); // closing the window may call back into `window_closed`
        if finished.counted {
            self.policy.respond(now, response)?;
        }
        self.presenter.close();
        if let Answer::Action { value } = answer {
            if let Some(kind) = pause_for(value) {
                self.pause.offer(kind);
            }
        }
        Ok(None)
    }

    /// The one-click dismiss, or a window the user closed some other way.
    pub fn dismiss(&self, now: DateTime<Utc>, id: &str) -> Result<()> {
        if self.current().is_none_or(|v| v.id != id) {
            return Err(InterventionError::NotActive);
        }
        self.ignore(now)
    }

    /// The window went away without an answer (not by our own doing).
    pub fn window_closed(&self, now: DateTime<Utc>) -> Result<()> {
        self.ignore(now)
    }

    /// Dismisses a check-in that has been visible long enough without being touched.
    /// Returns whether it did.
    pub fn expire(&self, now: DateTime<Utc>) -> Result<bool> {
        let due = self
            .active
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|a| now >= a.deadline);
        if due {
            self.ignore(now)?;
        }
        Ok(due)
    }

    /// Not answered: treated as ignored, which schedules the single retry.
    fn ignore(&self, now: DateTime<Utc>) -> Result<()> {
        let Some(mut active) = self.active.lock().unwrap().take() else {
            return Ok(());
        };
        if active.counted {
            self.record(&mut active, now, "action", "ignored")?;
            self.policy.respond(
                now,
                Response::Ignored {
                    is_retry: active.is_retry,
                },
            )?;
        }
        self.presenter.close();
        Ok(())
    }

    fn record(
        &self,
        active: &mut Active,
        now: DateTime<Utc>,
        kind: &str,
        value: &str,
    ) -> Result<()> {
        if active.counted {
            self.db.with_conn(|c| {
                interventions::add_feedback(c, &active.view.id, &time::format(now), kind, value)
            })?;
        }
        Ok(())
    }
}

/// The pause a chosen action leads to, preselected on the pause window's setup screen.
fn pause_for(action: Action) -> Option<PauseKind> {
    match action {
        Action::TakeBreak | Action::DoNothing => Some(PauseKind::Silence),
        Action::Meditate => Some(PauseKind::Meditation),
        Action::Move => Some(PauseKind::Walking),
        Action::Continue | Action::OnFire => None,
    }
}
