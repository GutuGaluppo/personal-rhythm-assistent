//! Pause Mode v0.1 (IMPLEMENTATION.md §15).
//!
//! The pause lives here, in the core, as a *deadline* (`ends_at`), not as a
//! ticking counter inside a window. Windows only draw `ends_at - now`, so the
//! timer cannot drift, stall or reset when a window loses focus, is hidden, or
//! is throttled in the background.

use crate::persistence::error::PersistenceError;
use crate::persistence::repositories::pauses;
use crate::persistence::{time, Database};
use crate::policy::engine::Response;
use crate::policy::service::PolicyService;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

pub const MIN_MINUTES: u32 = 1;
pub const MAX_MINUTES: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseKind {
    Silence,
    Meditation,
    Walking,
    Stretching,
}

impl PauseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Silence => "silence",
            Self::Meditation => "meditation",
            Self::Walking => "walking",
            Self::Stretching => "stretching",
        }
    }
}

/// A free-text note attached to an "Other" reason is capped, like other
/// user-entered text in this app (see `interest_inbox::MAX_CHARS`).
pub const REASON_NOTE_MAX_CHARS: usize = 140;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseReasonKind {
    Breakfast,
    Lunch,
    Dinner,
    Appointment,
    Call,
    Other,
}

impl PauseReasonKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Breakfast => "breakfast",
            Self::Lunch => "lunch",
            Self::Dinner => "dinner",
            Self::Appointment => "appointment",
            Self::Call => "call",
            Self::Other => "other",
        }
    }
}

/// Optional justification for a pause. `note` is only meaningful when
/// `kind` is `Other`; it is dropped otherwise (see `PauseService::normalize_reason`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseReason {
    pub kind: PauseReasonKind,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PausePhase {
    /// Choosing what kind of pause and for how long.
    Setup,
    Running,
    /// The time is up; waiting for the user to come back.
    Done,
}

/// What the pause window draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseView {
    pub phase: PausePhase,
    pub kind: PauseKind,
    pub started_at: Option<String>,
    /// `None` for a quick, untimed pause: no deadline, no auto "Done".
    pub ends_at: Option<String>,
    pub duration_seconds: Option<u32>,
    pub reason: Option<PauseReason>,
}

#[derive(Debug)]
pub enum PauseError {
    AlreadyRunning,
    /// No pause is currently running, so there is nothing to attach a reason to.
    NotRunning,
    InvalidDuration {
        minutes: u32,
    },
    InvalidReason(String),
    Persistence(PersistenceError),
}

impl fmt::Display for PauseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "a pause is already running"),
            Self::NotRunning => write!(f, "no pause is running"),
            Self::InvalidDuration { minutes } => {
                write!(
                    f,
                    "{minutes} minutes is outside {MIN_MINUTES}-{MAX_MINUTES}"
                )
            }
            Self::InvalidReason(reason) => write!(f, "{reason}"),
            Self::Persistence(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for PauseError {}
impl From<PersistenceError> for PauseError {
    fn from(e: PersistenceError) -> Self {
        Self::Persistence(e)
    }
}

/// Opens and closes the pause window. Implemented by the Tauri layer.
pub trait PausePresenter: Send + Sync {
    fn show(&self);
    fn close(&self);
}

#[derive(Debug, Clone)]
enum State {
    Setup {
        suggested: PauseKind,
    },
    Running {
        id: String,
        kind: PauseKind,
        started_at: DateTime<Utc>,
        /// `None` for a quick, untimed pause: it runs until the user ends it.
        ends_at: Option<DateTime<Utc>>,
        reason: Option<PauseReason>,
    },
}

pub struct PauseService {
    policy: Arc<PolicyService>,
    db: Arc<Database>,
    presenter: Box<dyn PausePresenter>,
    state: Mutex<Option<State>>,
}

impl PauseService {
    pub fn new(
        policy: Arc<PolicyService>,
        db: Arc<Database>,
        presenter: Box<dyn PausePresenter>,
    ) -> Self {
        Self {
            policy,
            db,
            presenter,
            state: Mutex::new(None),
        }
    }

    /// Opens the pause window on its setup screen with `suggested` preselected.
    /// Does nothing while a pause is already running.
    pub fn offer(&self, suggested: PauseKind) {
        let mut state = self.state.lock().unwrap();
        if matches!(*state, Some(State::Running { .. })) {
            return;
        }
        *state = Some(State::Setup { suggested });
        drop(state);
        self.presenter.show();
    }

    /// Starts the timer. Taking a pause is an accepted break, whichever way it began.
    pub fn start(
        &self,
        kind: PauseKind,
        minutes: u32,
        reason: Option<PauseReason>,
        now: DateTime<Utc>,
    ) -> Result<PauseView, PauseError> {
        if !(MIN_MINUTES..=MAX_MINUTES).contains(&minutes) {
            return Err(PauseError::InvalidDuration { minutes });
        }
        let reason = Self::normalize_reason(reason)?;
        let mut state = self.state.lock().unwrap();
        if matches!(*state, Some(State::Running { .. })) {
            return Err(PauseError::AlreadyRunning);
        }
        self.policy.respond(now, Response::AcceptedBreak)?;
        let id = format!("pause-{}", time::format(now));
        self.db.with_conn(|c| {
            pauses::insert(
                c,
                &id,
                &time::format(now),
                kind.as_str(),
                Some(minutes * 60),
                reason.as_ref(),
            )
        })?;
        let ends_at = Some(now + Duration::minutes(minutes.into()));
        let running = State::Running {
            id,
            kind,
            started_at: now,
            ends_at,
            reason,
        };
        let view = view_of(&running, now);
        *state = Some(running);
        Ok(view)
    }

    /// Starts a quick, untimed pause straight away: no setup screen, no deadline.
    /// Ended only by the user (or by starting a timed pause never happens while
    /// one is already running, per `AlreadyRunning` below).
    pub fn start_quick(&self, now: DateTime<Utc>) -> Result<PauseView, PauseError> {
        let mut state = self.state.lock().unwrap();
        if matches!(*state, Some(State::Running { .. })) {
            return Err(PauseError::AlreadyRunning);
        }
        self.policy.respond(now, Response::AcceptedBreak)?;
        let id = format!("pause-{}", time::format(now));
        self.db.with_conn(|c| {
            pauses::insert(
                c,
                &id,
                &time::format(now),
                PauseKind::Silence.as_str(),
                None,
                None,
            )
        })?;
        let running = State::Running {
            id,
            kind: PauseKind::Silence,
            started_at: now,
            ends_at: None,
            reason: None,
        };
        let view = view_of(&running, now);
        *state = Some(running);
        drop(state);
        // There was no setup screen to have opened the window already.
        self.presenter.show();
        Ok(view)
    }

    /// Attaches or changes the reason on the currently running pause. Needed
    /// because a quick pause starts before any reason is chosen.
    pub fn set_reason(
        &self,
        reason: Option<PauseReason>,
        now: DateTime<Utc>,
    ) -> Result<PauseView, PauseError> {
        let reason = Self::normalize_reason(reason)?;
        let mut state = self.state.lock().unwrap();
        let Some(State::Running {
            id,
            reason: current,
            ..
        }) = state.as_mut()
        else {
            return Err(PauseError::NotRunning);
        };
        self.db
            .with_conn(|c| pauses::set_reason(c, id, reason.as_ref()))?;
        *current = reason;
        Ok(view_of(state.as_ref().unwrap(), now))
    }

    /// A free-text note only makes sense for `Other`; trims it and enforces the
    /// length cap, and drops it for every other reason kind.
    fn normalize_reason(reason: Option<PauseReason>) -> Result<Option<PauseReason>, PauseError> {
        let Some(mut reason) = reason else {
            return Ok(None);
        };
        if reason.kind != PauseReasonKind::Other {
            reason.note = None;
            return Ok(Some(reason));
        }
        let note = reason.note.as_deref().unwrap_or("").trim().to_string();
        if note.is_empty() {
            return Err(PauseError::InvalidReason(
                "\"Other\" needs a short note".into(),
            ));
        }
        if note.chars().count() > REASON_NOTE_MAX_CHARS {
            return Err(PauseError::InvalidReason(format!(
                "a reason note is at most {REASON_NOTE_MAX_CHARS} characters"
            )));
        }
        reason.note = Some(note);
        Ok(Some(reason))
    }

    pub fn view(&self, now: DateTime<Utc>) -> Option<PauseView> {
        self.state.lock().unwrap().as_ref().map(|s| view_of(s, now))
    }

    /// True from the start until the user comes back, including the "done" screen.
    /// Nothing else may interrupt someone in a pause.
    pub fn is_active(&self) -> bool {
        matches!(*self.state.lock().unwrap(), Some(State::Running { .. }))
    }

    /// True while a timed pause has not yet reached its deadline. An untimed
    /// (quick) pause is never "running" in this deadline sense, only `is_active`.
    pub fn is_running(&self, now: DateTime<Utc>) -> bool {
        matches!(
            *self.state.lock().unwrap(),
            Some(State::Running { ends_at: Some(deadline), .. }) if now < deadline
        )
    }

    /// Brings the pause window forward if (and only if) a pause is running.
    /// Used when the quick-pause shortcut fires again mid-pause, when the user
    /// is detected active again, and when they manually reopen the app.
    pub fn refocus_if_active(&self) {
        if self.is_active() {
            self.presenter.show();
        }
    }

    /// The user is back (or ended early): clears the pause and closes the window.
    pub fn end(&self, now: DateTime<Utc>) {
        self.finish(now);
        self.presenter.close();
    }

    /// The window went away on its own: same as coming back, nothing to close.
    pub fn window_closed(&self, now: DateTime<Utc>) {
        self.finish(now);
    }

    /// Clears the pause and records when it ended.
    fn finish(&self, now: DateTime<Utc>) {
        let finished = self.state.lock().unwrap().take();
        if let Some(State::Running { id, .. }) = finished {
            let _ = self
                .db
                .with_conn(|c| pauses::set_ended(c, &id, &time::format(now)));
        }
    }
}

fn view_of(state: &State, now: DateTime<Utc>) -> PauseView {
    match state {
        State::Setup { suggested } => PauseView {
            phase: PausePhase::Setup,
            kind: *suggested,
            started_at: None,
            ends_at: None,
            duration_seconds: None,
            reason: None,
        },
        State::Running {
            kind,
            started_at,
            ends_at,
            reason,
            ..
        } => PauseView {
            phase: match ends_at {
                Some(deadline) if now >= *deadline => PausePhase::Done,
                _ => PausePhase::Running,
            },
            kind: *kind,
            started_at: Some(time::format(*started_at)),
            ends_at: ends_at.map(time::format),
            duration_seconds: ends_at.map(|e| (e - *started_at).num_seconds() as u32),
            reason: reason.clone(),
        },
    }
}
