//! Interventions (IMPLEMENTATION.md §12): the check-in and its answers.
//!
//! - `model`            — steps and answers; the window's whole vocabulary
//! - `manager`          — the flow: when to show, what each answer does, timeouts
//! - `pause`            — Pause Mode: a deadline-based timer that outlives any window
//! - `presence_watcher` — brings the pause window forward when the user is back
//! - `scheduler`        — the one background loop (timeouts every second, evaluation every minute)
//! - `window`           — the dedicated Tauri window (a `Presenter`)

pub mod manager;
pub mod model;
pub mod pause;
pub mod presence_watcher;
pub mod scheduler;
pub mod window;
