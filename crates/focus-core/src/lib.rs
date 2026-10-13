//! FocusMode domain logic: timing profile, session state machine, config defaults and
//! validation, usage statistics, the distracting-site blocklist, the game-launcher catalog,
//! UI language resolution and the warm screen tint schedule.
//!
//! Pure functional core (ADR-002): no Tauri, rodio, OS, network, clock or filesystem access.
//! Time is always passed in from the outside (`session::Now`, `tint::tint_level`).

pub mod api;
pub mod blocklist;
pub mod config;
pub mod language;
pub mod launchers;
pub mod session;
pub mod stats;
pub mod timing;
pub mod tint;
