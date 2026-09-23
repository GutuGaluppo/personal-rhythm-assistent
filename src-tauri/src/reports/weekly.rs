//! Weekly review (IMPLEMENTATION.md §20): the last seven days, deterministically.
//!
//! No AI, no score, no target. Every observation is a fixed rule over stored
//! data and carries the numbers it came from, so it can be checked by hand.
//! It never judges, advises or compares the user with anyone.

use super::daily::{self, CategoryShare, DailySummary};
use crate::persistence::error::Result;
use crate::persistence::repositories::interventions::{self, Outcome};
use crate::persistence::repositories::sessions;
use crate::persistence::{time, Database};
use crate::sessions::model::{Category, Session};
use crate::sessions::service::SessionService;
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Utc};
use serde::Serialize;
use std::collections::BTreeSet;

pub const REFLECTIVE_QUESTION: &str = "Does anything here feel different from what you would like?";
pub const WINDOW_DAYS: i64 = 7;

/// Thresholds are inclusive ("at least"). Shares and ratios come from division, so a
/// change of exactly ten points can land a hair under 0.10; this keeps it in.
const EPSILON: f64 = 1e-9;

/// Thresholds for the observations. Tunable in Milestone 13.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeeklyConfig {
    /// A "long" session: at least this many active minutes.
    pub long_session_minutes: f64,
    /// Switching rates are only stated once there are this many active minutes...
    pub min_active_for_switching: f64,
    /// ...and a day only counts as "busiest" with this many.
    pub min_day_active_for_rate: f64,
    /// A category comparison needs this much activity in both weeks...
    pub min_week_active_for_comparison: f64,
    /// ...and a change of at least this share (0.10 = ten percentage points).
    pub min_share_change: f64,
    /// A session-length trend needs this many sessions in each half...
    pub min_sessions_per_half: usize,
    /// ...a change of at least this ratio...
    pub min_length_change_ratio: f64,
    /// ...and at least this many minutes.
    pub min_length_change_minutes: f64,
}

impl Default for WeeklyConfig {
    fn default() -> Self {
        Self {
            long_session_minutes: 90.0,
            min_active_for_switching: 120.0,
            min_day_active_for_rate: 60.0,
            min_week_active_for_comparison: 120.0,
            min_share_change: 0.10,
            min_sessions_per_half: 3,
            min_length_change_ratio: 0.20,
            min_length_change_minutes: 10.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDay {
    pub date: String,
    pub active_minutes: f64,
    pub longest_session_minutes: f64,
    pub context_switches: u32,
    /// Sessions that began this day.
    pub sessions: u32,
    pub average_session_minutes: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub count: u32,
    pub average_minutes: f64,
    pub longest_minutes: f64,
    pub long_session_threshold_minutes: f64,
    pub long_sessions: u32,
    /// Distinct days on which at least one long session began.
    pub long_session_days: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusiestDay {
    pub date: String,
    pub per_active_hour: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchingStats {
    pub total: u32,
    pub per_active_hour: f64,
    pub busiest_day: Option<BusiestDay>,
}

/// Every check-in shown lands in exactly one bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckInStats {
    pub shown: u32,
    /// Took a break (including a silent one).
    pub accepted: u32,
    /// Chose "Continue" or "Leave me alone".
    pub declined: u32,
    /// Dismissed, or timed out.
    pub ignored: u32,
    /// Chose "I'm on fire": neither an acceptance nor a refusal.
    pub on_fire: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// Stable key for the rule that produced it.
    pub id: String,
    pub text: String,
    /// The stored numbers the text was made from.
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReview {
    /// First and last local day of the window, "YYYY-MM-DD".
    pub from: String,
    pub to: String,
    pub active_minutes: f64,
    /// Oldest first; always seven entries.
    pub days: Vec<WeekDay>,
    pub category_distribution: Vec<CategoryShare>,
    pub sessions: SessionStats,
    pub switching: SwitchingStats,
    pub check_ins: CheckInStats,
    pub observations: Vec<Observation>,
    pub reflective_question: String,
}

pub struct Inputs<'a> {
    pub tz: FixedOffset,
    /// The seven days of the window, oldest first.
    pub days: &'a [DailySummary],
    /// The seven days before them, oldest first (may be empty or quiet).
    pub previous_days: &'a [DailySummary],
    /// Sessions that began inside the window.
    pub sessions: &'a [Session],
    pub outcomes: &'a [Outcome],
}

pub fn build(cfg: &WeeklyConfig, input: Inputs) -> WeeklyReview {
    let days = week_days(&input);
    let active: f64 = input.days.iter().map(|d| d.active_minutes).sum();
    let category_distribution = distribution(input.days);
    let sessions = session_stats(cfg, &input);
    let switching = switching_stats(cfg, &days, active);
    let check_ins = check_in_stats(input.outcomes);

    let observations = observations(
        cfg,
        &input,
        &days,
        active,
        &category_distribution,
        &sessions,
        &switching,
        &check_ins,
    );

    WeeklyReview {
        from: input
            .days
            .first()
            .map(|d| d.date.clone())
            .unwrap_or_default(),
        to: input
            .days
            .last()
            .map(|d| d.date.clone())
            .unwrap_or_default(),
        active_minutes: active,
        days,
        category_distribution,
        sessions,
        switching,
        check_ins,
        observations,
        reflective_question: REFLECTIVE_QUESTION.to_string(),
    }
}

// ---- figures -------------------------------------------------------------------------

fn session_day(s: &Session, tz: FixedOffset) -> Option<String> {
    time::parse(&s.started_at)
        .ok()
        .map(|t| t.with_timezone(&tz).format("%Y-%m-%d").to_string())
}

fn week_days(input: &Inputs) -> Vec<WeekDay> {
    input
        .days
        .iter()
        .map(|d| {
            let mine: Vec<f64> = input
                .sessions
                .iter()
                .filter(|s| session_day(s, input.tz).as_deref() == Some(d.date.as_str()))
                .map(|s| s.active_minutes)
                .collect();
            WeekDay {
                date: d.date.clone(),
                active_minutes: d.active_minutes,
                longest_session_minutes: d.longest_session_minutes,
                context_switches: d.context_switches,
                sessions: mine.len() as u32,
                average_session_minutes: (!mine.is_empty())
                    .then(|| mine.iter().sum::<f64>() / mine.len() as f64),
            }
        })
        .collect()
}

fn totals_by_category(days: &[DailySummary]) -> Vec<(Category, f64)> {
    let mut totals: Vec<(Category, f64)> = Vec::new();
    for day in days {
        for c in &day.category_distribution {
            match totals.iter_mut().find(|(k, _)| *k == c.category) {
                Some((_, m)) => *m += c.minutes,
                None => totals.push((c.category, c.minutes)),
            }
        }
    }
    totals
}

fn distribution(days: &[DailySummary]) -> Vec<CategoryShare> {
    let mut totals = totals_by_category(days);
    let all: f64 = totals.iter().map(|(_, m)| m).sum();
    totals.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.as_str().cmp(b.0.as_str())));
    totals
        .into_iter()
        .filter(|(_, m)| *m > 0.0)
        .map(|(category, minutes)| CategoryShare {
            category,
            minutes,
            share: minutes / all,
        })
        .collect()
}

fn share_of(days: &[DailySummary], category: Category) -> (f64, f64) {
    let totals = totals_by_category(days);
    let all: f64 = totals.iter().map(|(_, m)| m).sum();
    let mine = totals
        .iter()
        .find(|(c, _)| *c == category)
        .map_or(0.0, |(_, m)| *m);
    (mine, all)
}

fn session_stats(cfg: &WeeklyConfig, input: &Inputs) -> SessionStats {
    let lengths: Vec<f64> = input.sessions.iter().map(|s| s.active_minutes).collect();
    let long: Vec<&Session> = input
        .sessions
        .iter()
        .filter(|s| s.active_minutes >= cfg.long_session_minutes)
        .collect();
    let long_days: BTreeSet<String> = long
        .iter()
        .filter_map(|s| session_day(s, input.tz))
        .collect();
    SessionStats {
        count: lengths.len() as u32,
        average_minutes: if lengths.is_empty() {
            0.0
        } else {
            lengths.iter().sum::<f64>() / lengths.len() as f64
        },
        longest_minutes: lengths.iter().copied().fold(0.0, f64::max),
        long_session_threshold_minutes: cfg.long_session_minutes,
        long_sessions: long.len() as u32,
        long_session_days: long_days.len() as u32,
    }
}

fn switching_stats(cfg: &WeeklyConfig, days: &[WeekDay], active: f64) -> SwitchingStats {
    let total: u32 = days.iter().map(|d| d.context_switches).sum();
    let per_hour = |switches: u32, minutes: f64| f64::from(switches) / (minutes / 60.0);
    let busiest = days
        .iter()
        .filter(|d| d.active_minutes >= cfg.min_day_active_for_rate)
        .map(|d| BusiestDay {
            date: d.date.clone(),
            per_active_hour: per_hour(d.context_switches, d.active_minutes),
        })
        .max_by(|a, b| a.per_active_hour.total_cmp(&b.per_active_hour));
    SwitchingStats {
        total,
        per_active_hour: if active > 0.0 {
            per_hour(total, active)
        } else {
            0.0
        },
        busiest_day: busiest,
    }
}

const ACCEPTED: [&str; 4] = ["take_break", "move", "meditate", "do_nothing"];
const DECLINED: [&str; 2] = ["continue", "leave_me_alone"];

fn check_in_stats(outcomes: &[Outcome]) -> CheckInStats {
    let mut s = CheckInStats {
        shown: outcomes.len() as u32,
        accepted: 0,
        declined: 0,
        ignored: 0,
        on_fire: 0,
    };
    for o in outcomes {
        match o.action.as_deref() {
            Some(a) if ACCEPTED.contains(&a) => s.accepted += 1,
            Some(a) if DECLINED.contains(&a) => s.declined += 1,
            Some("on_fire") => s.on_fire += 1,
            // "ignored", or no recorded ending at all
            _ => s.ignored += 1,
        }
    }
    s
}

// ---- observations ------------------------------------------------------------------------

pub fn format_duration(minutes: f64) -> String {
    let total = minutes.max(0.0).round() as i64;
    let (h, m) = (total / 60, total % 60);
    match (h, m) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

fn percent(share: f64) -> String {
    format!("{}%", (share * 100.0).round())
}

fn weekday(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| d.format("%A").to_string())
        .unwrap_or_else(|_| date.to_string())
}

fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

#[allow(clippy::too_many_arguments)]
fn observations(
    cfg: &WeeklyConfig,
    input: &Inputs,
    days: &[WeekDay],
    active: f64,
    distribution: &[CategoryShare],
    sessions: &SessionStats,
    switching: &SwitchingStats,
    check_ins: &CheckInStats,
) -> Vec<Observation> {
    let mut out = Vec::new();
    let mut add = |id: &str, text: String, evidence: String| {
        out.push(Observation {
            id: id.into(),
            text,
            evidence,
        });
    };

    // 1. Where the time went.
    if let Some(top) = distribution.first() {
        add(
            "top_category",
            format!(
                "{} took the largest share of your active time.",
                top.category.as_str()
            ),
            format!(
                "{}: {} of {} active this week ({}).",
                top.category.as_str(),
                format_duration(top.minutes),
                format_duration(active),
                percent(top.share)
            ),
        );
    }

    // 2. The biggest shift against the week before, if both weeks have enough to compare.
    let prev_active: f64 = input.previous_days.iter().map(|d| d.active_minutes).sum();
    if active >= cfg.min_week_active_for_comparison
        && prev_active >= cfg.min_week_active_for_comparison
    {
        let mut categories: BTreeSet<&'static str> =
            distribution.iter().map(|c| c.category.as_str()).collect();
        categories.extend(
            totals_by_category(input.previous_days)
                .iter()
                .map(|(c, _)| c.as_str()),
        );
        // The largest change wins. Equal changes (two categories trading places) go to
        // the one that grew, then to the name, so the answer never depends on hash or
        // insertion order. Shares are compared to nine decimals to ignore float noise.
        let key = |now: f64, was: f64| ((now - was).abs() * 1e9).round() as i64;
        let biggest = categories
            .into_iter()
            .filter_map(Category::parse)
            .map(|c| {
                let (now_min, now_all) = share_of(input.days, c);
                let (was_min, was_all) = share_of(input.previous_days, c);
                (c, now_min / now_all, was_min / was_all)
            })
            .max_by(|a, b| {
                key(a.1, a.2)
                    .cmp(&key(b.1, b.2))
                    .then((a.1 - a.2).total_cmp(&(b.1 - b.2)))
                    .then(b.0.as_str().cmp(a.0.as_str()))
            });
        if let Some((category, now, was)) = biggest {
            if (now - was).abs() + EPSILON >= cfg.min_share_change {
                let more = if now > was { "more" } else { "less" };
                add(
                    "category_change",
                    format!("{} occupied {more} space this week.", category.as_str()),
                    format!(
                        "{}: {} of active time this week, {} the week before.",
                        category.as_str(),
                        percent(now),
                        percent(was)
                    ),
                );
            }
        }
    }

    // 3. The longest session.
    if sessions.count > 0 {
        add(
            "longest_session",
            format!(
                "Your longest session was {}.",
                format_duration(sessions.longest_minutes)
            ),
            format!(
                "Longest of {}; average {}.",
                plural(sessions.count, "session", "sessions"),
                format_duration(sessions.average_minutes)
            ),
        );
    }

    // 4. Long sessions.
    if sessions.long_sessions > 0 {
        add(
            "long_sessions",
            format!(
                "{} lasted {} or more, across {}.",
                plural(sessions.long_sessions, "session", "sessions"),
                format_duration(sessions.long_session_threshold_minutes),
                plural(sessions.long_session_days, "day", "days")
            ),
            format!(
                "Counted from active time; {} in total this week.",
                plural(sessions.count, "session", "sessions")
            ),
        );
    }

    // 5. App switching.
    if active >= cfg.min_active_for_switching {
        let mut text = format!(
            "You switched apps about {} times per active hour.",
            switching.per_active_hour.round()
        );
        if let Some(b) = &switching.busiest_day {
            text.push_str(&format!(
                " The busiest day was {} (about {}).",
                weekday(&b.date),
                b.per_active_hour.round()
            ));
        }
        add(
            "switching",
            text,
            format!(
                "{} switches in {} of active time.",
                switching.total,
                format_duration(active)
            ),
        );
    }

    // 6. Session length, earlier in the week against later. The middle day is left out
    //    so the two halves are three days each.
    let half = |range: std::ops::Range<usize>| -> Vec<f64> {
        let dates: BTreeSet<&str> = days[range].iter().map(|d| d.date.as_str()).collect();
        input
            .sessions
            .iter()
            .filter(|s| session_day(s, input.tz).is_some_and(|d| dates.contains(d.as_str())))
            .map(|s| s.active_minutes)
            .collect()
    };
    if days.len() == WINDOW_DAYS as usize {
        let (early, late) = (half(0..3), half(4..7));
        if early.len() >= cfg.min_sessions_per_half && late.len() >= cfg.min_sessions_per_half {
            let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
            let (a, b) = (mean(&early), mean(&late));
            if (b - a).abs() + EPSILON >= cfg.min_length_change_minutes
                && (b - a).abs() / a.max(1.0) + EPSILON >= cfg.min_length_change_ratio
            {
                let which = if b > a { "later" } else { "earlier" };
                add(
                    "session_length_trend",
                    format!("Your sessions were longer in the {which} part of the week."),
                    format!(
                        "Average {} across {} in the first three days, {} across {} in the last three.",
                        format_duration(a),
                        plural(early.len() as u32, "session", "sessions"),
                        format_duration(b),
                        plural(late.len() as u32, "session", "sessions"),
                    ),
                );
            }
        }
    }

    // 7. Check-ins.
    if check_ins.shown == 0 {
        add(
            "check_ins",
            "There were no check-ins this week.".into(),
            "0 check-ins shown.".into(),
        );
    } else {
        add(
            "check_ins",
            format!(
                "You accepted {} of {} break suggestions.",
                check_ins.accepted,
                check_ins.shown
            ),
            format!(
                "{} shown: {} accepted, {} declined, {} dismissed or timed out, {} \"I'm on fire\".",
                check_ins.shown, check_ins.accepted, check_ins.declined, check_ins.ignored, check_ins.on_fire
            ),
        );
    }

    out
}

// ---- from the database ----------------------------------------------------------------------

/// The review for the seven local days ending today.
pub fn review(
    db: &Database,
    session_service: &SessionService,
    cfg: &WeeklyConfig,
    now: DateTime<Utc>,
    tz: FixedOffset,
) -> Result<WeeklyReview> {
    let today = daily::today(now, tz);
    let date_of = |back: i64| today - Duration::days(back);

    // Oldest first: back = 6 .. 0 for this week, 13 .. 7 for the one before.
    let summaries = |range: std::ops::RangeInclusive<i64>| -> Result<Vec<DailySummary>> {
        range
            .rev()
            .map(|back| daily::summary_for(db, session_service, now, tz, date_of(back)))
            .collect()
    };
    let days = summaries(0..=6)?;
    let previous_days = summaries(7..=13)?;

    let (window_start, _) = daily::day_bounds(date_of(6), tz);
    let (_, window_end) = daily::day_bounds(today, tz);
    let (from, to) = (time::format(window_start), time::format(window_end));

    let (sessions, outcomes) = db.with_conn(|c| {
        let started_inside = sessions::list_started_between(c, &from, &to)?;
        let outcomes = interventions::outcomes_between(c, &from, &to)?;
        Ok((started_inside, outcomes))
    })?;

    Ok(build(
        cfg,
        Inputs {
            tz,
            days: &days,
            previous_days: &previous_days,
            sessions: &sessions,
            outcomes: &outcomes,
        },
    ))
}
