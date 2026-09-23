use super::detectors;
use super::signals::{ContextAssessment, ContextDecision, Signal};
use crate::persistence::time;
use crate::sessions::model::Session;
use chrono::{DateTime, Utc};

/// Every threshold is configurable during development (IMPLEMENTATION.md §10).
/// Defaults are starting points to be tuned in Milestone 13.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextConfig {
    /// Signal A: active minutes in the current session.
    pub continuous_activity_minutes: f64,
    /// Signal B: only checked for sessions at least this long...
    pub idle_check_min_session_minutes: f64,
    /// ...and met when idle time is below this share of the session.
    pub min_idle_ratio: f64,
    /// Signal C: only checked for sessions at least this long...
    pub rapid_switching_min_session_minutes: f64,
    /// ...and met at this many app switches per hour or more.
    pub rapid_switching_per_hour: f64,
    /// Signal D: look back this many days...
    pub create_dominance_days: i64,
    /// ...met when Create is at least this share of active time...
    pub create_dominance_share: f64,
    /// ...and there are at least this many active minutes to compare.
    pub create_dominance_min_active_minutes: f64,
    /// Never intervene on one isolated signal.
    pub min_active_signals: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            continuous_activity_minutes: 90.0,
            idle_check_min_session_minutes: 60.0,
            min_idle_ratio: 0.05,
            rapid_switching_min_session_minutes: 15.0,
            rapid_switching_per_hour: 30.0,
            create_dominance_days: 3,
            create_dominance_share: 0.75,
            create_dominance_min_active_minutes: 240.0,
            min_active_signals: 2,
        }
    }
}

pub struct ContextInput<'a> {
    pub now: DateTime<Utc>,
    /// The session in progress, freshly computed.
    pub current_session: Option<&'a Session>,
    /// Recent sessions from storage (may include a stale copy of the current one).
    pub recent_sessions: &'a [Session],
}

pub fn assess(cfg: &ContextConfig, input: ContextInput) -> ContextAssessment {
    let current = input.current_session;

    let mut sessions: Vec<&Session> = input
        .recent_sessions
        .iter()
        .filter(|s| current.is_none_or(|c| c.id != s.id))
        .collect();
    sessions.extend(current);

    let signals: Vec<Signal> = vec![
        detectors::continuous_activity(cfg, current),
        detectors::insufficient_idle(cfg, current),
        detectors::rapid_switching(cfg, current),
        detectors::create_dominance(cfg, input.now, &sessions),
    ];

    let active_signal_count = signals.iter().filter(|s| s.active).count();
    let decision = if active_signal_count >= cfg.min_active_signals {
        ContextDecision::CandidateIntervention
    } else {
        ContextDecision::Observe
    };

    ContextAssessment {
        assessed_at: time::format(input.now),
        signals,
        active_signal_count,
        decision,
    }
}
