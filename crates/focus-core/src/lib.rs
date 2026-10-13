//! FocusMode domain logic: timing profile, session state machine (incl. meeting holds and the session
//! goal), config defaults and validation (incl. per-break reminders, custom chime file names, calendars
//! and developer tools), meetings against planned breaks and their reminders, usage statistics (incl.
//! Windows shell surfaces left out of the chart), the distracting-site blocklist, the game-launcher
//! catalog, UI language resolution, the warm screen tint (schedule, colour temperature, gains) and the
//! ranking of work items for "Co dalej?" (assigned first, then waiting on the user, then notifications).
//!
//! Pure functional core (ADR-002): no Tauri, rodio, OS, network, clock or filesystem access.
//! Time is always passed in from the outside (`session::Now`, `tint::tint_level`, `calendar::*`,
//! `work::rank`).

pub mod api;
pub mod blocklist;
pub mod calendar;
pub mod config;
pub mod language;
pub mod launchers;
pub mod session;
pub mod stats;
pub mod timing;
pub mod tint;
pub mod work;
