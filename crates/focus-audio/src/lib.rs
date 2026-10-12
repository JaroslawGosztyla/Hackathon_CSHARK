//! FocusMode audio: synthesized bell chimes played on a dedicated audio thread (rodio 0.22, ADR-003).
//!
//! Music is played by the WebView; this crate only plays the phase-change chimes
//! ([`Chime::BreakStart`], [`Chime::FocusStart`]). The thread owns the output device, opens it lazily on the
//! first chime and re-opens it after errors (ARCHITECTURE §8). Errors never panic or stop the session: they
//! are logged and reported as [`AudioEvent::Error`].
//!
//! ```no_run
//! use focus_audio::{Chime, ChimePlayer};
//!
//! let (chimes, events) = ChimePlayer::spawn();
//! chimes.play(Chime::BreakStart, 0.7); // non-blocking
//! for event in events.try_iter() {
//!     eprintln!("{event:?}");
//! }
//! ```
//!
//! Reference for the rodio 0.22 API: `spikes/audio-spike`.

pub mod chime;
mod player;

pub use chime::Chime;
pub use player::{AudioError, AudioEvent, ChimePlayer};
