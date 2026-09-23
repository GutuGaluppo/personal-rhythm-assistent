use super::engine::{PolicyState, Response};
use super::rules::{PolicyConfig, PolicyDecision};
use crate::persistence::error::Result;
use crate::persistence::repositories::settings;
use crate::persistence::Database;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::{Arc, Mutex};

const SETTINGS_KEY: &str = "policy_state";

/// What the UI may show about the policy. Also serves the developer view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyView {
    pub silent: bool,
    pub on_fire_until: Option<DateTime<Utc>>,
    pub cooldown_until: Option<DateTime<Utc>>,
    pub shown_today: u32,
    pub daily_limit: u32,
    pub retry_pending: bool,
    pub frequency_prompt_due: bool,
}

/// Loads, mutates and saves the policy state one operation at a time, so the
/// menu bar, commands and the scheduler cannot overwrite each other.
pub struct PolicyService {
    db: Arc<Database>,
    cfg: PolicyConfig,
    lock: Mutex<()>,
}

impl PolicyService {
    pub fn new(db: Arc<Database>, cfg: PolicyConfig) -> Self {
        Self {
            db,
            cfg,
            lock: Mutex::new(()),
        }
    }

    pub fn config(&self) -> &PolicyConfig {
        &self.cfg
    }

    pub fn state(&self) -> Result<PolicyState> {
        let _guard = self.lock.lock().unwrap();
        self.load()
    }

    fn load(&self) -> Result<PolicyState> {
        Ok(self
            .db
            .with_conn(|c| settings::get_json(c, SETTINGS_KEY))?
            .unwrap_or_default())
    }

    fn update<T>(&self, f: impl FnOnce(&mut PolicyState) -> T) -> Result<T> {
        let _guard = self.lock.lock().unwrap();
        let mut state = self.load()?;
        let out = f(&mut state);
        self.db
            .with_conn(|c| settings::set_json(c, SETTINGS_KEY, &state))?;
        Ok(out)
    }

    pub fn decide(
        &self,
        now: DateTime<Utc>,
        day_start: DateTime<Utc>,
        active_signals: usize,
    ) -> Result<PolicyDecision> {
        Ok(self
            .state()?
            .decide(&self.cfg, now, day_start, active_signals))
    }

    pub fn record_shown(&self, now: DateTime<Utc>) -> Result<()> {
        self.update(|s| s.record_shown(now))
    }

    pub fn respond(&self, now: DateTime<Utc>, response: Response) -> Result<()> {
        self.update(|s| s.respond(&self.cfg, now, response))
    }

    pub fn set_silent(&self, silent: bool) -> Result<()> {
        self.update(|s| s.set_silent(silent))
    }

    pub fn set_on_fire(&self, on: bool, now: DateTime<Utc>) -> Result<()> {
        self.update(|s| {
            if on {
                s.activate_on_fire(&self.cfg, now)
            } else {
                s.clear_on_fire()
            }
        })
    }

    /// Switches "I'm on fire" on if it is off and off if it is on, and returns the
    /// new state. The menu bar and the keyboard shortcut both come through here.
    pub fn toggle_on_fire(&self, now: DateTime<Utc>, day_start: DateTime<Utc>) -> Result<bool> {
        let on = self.view(now, day_start)?.on_fire_until.is_none();
        self.set_on_fire(on, now)?;
        Ok(on)
    }

    pub fn answer_frequency_prompt(&self, reduce: bool) -> Result<()> {
        self.update(|s| s.answer_frequency_prompt(&self.cfg, reduce))
    }

    pub fn view(&self, now: DateTime<Utc>, day_start: DateTime<Utc>) -> Result<PolicyView> {
        let s = self.state()?;
        Ok(PolicyView {
            silent: s.silent,
            on_fire_until: s.on_fire_until.filter(|u| *u > now),
            cooldown_until: s.cooldown_until.filter(|u| *u > now),
            shown_today: s.shown_today(day_start),
            daily_limit: s.effective_ceiling(),
            retry_pending: s.retry_ready(now),
            frequency_prompt_due: s.frequency_prompt_due(&self.cfg),
        })
    }
}
