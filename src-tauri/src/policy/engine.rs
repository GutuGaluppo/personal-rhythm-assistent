use super::rules::{decide, PolicyConfig, PolicyDecision, RuleInput, HARD_DAILY_CEILING};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// How the user answered a check-in (or didn't).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
    Continue,
    LeaveMeAlone,
    /// "Take a break", "Move" or "Meditate".
    AcceptedBreak,
    DoNothing,
    /// "I'm on fire", chosen from the check-in.
    OnFire,
    /// Timed out or dismissed without an answer.
    Ignored {
        is_retry: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRetry {
    pub due_at: DateTime<Utc>,
    pub lapses_at: DateTime<Utc>,
}

/// Everything the Policy Engine remembers. Persisted, so a restart does not
/// forget a cooldown or an "I'm on fire".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyState {
    pub silent: bool,
    pub cooldown_until: Option<DateTime<Utc>>,
    pub on_fire_until: Option<DateTime<Utc>>,
    /// When check-ins surfaced (retries included); the last week is kept.
    pub shown: Vec<DateTime<Utc>>,
    pub retry: Option<PendingRetry>,
    /// Consecutive refusals (Continue / Leave me alone / Do nothing).
    pub refusal_streak: u32,
    /// The user's own daily limit. Never above the hard ceiling.
    pub max_per_day: u32,
}

impl Default for PolicyState {
    fn default() -> Self {
        Self {
            silent: false,
            cooldown_until: None,
            on_fire_until: None,
            shown: vec![],
            retry: None,
            refusal_streak: 0,
            max_per_day: HARD_DAILY_CEILING,
        }
    }
}

impl PolicyState {
    pub fn effective_ceiling(&self) -> u32 {
        self.max_per_day.min(HARD_DAILY_CEILING)
    }

    pub fn shown_today(&self, day_start: DateTime<Utc>) -> u32 {
        self.shown.iter().filter(|t| **t >= day_start).count() as u32
    }

    /// The retry of an ignored check-in is due and has not lapsed.
    pub fn retry_ready(&self, now: DateTime<Utc>) -> bool {
        self.retry
            .is_some_and(|r| now >= r.due_at && now <= r.lapses_at)
    }

    pub fn on_fire_active(&self, now: DateTime<Utc>) -> bool {
        self.on_fire_until.is_some_and(|u| u > now)
    }

    pub fn decide(
        &self,
        cfg: &PolicyConfig,
        now: DateTime<Utc>,
        day_start: DateTime<Utc>,
        active_signals: usize,
    ) -> PolicyDecision {
        decide(&RuleInput {
            now,
            silent: self.silent,
            cooldown_until: self.cooldown_until,
            shown_today: self.shown_today(day_start),
            ceiling: self.effective_ceiling(),
            on_fire_until: self.on_fire_until,
            active_signals,
            min_active_signals: cfg.min_active_signals,
            retry_pending: self.retry_ready(now),
        })
    }

    /// A check-in surfaced. It counts toward the daily ceiling from this moment.
    pub fn record_shown(&mut self, now: DateTime<Utc>) {
        self.shown.push(now);
        let week_ago = now - Duration::days(7);
        self.shown.retain(|t| *t >= week_ago);
    }

    pub fn respond(&mut self, cfg: &PolicyConfig, now: DateTime<Utc>, response: Response) {
        let minutes = |m: i64| now + Duration::minutes(m);

        // Any answer, or a second ignore, settles the pending retry.
        if !matches!(response, Response::Ignored { is_retry: false }) {
            self.retry = None;
        }

        match response {
            Response::Continue => {
                self.extend_cooldown(minutes(cfg.cooldown_after_continue_minutes));
                self.refusal_streak += 1;
            }
            Response::LeaveMeAlone => {
                self.extend_cooldown(minutes(cfg.cooldown_after_leave_me_alone_minutes));
                self.refusal_streak += 1;
            }
            Response::DoNothing => {
                self.extend_cooldown(minutes(cfg.cooldown_after_do_nothing_minutes));
                self.refusal_streak += 1;
            }
            Response::AcceptedBreak => {
                self.extend_cooldown(minutes(cfg.cooldown_after_accepted_break_minutes));
                self.refusal_streak = 0;
            }
            Response::OnFire => self.activate_on_fire(cfg, now),
            Response::Ignored { is_retry: false } => {
                // Nothing may surface until the retry is due.
                let due_at = minutes(cfg.retry_delay_minutes);
                self.retry = Some(PendingRetry {
                    due_at,
                    lapses_at: due_at + Duration::minutes(cfg.retry_lapse_minutes),
                });
                self.extend_cooldown(due_at);
            }
            Response::Ignored { is_retry: true } => {
                self.extend_cooldown(minutes(cfg.cooldown_after_ignored_retry_minutes));
            }
        }
    }

    pub fn set_silent(&mut self, silent: bool) {
        self.silent = silent;
    }

    pub fn activate_on_fire(&mut self, cfg: &PolicyConfig, now: DateTime<Utc>) {
        self.on_fire_until = Some(now + Duration::minutes(cfg.on_fire_minutes));
    }

    pub fn clear_on_fire(&mut self) {
        self.on_fire_until = None;
    }

    /// After enough consecutive refusals the app asks whether it should speak up less.
    pub fn frequency_prompt_due(&self, cfg: &PolicyConfig) -> bool {
        self.refusal_streak >= cfg.frequency_prompt_refusals
    }

    /// Answer to "should interventions be less frequent?". Either way the question is settled.
    pub fn answer_frequency_prompt(&mut self, cfg: &PolicyConfig, reduce: bool) {
        if reduce {
            self.max_per_day = cfg.reduced_daily_limit.min(HARD_DAILY_CEILING);
        }
        self.refusal_streak = 0;
    }

    fn extend_cooldown(&mut self, until: DateTime<Utc>) {
        self.cooldown_until = Some(self.cooldown_until.map_or(until, |c| c.max(until)));
    }
}
