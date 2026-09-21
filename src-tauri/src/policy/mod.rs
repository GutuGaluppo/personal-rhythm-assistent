//! Policy Engine v0.1: decides whether a candidate intervention may surface
//! (IMPLEMENTATION.md §11). The Context Engine detects; this decides.
//! Never merged with the Context Engine.
//!
//! - `rules`   — configuration and the pure decision function
//! - `engine`  — the state (cooldown, ceiling, silence, on-fire, retry) and how responses change it
//! - `service` — persistence and thread-safe access to that state

pub mod engine;
pub mod rules;
pub mod service;
