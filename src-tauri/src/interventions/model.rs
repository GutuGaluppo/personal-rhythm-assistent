use serde::{Deserialize, Serialize};

/// Which screen of the check-in the user is on (IMPLEMENTATION.md §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// "How is your energy right now?" Good / Okay / Low.
    Energy,
    /// After Good or Okay: Continue / Take a break (/ I'm on fire).
    Positive,
    /// After Low: Move / Meditate / Do nothing.
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Energy {
    Good,
    Okay,
    Low,
}

impl Energy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Okay => "okay",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Continue,
    TakeBreak,
    Move,
    Meditate,
    DoNothing,
    OnFire,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::TakeBreak => "take_break",
            Self::Move => "move",
            Self::Meditate => "meditate",
            Self::DoNothing => "do_nothing",
            Self::OnFire => "on_fire",
        }
    }

    /// The step on which this action is offered.
    pub fn offered_on(self) -> Step {
        match self {
            Self::Continue | Self::TakeBreak | Self::OnFire => Step::Positive,
            Self::Move | Self::Meditate | Self::DoNothing => Step::Low,
        }
    }
}

/// What the intervention window may send back. Nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Answer {
    Energy {
        value: Energy,
    },
    Action {
        value: Action,
    },
    /// Available on every step.
    LeaveMeAlone,
}

/// Everything the window needs to draw itself. Deliberately small: the window
/// can read this, answer, or dismiss, and nothing more.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionView {
    pub id: String,
    pub headline: String,
    pub question: String,
    /// Why this is being shown, as plain statements of the measurements.
    pub reasons: Vec<String>,
    pub step: Step,
}
