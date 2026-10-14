//! The user's own chime sound: decoding, length limit, fades and level.
//!
//! A file is decoded once with rodio's [`Decoder`] (symphonia: mp3, wav, ogg/vorbis, flac, m4a/aac) into a
//! buffer on the audio thread, so no decoding runs on the device callback thread. At most
//! [`MAX_FILE_CHIME_DURATION`] is kept; a longer sound fades out over [`FILE_FADE_OUT`]. The buffer is
//! scaled to the packs' peak level, so a custom chime is about as loud as the built-in ones.

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, Decoder, Sample, SampleRate, Source};

use crate::chime::PEAK_LEVEL;
use crate::error::AudioError;
use crate::pcm::{Pcm, frames_to_duration};
use crate::synth::smoothstep;

/// A custom chime plays for at most this long.
pub const MAX_FILE_CHIME_DURATION: Duration = Duration::from_secs(10);

/// A custom chime cut at [`MAX_FILE_CHIME_DURATION`] fades out over this long.
pub const FILE_FADE_OUT: Duration = Duration::from_millis(300);

/// Short fades at both ends of every custom chime, so a file that starts or stops abruptly never clicks.
const EDGE_FADE: Duration = Duration::from_millis(5);

/// A quiet file is boosted by at most this factor (+12 dB) to reach the packs' peak level.
const MAX_GAIN: f32 = 4.0;

/// Upper bound on decoded samples (10 s of 192 kHz stereo), so an exotic format cannot use much memory.
const MAX_SAMPLES: usize = 10 * 192_000 * 2;

/// Checks that `path` is a sound file this player can play and returns its length.
///
/// The file is decoded up to [`MAX_FILE_CHIME_DURATION`], the part a chime plays. The result is the length
/// stated by the file when it states one, otherwise the decoded length (then at most
/// [`MAX_FILE_CHIME_DURATION`]). Fails with [`AudioError::File`] when the file cannot be read and with
/// [`AudioError::Decode`] when it is not a supported format, is damaged or contains no audio.
///
/// Blocking (it reads and decodes the file): call it off the UI thread, after checking the file size.
pub fn probe_file(path: &Path) -> Result<Duration, AudioError> {
    let decoded = decode(path)?;
    let played = frames_to_duration(decoded.frames(), decoded.sample_rate);
    Ok(decoded.stated_duration.unwrap_or(played))
}

/// Decodes `path` into a ready-to-play chime buffer (length limit, fades and level applied).
pub(crate) fn load(path: &Path) -> Result<Pcm, AudioError> {
    let mut decoded = decode(path)?;
    shape(
        &mut decoded.samples,
        decoded.channels,
        decoded.sample_rate,
        decoded.truncated,
    );
    Ok(Pcm {
        samples: Arc::from(decoded.samples),
        channels: decoded.channels,
        sample_rate: decoded.sample_rate,
    })
}

/// Raw decoded audio (at most [`MAX_FILE_CHIME_DURATION`]).
struct Decoded {
    samples: Vec<Sample>,
    channels: ChannelCount,
    sample_rate: SampleRate,
    /// The file has more audio than was kept.
    truncated: bool,
    /// Length stated by the file's headers, if any.
    stated_duration: Option<Duration>,
}

impl Decoded {
    fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.get())
    }
}

fn decode(path: &Path) -> Result<Decoded, AudioError> {
    let name = display_name(path);
    let file_error = |err: std::io::Error| AudioError::File(format!("{name}: {err}"));
    let mut file = File::open(path).map_err(file_error)?;
    let metadata = file.metadata().map_err(file_error)?;
    if !metadata.is_file() {
        return Err(AudioError::File(format!("{name}: not a file")));
    }
    check_wav_header(&mut file).map_err(|err| match err {
        HeaderError::Io(err) => file_error(err),
        HeaderError::Invalid(reason) => AudioError::Decode(format!("{name}: {reason}")),
    })?;
    let hint = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    let byte_len = metadata.len();

    // symphonia panics on some malformed input instead of returning an error. Contain that where panics
    // unwind (debug and test builds); release builds abort on panic, which the header check above and the
    // decoder's own validation make unlikely.
    let result = panic::catch_unwind(AssertUnwindSafe(move || -> Result<Decoded, String> {
        let mut builder = Decoder::builder()
            .with_data(BufReader::new(file))
            .with_byte_len(byte_len)
            .with_seekable(true);
        if let Some(hint) = &hint {
            builder = builder.with_hint(hint);
        }
        let decoder = builder.build().map_err(|err| err.to_string())?;
        Ok(read_samples(decoder))
    }));
    match result {
        Ok(Ok(decoded)) if decoded.samples.is_empty() => {
            Err(AudioError::Decode(format!("{name}: no audio")))
        }
        Ok(Ok(decoded)) => Ok(decoded),
        Ok(Err(reason)) => Err(AudioError::Decode(format!("{name}: {reason}"))),
        Err(_) => Err(AudioError::Decode(format!("{name}: the decoder failed"))),
    }
}

/// Reads at most [`MAX_FILE_CHIME_DURATION`] of samples. Stops early (as if cut) if the format changes
/// mid-stream, since the buffer has one format.
fn read_samples<R: Read + Seek>(mut decoder: Decoder<R>) -> Decoded {
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let stated_duration = decoder.total_duration();
    let frames = u64::from(sample_rate.get()).saturating_mul(MAX_FILE_CHIME_DURATION.as_secs());
    let limit = usize::try_from(frames)
        .unwrap_or(usize::MAX)
        .saturating_mul(usize::from(channels.get()))
        .min(MAX_SAMPLES);
    let mut samples = Vec::new();
    let mut truncated = false;
    while let Some(sample) = decoder.next() {
        if samples.len() >= limit
            || decoder.channels() != channels
            || decoder.sample_rate() != sample_rate
        {
            truncated = true;
            break;
        }
        samples.push(if sample.is_finite() { sample } else { 0.0 });
    }
    // Whole frames only.
    let channel_count = usize::from(channels.get());
    samples.truncate(samples.len() - samples.len() % channel_count);
    Decoded {
        samples,
        channels,
        sample_rate,
        truncated,
        stated_duration,
    }
}

/// Applies the fades and scales the buffer to [`PEAK_LEVEL`].
fn shape(samples: &mut [Sample], channels: ChannelCount, sample_rate: SampleRate, truncated: bool) {
    let channels = usize::from(channels.get());
    let frames = samples.len() / channels;
    let frames_in = |d: Duration| {
        let n = (d.as_secs_f64() * f64::from(sample_rate.get())).round() as usize;
        n.clamp(1, frames.max(1))
    };
    let fade_in = frames_in(EDGE_FADE);
    let fade_out = frames_in(if truncated { FILE_FADE_OUT } else { EDGE_FADE });
    for (frame, chunk) in samples.chunks_mut(channels).enumerate() {
        // Frame `frames - 1` is the last one and must reach exactly zero.
        let from_start = frame as f64 / fade_in as f64;
        let to_end = (frames - 1 - frame) as f64 / fade_out as f64;
        let gain = (smoothstep(from_start) * smoothstep(to_end)) as Sample;
        if gain < 1.0 {
            chunk.iter_mut().for_each(|s| *s *= gain);
        }
    }
    let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let gain = (PEAK_LEVEL / peak).min(MAX_GAIN);
        samples
            .iter_mut()
            .for_each(|s| *s = (*s * gain).clamp(-1.0, 1.0));
    }
}

/// File name without the folders (error messages never contain the user's folder names).
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sound file".to_owned())
}

enum HeaderError {
    Io(std::io::Error),
    Invalid(&'static str),
}

/// symphonia 0.5 panics on a WAV whose format chunk has a sample rate of 0 (it builds a `TimeBase` from
/// it). Reject such a file before decoding. Other files and WAVs whose format chunk is not near the start
/// are left to the decoder. Rewinds the file.
fn check_wav_header(file: &mut File) -> Result<(), HeaderError> {
    let mut head = Vec::with_capacity(4096);
    file.by_ref()
        .take(4096)
        .read_to_end(&mut head)
        .map_err(HeaderError::Io)?;
    file.rewind().map_err(HeaderError::Io)?;
    if head.len() < 12 || &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Ok(());
    }
    let mut pos = 12_usize;
    while let Some(chunk) = head.get(pos..pos.saturating_add(8)) {
        let size = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        let body = pos + 8;
        if &chunk[0..4] == b"fmt " {
            let Some(fmt) = head.get(body..body + 8) else {
                return Ok(());
            };
            let channels = u16::from_le_bytes([fmt[2], fmt[3]]);
            let rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
            return if channels == 0 || rate == 0 {
                Err(HeaderError::Invalid(
                    "invalid WAV format (no channels or sample rate)",
                ))
            } else {
                Ok(())
            };
        }
        // Chunks are padded to an even size.
        let size = usize::try_from(size).unwrap_or(usize::MAX);
        pos = body.saturating_add(size).saturating_add(size & 1);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_wav {
    //! Hand-written WAV files for tests (rodio's `wav_output` feature is not enabled).

    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh empty folder under the system temp folder; removed when dropped.
    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "focus-audio-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        pub fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A 16-bit PCM WAV with a format chunk claiming `rate` and `channels`.
    pub fn wav_bytes(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data_len = u32::try_from(samples.len() * 2).expect("small test file");
        let block_align = channels * 2;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&16_u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    /// `secs` of a 440 Hz sine at half scale, mono.
    pub fn sine(rate: u32, secs: f64) -> Vec<i16> {
        let count = (f64::from(rate) * secs).round() as usize;
        (0..count)
            .map(|n| {
                let t = n as f64 / f64::from(rate);
                ((std::f64::consts::TAU * 440.0 * t).sin() * 16_000.0) as i16
            })
            .collect()
    }

    pub fn write(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("write test file");
    }
}

#[cfg(test)]
mod tests {
    use super::test_wav::{TempDir, sine, wav_bytes, write};
    use super::*;

    fn peak(samples: &[Sample]) -> f32 {
        samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()))
    }

    #[test]
    fn probe_reports_the_length_of_a_short_wav() {
        let dir = TempDir::new();
        let path = dir.path("ding.wav");
        write(&path, &wav_bytes(8_000, 1, &sine(8_000, 0.5)));
        assert_close(
            probe_file(&path).expect("valid wav"),
            Duration::from_millis(500),
        );
    }

    fn assert_close(actual: Duration, expected: Duration) {
        assert!(
            actual.abs_diff(expected) < Duration::from_millis(1),
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn probe_reports_the_full_length_of_a_long_wav() {
        let dir = TempDir::new();
        let path = dir.path("long.WAV");
        write(&path, &wav_bytes(4_000, 1, &sine(4_000, 12.0)));
        assert_close(
            probe_file(&path).expect("valid wav"),
            Duration::from_secs(12),
        );
    }

    #[test]
    fn stereo_file_keeps_its_channels() {
        let dir = TempDir::new();
        let path = dir.path("stereo.wav");
        let mono = sine(8_000, 0.25);
        let stereo: Vec<i16> = mono.iter().flat_map(|s| [*s, s / 2]).collect();
        write(&path, &wav_bytes(8_000, 2, &stereo));
        let pcm = load(&path).expect("valid wav");
        assert_eq!(pcm.channels.get(), 2);
        assert_eq!(pcm.sample_rate.get(), 8_000);
        assert_eq!(pcm.samples.len(), stereo.len());
        assert_eq!(pcm.duration(), Duration::from_millis(250));
    }

    #[test]
    fn short_file_is_scaled_to_the_pack_level_without_clicks() {
        let dir = TempDir::new();
        let path = dir.path("short.wav");
        write(&path, &wav_bytes(8_000, 1, &sine(8_000, 1.0)));
        let pcm = load(&path).expect("valid wav");
        assert_eq!(pcm.samples.len(), 8_000);
        assert!((peak(&pcm.samples) - PEAK_LEVEL).abs() < 1e-3);
        assert_eq!(pcm.samples[0], 0.0);
        assert_eq!(pcm.samples[pcm.samples.len() - 1], 0.0);
        // The edge fades are short: 100 ms in, the sine is at full level again.
        assert!(peak(&pcm.samples[800..900]) > 0.9 * PEAK_LEVEL);
    }

    #[test]
    fn long_file_is_cut_at_the_limit_and_fades_out() {
        let dir = TempDir::new();
        let path = dir.path("long.wav");
        let rate = 4_000;
        write(&path, &wav_bytes(rate, 1, &sine(rate, 12.0)));
        let pcm = load(&path).expect("valid wav");
        assert_eq!(pcm.duration(), MAX_FILE_CHIME_DURATION);
        let samples = &pcm.samples;
        let fade = (FILE_FADE_OUT.as_secs_f64() * f64::from(rate)) as usize;
        let end = samples.len();
        // Full level before the fade, falling during it, silent at the very end.
        assert!(peak(&samples[end - 2 * fade..end - fade]) > 0.9 * PEAK_LEVEL);
        let mid = peak(&samples[end - fade / 2 - 20..end - fade / 2 + 20]);
        assert!(mid > 0.2 * PEAK_LEVEL && mid < 0.8 * PEAK_LEVEL, "{mid}");
        assert!(peak(&samples[end - 10..]) < 0.01);
        assert_eq!(samples[end - 1], 0.0);
    }

    #[test]
    fn quiet_file_gain_is_limited() {
        let dir = TempDir::new();
        let path = dir.path("quiet.wav");
        // Peak 160 / 32768 ≈ 0.005: reaching PEAK_LEVEL would need more than MAX_GAIN.
        let quiet: Vec<i16> = sine(8_000, 0.5).iter().map(|s| s / 100).collect();
        write(&path, &wav_bytes(8_000, 1, &quiet));
        let pcm = load(&path).expect("valid wav");
        let expected = 160.0 / 32_768.0 * MAX_GAIN;
        assert!(
            (peak(&pcm.samples) - expected).abs() < 1e-3,
            "{}",
            peak(&pcm.samples)
        );
    }

    #[test]
    fn silent_file_stays_silent() {
        let dir = TempDir::new();
        let path = dir.path("silence.wav");
        write(&path, &wav_bytes(8_000, 1, &[0; 4_000]));
        let pcm = load(&path).expect("silence is valid audio");
        assert!(pcm.samples.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn missing_file_is_a_file_error() {
        let dir = TempDir::new();
        let err = probe_file(&dir.path("missing.mp3")).expect_err("no file");
        assert!(matches!(err, AudioError::File(_)), "{err:?}");
        assert!(err.to_string().contains("missing.mp3"), "{err}");
        assert!(!err.to_string().contains("focus-audio-test"), "{err}");
    }

    #[test]
    fn folder_is_a_file_error() {
        let dir = TempDir::new();
        let folder = dir.path("chime.wav");
        std::fs::create_dir_all(&folder).expect("create folder");
        let err = probe_file(&folder).expect_err("a folder");
        assert!(matches!(err, AudioError::File(_)), "{err:?}");
    }

    #[test]
    fn garbage_is_a_decode_error() {
        let dir = TempDir::new();
        for (name, bytes) in [
            ("empty.mp3", Vec::new()),
            ("text.wav", b"definitely not a sound file".repeat(40)),
            (
                "noise.ogg",
                (0..5_000_u32).map(|i| (i * 7919 % 251) as u8).collect(),
            ),
        ] {
            let path = dir.path(name);
            write(&path, &bytes);
            let err = probe_file(&path).expect_err(name);
            assert!(matches!(err, AudioError::Decode(_)), "{name}: {err:?}");
        }
    }

    #[test]
    fn wav_without_samples_is_a_decode_error() {
        let dir = TempDir::new();
        let path = dir.path("empty.wav");
        write(&path, &wav_bytes(8_000, 1, &[]));
        let err = probe_file(&path).expect_err("no audio");
        assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
    }

    #[test]
    fn wav_with_zero_sample_rate_is_rejected_before_decoding() {
        let dir = TempDir::new();
        let path = dir.path("zero.wav");
        write(&path, &wav_bytes(0, 1, &sine(8_000, 0.1)));
        let err = probe_file(&path).expect_err("invalid header");
        assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
        assert!(err.to_string().contains("sample rate"), "{err}");
    }

    #[test]
    fn wav_header_check_skips_other_chunks() {
        let dir = TempDir::new();
        let path = dir.path("junk.wav");
        let mut bytes = wav_bytes(0, 1, &[0; 8]);
        // A padded odd-sized chunk before "fmt ".
        let junk = [b'J', b'U', b'N', b'K', 3, 0, 0, 0, 1, 2, 3, 0];
        bytes.splice(12..12, junk);
        write(&path, &bytes);
        let mut file = File::open(&path).expect("open");
        assert!(matches!(
            check_wav_header(&mut file),
            Err(HeaderError::Invalid(_))
        ));
        // The check rewinds the file for the decoder.
        assert_eq!(file.stream_position().expect("position"), 0);
    }
}
