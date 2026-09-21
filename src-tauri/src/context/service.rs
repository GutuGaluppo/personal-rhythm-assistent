use super::engine::{assess, ContextConfig, ContextInput};
use super::signals::{ContextAssessment, ContextDecision};
use crate::persistence::error::Result;
use crate::persistence::repositories::sessions;
use crate::persistence::{time, Database};
use crate::privacy::toggles::{self, PrivacyToggles};
use crate::sessions::service::SessionService;
use chrono::{DateTime, Duration, Utc};

/// Assesses the present moment from what is stored. With "Active time" off there
/// is nothing to assess, so the answer is simply "observe".
pub fn assess_now(
    db: &Database,
    session_service: &SessionService,
    cfg: &ContextConfig,
    now: DateTime<Utc>,
) -> Result<ContextAssessment> {
    let tracking = db
        .with_conn(toggles::load)
        .unwrap_or_else(|_| PrivacyToggles::all_off());
    if !tracking.active_time {
        return Ok(ContextAssessment {
            assessed_at: time::format(now),
            signals: vec![],
            active_signal_count: 0,
            decision: ContextDecision::Observe,
        });
    }

    let current = session_service.refresh(now)?;
    let from = time::format(now - Duration::days(cfg.create_dominance_days));
    let recent =
        db.with_conn(|conn| sessions::list_started_between(conn, &from, "9999-12-31T23:59:59Z"))?;

    Ok(assess(
        cfg,
        ContextInput {
            now,
            current_session: current.as_ref(),
            recent_sessions: &recent,
        },
    ))
}
