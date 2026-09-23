//! Keeps persisted sessions in step with the stored events.

use super::model::Session;
use super::sessionizer::Sessionizer;
use crate::persistence::error::Result;
use crate::persistence::repositories::{activity_events, sessions};
use crate::persistence::{time, Database};
use crate::privacy::toggles::{self, PrivacyToggles};
use crate::sensors::service::SharedSnapshot;
use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, Mutex};

const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
const BEGINNING: &str = "1970-01-01T00:00:00Z";
const END_OF_TIME: &str = "9999-12-31T23:59:59Z";

pub struct SessionService {
    sessionizer: Sessionizer,
    db: Arc<Database>,
    snapshot: SharedSnapshot,
}

impl SessionService {
    pub fn new(sessionizer: Sessionizer, db: Arc<Database>, snapshot: SharedSnapshot) -> Self {
        Self {
            sessionizer,
            db,
            snapshot,
        }
    }

    /// Recomputes sessions from the stored events, persists them, and returns the
    /// session in progress, if any. Idempotent. Does nothing while the
    /// "Active time" toggle is off (or unreadable).
    pub fn refresh(&self, now: DateTime<Utc>) -> Result<Option<Session>> {
        let toggles = self
            .db
            .with_conn(toggles::load)
            .unwrap_or_else(|_| PrivacyToggles::all_off());
        if !toggles.active_time {
            return Ok(None);
        }

        let live_idle_since = self
            .snapshot
            .lock()
            .unwrap()
            .idle_since
            .as_deref()
            .and_then(|s| time::parse(s).ok());

        self.db.with_conn(|conn| {
            let events = activity_events::list_range(conn, BEGINNING, END_OF_TIME)?;
            let computed = self.sessionizer.run(&events, now, live_idle_since);

            // Sessions before the latest stored one are final. Events are pruned
            // sooner than sessions, so recomputing them would only truncate them.
            let frontier = sessions::latest_started_at(conn)?;
            for s in &computed {
                if frontier.as_ref().is_none_or(|f| s.started_at >= *f) {
                    sessions::upsert(conn, s)?;
                }
            }

            // An open session that can no longer be recomputed (its events were
            // pruned, or the app was off for days) is closed at its last refresh.
            for mut stale in sessions::list_open(conn)? {
                if computed.iter().all(|s| s.id != stale.id) {
                    let covered = stale.active_minutes + stale.idle_minutes;
                    let end = time::parse(&stale.started_at)?
                        + Duration::milliseconds((covered * 60_000.0) as i64);
                    stale.ended_at = Some(time::format(end));
                    sessions::upsert(conn, &stale)?;
                }
            }

            Ok(computed.into_iter().last().filter(|s| s.ended_at.is_none()))
        })
    }

    /// Refreshes on a background thread. A failed round is skipped; the next retries.
    pub fn spawn(service: Arc<Mutex<Self>>) {
        std::thread::Builder::new()
            .name("session-service".into())
            .spawn(move || loop {
                let _ = service.lock().unwrap().refresh(Utc::now());
                std::thread::sleep(REFRESH_INTERVAL);
            })
            .expect("spawn session thread");
    }
}
