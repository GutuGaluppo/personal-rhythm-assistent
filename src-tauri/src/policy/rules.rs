use chrono::{DateTime, Utc};
use serde::Serialize;

/// The absolute experimental ceiling (IMPLEMENTATION.md §2 rule 14).
pub const HARD_DAILY_CEILING: u32 = 4;

/// Every number here is a starting point to be tuned in Milestone 13.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicyConfig {
    pub cooldown_after_continue_minutes: i64,
    pub cooldown_after_leave_me_alone_minutes: i64,
    /// "Accepted break → no immediate follow-up."
    pub cooldown_after_accepted_break_minutes: i64,
    pub cooldown_after_do_nothing_minutes: i64,
    pub on_fire_minutes: i64,
    /// One retry of an ignored check-in, 30–60 minutes later.
    pub retry_delay_minutes: i64,
    /// A pending retry that has not fired this long after it was due is dropped.
    pub retry_lapse_minutes: i64,
    /// Cooldown after a retry is ignored too.
    pub cooldown_after_ignored_retry_minutes: i64,
    /// Consecutive refusals before asking whether to reduce the frequency.
    pub frequency_prompt_refusals: u32,
    /// Daily limit chosen when the user says "reduce".
    pub reduced_daily_limit: u32,
    /// Signals the Context Engine must report before anything may surface.
    pub min_active_signals: usize,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            cooldown_after_continue_minutes: 60,
            cooldown_after_leave_me_alone_minutes: 60,
            cooldown_after_accepted_break_minutes: 60,
            cooldown_after_do_nothing_minutes: 60,
            on_fire_minutes: 120,
            retry_delay_minutes: 45,
            retry_lapse_minutes: 60,
            cooldown_after_ignored_retry_minutes: 60,
            frequency_prompt_refusals: 3,
            reduced_daily_limit: 2,
            min_active_signals: 2,
        }
    }
}

/// Why nothing may surface. Checked in this order; the first that applies wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SilentReason {
    /// The user asked for silence. Overrides everything.
    SilentMode,
    Cooldown {
        until: DateTime<Utc>,
    },
    DailyCeiling {
        limit: u32,
    },
    OnFire {
        until: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyDecision {
    Silent {
        reason: SilentReason,
    },
    /// Allowed to surface, but the evidence is not there.
    Observe,
    /// May ask "How is your energy right now?". `is_retry` marks the single retry
    /// of a check-in that was ignored.
    AskCheckin {
        is_retry: bool,
    },
}

pub struct RuleInput {
    pub now: DateTime<Utc>,
    pub silent: bool,
    pub cooldown_until: Option<DateTime<Utc>>,
    pub shown_today: u32,
    pub ceiling: u32,
    pub on_fire_until: Option<DateTime<Utc>>,
    pub active_signals: usize,
    pub min_active_signals: usize,
    pub retry_pending: bool,
}

/// IMPLEMENTATION.md §11, in the same order:
/// silence, cooldown, daily ceiling, on fire, then the evidence.
pub fn decide(i: &RuleInput) -> PolicyDecision {
    if i.silent {
        return silent(SilentReason::SilentMode);
    }
    if let Some(until) = i.cooldown_until.filter(|u| *u > i.now) {
        return silent(SilentReason::Cooldown { until });
    }
    if i.shown_today >= i.ceiling {
        return silent(SilentReason::DailyCeiling { limit: i.ceiling });
    }
    if let Some(until) = i.on_fire_until.filter(|u| *u > i.now) {
        return silent(SilentReason::OnFire { until });
    }
    if i.active_signals < i.min_active_signals {
        return PolicyDecision::Observe;
    }
    PolicyDecision::AskCheckin {
        is_retry: i.retry_pending,
    }
}

fn silent(reason: SilentReason) -> PolicyDecision {
    PolicyDecision::Silent { reason }
}
