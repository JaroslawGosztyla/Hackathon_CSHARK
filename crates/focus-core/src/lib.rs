//! FocusMode domain logic: timing profile, session state machine, break planner,
//! config validation, stats and tint math.
//!
//! Pure functional core (ADR-002): no Tauri, rodio, OS, network or filesystem dependencies.
//! Time, exercise catalog and random seed are always passed in from the outside.
