use super::manager::InterventionManager;
use crate::context::engine::ContextConfig;
use crate::context::service::assess_now;
use crate::persistence::Database;
use crate::reports::my_day::local_day_start;
use crate::sensors::service::SharedSnapshot;
use crate::sensors::system_state::ActivityState;
use crate::sessions::service::SessionService;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Timeouts are checked every second, evidence once a minute.
const EVALUATE_EVERY_TICKS: u64 = 60;

pub fn spawn(
    manager: Arc<InterventionManager>,
    db: Arc<Database>,
    sessions: Arc<Mutex<SessionService>>,
    snapshot: SharedSnapshot,
) {
    std::thread::Builder::new()
        .name("intervention-scheduler".into())
        .spawn(move || {
            let mut tick: u64 = 0;
            loop {
                std::thread::sleep(Duration::from_secs(1));
                tick += 1;
                let now = Utc::now();
                let _ = manager.expire(now);

                if tick % EVALUATE_EVERY_TICKS != 0 {
                    continue;
                }
                // Someone who has stepped away is not asked how they feel.
                let present = snapshot.lock().unwrap().state != ActivityState::Idle;
                let assessment = {
                    let service = sessions.lock().unwrap();
                    assess_now(&db, &service, &ContextConfig::default(), now)
                };
                if let Ok(assessment) = assessment {
                    let _ = manager.consider(now, local_day_start(now), &assessment, present);
                }
            }
        })
        .expect("spawn intervention scheduler");
}
