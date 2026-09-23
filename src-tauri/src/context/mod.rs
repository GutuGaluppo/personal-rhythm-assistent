//! Context Engine v0.1: detects evidence, never judges (IMPLEMENTATION.md §10).
//!
//! It answers "what is true right now?" with discrete signals and the numbers
//! behind them. Whether the app may act on that is the Policy Engine's call
//! (Milestone 6); the two are deliberately separate.

pub mod detectors;
pub mod engine;
pub mod service;
pub mod signals;
