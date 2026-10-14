//! Audio errors.

/// Audio failures. During playback they are reported as [`crate::AudioEvent::Error`] and never stop the
/// session; [`crate::probe_file`] returns them directly.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no usable audio output device: {0}")]
    Device(String),
    #[error("audio output device failed: {0}")]
    DeviceLost(String),
    #[error("could not start the audio thread: {0}")]
    Thread(String),
    /// The sound file could not be opened or read (missing, not a file, no access).
    #[error("could not read the sound file: {0}")]
    File(String),
    /// The sound file is not a supported audio format, is damaged or contains no audio.
    #[error("unsupported or damaged sound file: {0}")]
    Decode(String),
}
