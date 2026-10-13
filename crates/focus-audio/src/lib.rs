//! FocusMode audio: phase-change chimes played on a dedicated audio thread (rodio 0.22, ADR-003).
//!
//! Music is played by the WebView; this crate only plays the phase-change chimes
//! ([`Chime::BreakStart`], [`Chime::FocusStart`]) and the short beta reward sounds ([`Chime::REWARDS`], ADR-022,
//! synthesized per pack). A chime comes from a synthesized [`SoundPack`] (one per
//! app theme) or from the user's own sound file ([`ChimeSource::File`], checked beforehand with
//! [`probe_file`]). The thread owns the output device, opens it lazily on the first chime and re-opens it
//! after errors (ARCHITECTURE §8). Errors never panic or stop the session: they are logged and reported as
//! [`AudioEvent::Error`].
//!
//! ```no_run
//! use focus_audio::{Chime, ChimePlayer, ChimeSource, SoundPack};
//!
//! let (chimes, events) = ChimePlayer::spawn();
//! chimes.play(Chime::BreakStart, ChimeSource::Pack(SoundPack::Lunar), 0.7); // non-blocking
//! for event in events.try_iter() {
//!     eprintln!("{event:?}");
//! }
//! ```
//!
//! Reference for the rodio 0.22 API: `spikes/audio-spike`.

pub mod chime;
mod error;
mod file;
mod pcm;
mod player;
mod synth;

pub use chime::{Chime, ChimeSource, SoundPack};
pub use error::AudioError;
pub use file::{FILE_FADE_OUT, MAX_FILE_CHIME_DURATION, probe_file};
pub use player::{AudioEvent, ChimePlayer};
