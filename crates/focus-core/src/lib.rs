//! FocusMode domain logic: timing profile, session state machine (incl. meeting holds), config
//! defaults and validation (incl. per-break reminders, custom chime file names and calendars),
//! meetings against planned breaks and their reminders, usage statistics (incl. Windows shell
//! surfaces left out of the chart), the distracting-site blocklist, the game-launcher catalog,
//! UI language resolution and the warm screen tint (schedule, colour temperature, gains).
//!
//! Pure functional core (ADR-002): no Tauri, rodio, OS, network, clock or filesystem access.
//! Time is always passed in from the outside (`session::Now`, `tint::tint_level`, `calendar::*`).

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
