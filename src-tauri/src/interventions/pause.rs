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
    pub ends_at: Option<String>,
    pub duration_seconds: Option<u32>,
}

#[derive(Debug)]
pub enum PauseError {
    AlreadyRunning,
    InvalidDuration { minutes: u32 },
    Persistence(PersistenceError),
}

impl fmt::Display for PauseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "a pause is already running"),
            Self::InvalidDuration { minutes } => {
                write!(
                    f,
                    "{minutes} minutes is outside {MIN_MINUTES}-{MAX_MINUTES}"
                )
            }
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
        ends_at: DateTime<Utc>,
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
        now: DateTime<Utc>,
    ) -> Result<PauseView, PauseError> {
        if !(MIN_MINUTES..=MAX_MINUTES).contains(&minutes) {
            return Err(PauseError::InvalidDuration { minutes });
        }
        let mut state = self.state.lock().unwrap();
        if matches!(*state, Some(State::Running { .. })) {
            return Err(PauseError::AlreadyRunning);
        }
        self.policy.respond(now, Response::AcceptedBreak)?;
        let id = format!("pause-{}", time::format(now));
        self.db.with_conn(|c| {
            pauses::insert(c, &id, &time::format(now), kind.as_str(), minutes * 60)
        })?;
        let ends_at = now + Duration::minutes(minutes.into());
        let running = State::Running {
            id,
            kind,
            started_at: now,
            ends_at,
        };
        let view = view_of(&running, now);
        *state = Some(running);
        Ok(view)
    }

    pub fn view(&self, now: DateTime<Utc>) -> Option<PauseView> {
        self.state.lock().unwrap().as_ref().map(|s| view_of(s, now))
    }

    /// True from the start until the user comes back, including the "done" screen.
    /// Nothing else may interrupt someone in a pause.
    pub fn is_active(&self) -> bool {
        matches!(*self.state.lock().unwrap(), Some(State::Running { .. }))
    }

    pub fn is_running(&self, now: DateTime<Utc>) -> bool {
        matches!(*self.state.lock().unwrap(), Some(State::Running { ends_at, .. }) if now < ends_at)
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
    match *state {
        State::Setup { suggested } => PauseView {
            phase: PausePhase::Setup,
            kind: suggested,
            started_at: None,
            ends_at: None,
            duration_seconds: None,
        },
        State::Running {
            kind,
            started_at,
            ends_at,
            ..
        } => PauseView {
            phase: if now >= ends_at {
                PausePhase::Done
            } else {
                PausePhase::Running
            },
            kind,
            started_at: Some(time::format(started_at)),
            ends_at: Some(time::format(ends_at)),
            duration_seconds: Some((ends_at - started_at).num_seconds() as u32),
        },
    }
}
