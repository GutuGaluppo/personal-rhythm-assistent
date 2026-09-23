use super::bus::EventBus;
use super::collector::Collector;
use super::probe::Probe;
use super::system_state::SensorSnapshot;
use crate::persistence::error::Result;
use crate::persistence::repositories::activity_events;
use crate::persistence::Database;
use crate::privacy::toggles::{self, PrivacyToggles};
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const TICK_INTERVAL: Duration = Duration::from_secs(2);

pub type SharedSnapshot = Arc<Mutex<SensorSnapshot>>;

pub struct SensorService {
    probe: Box<dyn Probe>,
    collector: Collector,
    db: Arc<Database>,
    bus: Arc<EventBus>,
    snapshot: SharedSnapshot,
}

impl SensorService {
    pub fn new(
        probe: Box<dyn Probe>,
        collector: Collector,
        db: Arc<Database>,
        bus: Arc<EventBus>,
        snapshot: SharedSnapshot,
    ) -> Self {
        Self {
            probe,
            collector,
            db,
            bus,
            snapshot,
        }
    }

    /// One collection round. Toggles are re-read every time, so switching a
    /// sensor off takes effect on the very next tick, and a sensor that is off
    /// is never probed. If the toggles cannot be read, everything stays off.
    /// Returns the number of events recorded.
    pub fn tick(&mut self, now: DateTime<Utc>) -> Result<usize> {
        let toggles = self
            .db
            .with_conn(toggles::load)
            .unwrap_or_else(|_| PrivacyToggles::all_off());

        let frontmost = toggles
            .active_application
            .then(|| self.probe.frontmost_app())
            .flatten();
        let idle_secs = toggles.idle_detection.then(|| self.probe.idle_seconds());

        let events = self.collector.tick(now, &toggles, frontmost, idle_secs);
        *self.snapshot.lock().unwrap() = self.collector.snapshot();

        for event in &events {
            self.db
                .with_conn(|conn| activity_events::insert(conn, event))?;
            self.bus.publish(event);
        }
        Ok(events.len())
    }

    /// Runs `tick` forever on a background thread.
    pub fn spawn(mut self) {
        std::thread::Builder::new()
            .name("sensor-service".into())
            .spawn(move || loop {
                // A failed round is skipped, not fatal; the next tick retries.
                let _ = self.tick(Utc::now());
                std::thread::sleep(TICK_INTERVAL);
            })
            .expect("spawn sensor thread");
    }
}
