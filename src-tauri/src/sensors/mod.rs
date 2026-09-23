//! Sensors emit neutral facts only (IMPLEMENTATION.md §7), never interpretations.
//!
//! Layering:
//! - `probe`      — what the OS can tell us (trait; macOS adapter in `platform::macos`)
//! - `active_app` — frontmost-app observations -> `active_application` / `application_switch`
//! - `idle`       — turns input-idle observations into `idle` events
//! - `collector`  — combines both, honouring the privacy toggles
//! - `service`    — the loop: read toggles, probe only what is enabled, persist, publish
//! - `bus`        — fan-out of events to in-process consumers (Sessionizer, UI)

pub mod active_app;
pub mod bus;
pub mod collector;
pub mod event;
pub mod idle;
pub mod probe;
pub mod service;
pub mod system_state;
