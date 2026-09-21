//! Interventions (IMPLEMENTATION.md §12): the check-in and its answers.
//!
//! - `model`     — steps and answers; the window's whole vocabulary
//! - `manager`   — the flow: when to show, what each answer does, timeouts
//! - `scheduler` — the one background loop (timeouts every second, evaluation every minute)
//! - `window`    — the dedicated Tauri window (a `Presenter`)

pub mod manager;
pub mod model;
pub mod scheduler;
pub mod window;
