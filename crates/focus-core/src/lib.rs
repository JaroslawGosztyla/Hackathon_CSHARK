//! FocusMode domain logic: timing profile, session state machine, config defaults and
//! validation, usage statistics and the distracting-site blocklist.
//!
//! Pure functional core (ADR-002): no Tauri, rodio, OS, network, clock or filesystem access.
//! Time is always passed in from the outside (`session::Now`).

pub mod api;
pub mod blocklist;
pub mod config;
pub mod session;
pub mod stats;
pub mod timing;
