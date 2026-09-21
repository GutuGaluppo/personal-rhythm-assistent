//! Local SQLite persistence (Milestone 1). Everything stays on this machine.

pub mod db;
pub mod error;
pub mod migrations;
pub mod repositories;
pub mod time;

pub use db::Database;
pub use error::PersistenceError;
