use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    /// Signal A: active for a long stretch.
    ContinuousActivity,
    /// Signal B: a long session with little inactivity in it.
    InsufficientIdle,
    /// Signal C: many app switches per hour.
    RapidSwitching,
    /// Signal D: Create takes most of the active time across days.
    CreateDominance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Minutes,
    /// 0.0 to 1.0.
    Ratio,
    PerHour,
}

/// The numbers behind a signal, plus a plain statement of them. The statement
/// only restates measurements; it never interprets them (no "overworking",
/// no "stress", no "should").
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub measured: f64,
    pub threshold: f64,
    pub unit: Unit,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub kind: SignalKind,
    pub active: bool,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextDecision {
    /// Not enough evidence to even consider surfacing anything.
    Observe,
    /// Enough independent signals to be a candidate. The Policy Engine decides what happens.
    CandidateIntervention,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextAssessment {
    pub assessed_at: String,
    pub signals: Vec<Signal>,
    pub active_signal_count: usize,
    pub decision: ContextDecision,
}
