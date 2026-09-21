//! One pure function per signal. Each reads sessions (never raw events) and
//! returns the signal together with the evidence for it.

use super::engine::ContextConfig;
use super::signals::{Evidence, Signal, SignalKind, Unit};
use crate::persistence::time;
use crate::sessions::model::{Category, Session};
use chrono::{DateTime, Duration, Utc};

fn signal(
    kind: SignalKind,
    active: bool,
    measured: f64,
    threshold: f64,
    unit: Unit,
    text: String,
) -> Signal {
    Signal {
        kind,
        active,
        evidence: Evidence {
            measured,
            threshold,
            unit,
            explanation: text,
        },
    }
}

fn no_session(kind: SignalKind, threshold: f64, unit: Unit) -> Signal {
    signal(
        kind,
        false,
        0.0,
        threshold,
        unit,
        "No session in progress.".to_string(),
    )
}

/// Signal A. Active minutes exclude idle time.
pub fn continuous_activity(cfg: &ContextConfig, current: Option<&Session>) -> Signal {
    let kind = SignalKind::ContinuousActivity;
    let t = cfg.continuous_activity_minutes;
    let Some(s) = current else {
        return no_session(kind, t, Unit::Minutes);
    };
    let m = s.active_minutes;
    signal(
        kind,
        m >= t,
        m,
        t,
        Unit::Minutes,
        format!("Active for {} minutes in this session.", m.round()),
    )
}

/// Signal B. Only judged once the session is long enough for the ratio to mean something.
pub fn insufficient_idle(cfg: &ContextConfig, current: Option<&Session>) -> Signal {
    let kind = SignalKind::InsufficientIdle;
    let t = cfg.min_idle_ratio;
    let Some(s) = current else {
        return no_session(kind, t, Unit::Ratio);
    };

    let elapsed = s.active_minutes + s.idle_minutes;
    if elapsed < cfg.idle_check_min_session_minutes {
        return signal(
            kind,
            false,
            0.0,
            t,
            Unit::Ratio,
            format!(
                "Session is {} minutes; inactivity is checked after {}.",
                elapsed.round(),
                cfg.idle_check_min_session_minutes.round()
            ),
        );
    }
    let ratio = s.idle_minutes / elapsed;
    signal(
        kind,
        ratio < t,
        ratio,
        t,
        Unit::Ratio,
        format!(
            "{} minutes without input in a {}-minute session.",
            s.idle_minutes.round(),
            elapsed.round()
        ),
    )
}

/// Signal C. A rate, so it needs a minimum session length to avoid dividing by a few seconds.
pub fn rapid_switching(cfg: &ContextConfig, current: Option<&Session>) -> Signal {
    let kind = SignalKind::RapidSwitching;
    let t = cfg.rapid_switching_per_hour;
    let Some(s) = current else {
        return no_session(kind, t, Unit::PerHour);
    };

    let elapsed = s.active_minutes + s.idle_minutes;
    if elapsed < cfg.rapid_switching_min_session_minutes {
        return signal(
            kind,
            false,
            0.0,
            t,
            Unit::PerHour,
            format!(
                "Session is {} minutes; switching is checked after {}.",
                elapsed.round(),
                cfg.rapid_switching_min_session_minutes.round()
            ),
        );
    }
    let per_hour = f64::from(s.context_switches) / (elapsed / 60.0);
    signal(
        kind,
        per_hour >= t,
        per_hour,
        t,
        Unit::PerHour,
        format!(
            "{} app switches in {} minutes.",
            s.context_switches,
            elapsed.round()
        ),
    )
}

/// Signal D. Share of active time in `Create` over the last few days.
/// Everything counts in the denominator, including `Unknown`.
pub fn create_dominance(cfg: &ContextConfig, now: DateTime<Utc>, sessions: &[&Session]) -> Signal {
    let kind = SignalKind::CreateDominance;
    let t = cfg.create_dominance_share;
    let since = now - Duration::days(cfg.create_dominance_days);

    let (mut total, mut create) = (0.0, 0.0);
    for s in sessions {
        let Ok(start) = time::parse(&s.started_at) else {
            continue;
        };
        if start < since {
            continue;
        }
        total += s.active_minutes;
        if s.category == Category::Create {
            create += s.active_minutes;
        }
    }

    if total < cfg.create_dominance_min_active_minutes {
        return signal(
            kind,
            false,
            0.0,
            t,
            Unit::Ratio,
            format!(
                "{} minutes of activity in the last {} days; at least {} are needed to compare.",
                total.round(),
                cfg.create_dominance_days,
                cfg.create_dominance_min_active_minutes.round()
            ),
        );
    }
    let share = create / total;
    signal(
        kind,
        share >= t,
        share,
        t,
        Unit::Ratio,
        format!(
            "Create was {}% of active time over the last {} days.",
            (share * 100.0).round(),
            cfg.create_dominance_days
        ),
    )
}
