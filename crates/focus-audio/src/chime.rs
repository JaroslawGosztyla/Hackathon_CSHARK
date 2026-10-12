//! Synthesized bell chimes.
//!
//! Pure sample generation without any device access, so it is fully unit-tested. Every chime is two soft
//! bell strikes. A strike is a sum of sine partials (frequency ratios 1, 2.0, 2.76 and 5.4) with a 5 ms
//! attack and a per-partial exponential decay. The whole chime ends with a short release, so it never
//! clicks when the buffer runs out.

use std::f64::consts::TAU;
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

/// Which chime to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chime {
    /// Focus ended, break begins: descending soft bell (E5 -> C5).
    BreakStart,
    /// Break ended or the session started, focus begins: ascending soft bell (C5 -> G5).
    FocusStart,
}

impl Chime {
    /// Every chime kind.
    pub const ALL: [Chime; 2] = [Chime::BreakStart, Chime::FocusStart];
}

/// Sample rate of rendered chimes (mono). The output mixer resamples to the device rate.
pub const SAMPLE_RATE_HZ: u32 = 44_100;

/// [`SAMPLE_RATE_HZ`] as rodio's non-zero type.
pub const SAMPLE_RATE: SampleRate = match SampleRate::new(SAMPLE_RATE_HZ) {
    Some(rate) => rate,
    // Evaluated at compile time: it can never trigger at runtime.
    None => panic!("SAMPLE_RATE_HZ must be non-zero"),
};

/// Chimes are rendered as a single channel.
const MONO: ChannelCount = ChannelCount::MIN;

const CHIME_LEN_MS: u64 = 2_200;

/// Length of every chime.
pub const CHIME_DURATION: Duration = Duration::from_millis(CHIME_LEN_MS);

const CHIME_LEN_S: f64 = CHIME_LEN_MS as f64 / 1000.0;
/// Start of the second strike, relative to the start of the chime.
const SECOND_STRIKE_S: f64 = 0.45;
/// Linear attack of each strike (avoids a click at the onset).
const ATTACK_S: f64 = 0.005;
/// Smooth fade at the very end of the chime.
const RELEASE_S: f64 = 0.3;
/// Output level before the user's chime volume. The theoretical peak is `MASTER_GAIN` times the sum of the
/// strike gains (at most 1.9), so the output always stays below 0.95.
const MASTER_GAIN: f64 = 0.5;

const C5_HZ: f64 = 523.2511;
const E5_HZ: f64 = 659.2551;
const G5_HZ: f64 = 783.9909;

/// One sine component of a bell strike.
struct Partial {
    /// Frequency as a multiple of the strike's fundamental.
    ratio: f64,
    /// Relative amplitude.
    amp: f64,
    /// Time constant of the exponential decay (higher partials die out faster).
    decay_s: f64,
}

const PARTIALS: [Partial; 4] = [
    Partial {
        ratio: 1.0,
        amp: 1.0,
        decay_s: 0.8,
    },
    Partial {
        ratio: 2.0,
        amp: 0.4,
        decay_s: 0.5,
    },
    Partial {
        ratio: 2.76,
        amp: 0.25,
        decay_s: 0.32,
    },
    Partial {
        ratio: 5.4,
        amp: 0.1,
        decay_s: 0.15,
    },
];

/// One strike of the bell inside a chime.
struct Strike {
    at_s: f64,
    freq_hz: f64,
    gain: f64,
}

fn strikes(chime: Chime) -> [Strike; 2] {
    match chime {
        Chime::BreakStart => [
            Strike {
                at_s: 0.0,
                freq_hz: E5_HZ,
                gain: 1.0,
            },
            Strike {
                at_s: SECOND_STRIKE_S,
                freq_hz: C5_HZ,
                gain: 0.9,
            },
        ],
        Chime::FocusStart => [
            Strike {
                at_s: 0.0,
                freq_hz: C5_HZ,
                gain: 0.85,
            },
            Strike {
                at_s: SECOND_STRIKE_S,
                freq_hz: G5_HZ,
                gain: 1.0,
            },
        ],
    }
}

/// Number of samples of one chime at `sample_rate_hz`.
pub fn sample_count(sample_rate_hz: u32) -> usize {
    let count = u64::from(sample_rate_hz).saturating_mul(CHIME_LEN_MS) / 1000;
    usize::try_from(count).unwrap_or(usize::MAX)
}

/// Renders `chime` as mono samples at [`SAMPLE_RATE_HZ`]. Every sample is finite and within [-1, 1].
pub fn render(chime: Chime) -> Vec<Sample> {
    render_at(chime, SAMPLE_RATE_HZ)
}

fn render_at(chime: Chime, sample_rate_hz: u32) -> Vec<Sample> {
    if sample_rate_hz == 0 {
        return Vec::new();
    }
    let rate = f64::from(sample_rate_hz);
    let nyquist = rate / 2.0;
    let strikes = strikes(chime);
    let norm: f64 = PARTIALS.iter().map(|p| p.amp).sum();
    let gain = MASTER_GAIN / norm;
    (0..sample_count(sample_rate_hz))
        .map(|n| {
            let t = n as f64 / rate;
            let bells: f64 = strikes
                .iter()
                .map(|s| s.gain * strike_value(s, t - s.at_s, nyquist))
                .sum();
            let release = smoothstep((CHIME_LEN_S - t) / RELEASE_S);
            (bells * gain * release).clamp(-1.0, 1.0) as Sample
        })
        .collect()
}

/// Value of one strike `local_s` seconds after it was struck (0 before that).
fn strike_value(strike: &Strike, local_s: f64, nyquist: f64) -> f64 {
    if local_s < 0.0 {
        return 0.0;
    }
    let attack = (local_s / ATTACK_S).min(1.0);
    let partials: f64 = PARTIALS
        .iter()
        .map(|p| (p, strike.freq_hz * p.ratio))
        .filter(|&(_, freq)| freq < nyquist)
        .map(|(p, freq)| p.amp * (-local_s / p.decay_s).exp() * (TAU * freq * local_s).sin())
        .sum();
    attack * partials
}

/// Smooth 0..1 ramp (input clamped to 0..=1).
fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A rodio [`Source`] over a pre-rendered mono buffer.
///
/// The buffer is shared (`Arc`), so starting a chime does not copy or re-render it, and no synthesis runs on
/// the device callback thread.
#[derive(Debug, Clone)]
pub struct PcmSource {
    samples: Arc<[Sample]>,
    pos: usize,
    sample_rate: SampleRate,
}

impl PcmSource {
    /// Plays `samples` (mono) at `sample_rate`.
    pub fn new(samples: Arc<[Sample]>, sample_rate: SampleRate) -> Self {
        Self {
            samples,
            pos: 0,
            sample_rate,
        }
    }
}

impl Iterator for PcmSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.samples.get(self.pos).copied()?;
        self.pos += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.samples.len().saturating_sub(self.pos);
        (left, Some(left))
    }
}

impl ExactSizeIterator for PcmSource {}

impl Source for PcmSource {
    fn current_span_len(&self) -> Option<usize> {
        // The format never changes, so the whole buffer is one span ("until the sound ends").
        None
    }

    fn channels(&self) -> ChannelCount {
        MONO
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let len = u64::try_from(self.samples.len()).unwrap_or(u64::MAX);
        let nanos = len.saturating_mul(1_000_000_000) / u64::from(self.sample_rate.get());
        Some(Duration::from_nanos(nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Power of `samples[from_s..to_s]` at `freq_hz` (single DFT bin at an arbitrary frequency).
    fn power_at(samples: &[Sample], from_s: f64, to_s: f64, freq_hz: f64) -> f64 {
        let rate = f64::from(SAMPLE_RATE_HZ);
        let from = (from_s * rate) as usize;
        let to = (to_s * rate) as usize;
        let (mut re, mut im) = (0.0_f64, 0.0_f64);
        for (n, s) in samples[from..to].iter().enumerate() {
            let phase = TAU * freq_hz * n as f64 / rate;
            re += f64::from(*s) * phase.cos();
            im += f64::from(*s) * phase.sin();
        }
        re * re + im * im
    }

    fn peak(samples: &[Sample]) -> f64 {
        samples
            .iter()
            .map(|s| f64::from(*s).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn chime_has_expected_length() {
        assert_eq!(sample_count(SAMPLE_RATE_HZ), 97_020);
        for chime in Chime::ALL {
            assert_eq!(
                render(chime).len(),
                sample_count(SAMPLE_RATE_HZ),
                "{chime:?}"
            );
        }
    }

    #[test]
    fn samples_are_finite_and_in_range() {
        for chime in Chime::ALL {
            for (n, s) in render(chime).iter().enumerate() {
                assert!(
                    s.is_finite() && (-1.0..=1.0).contains(s),
                    "{chime:?}: sample {n} out of range: {s}"
                );
            }
        }
    }

    #[test]
    fn chime_is_audible_with_headroom() {
        for chime in Chime::ALL {
            let p = peak(&render(chime));
            assert!(p > 0.2, "{chime:?} too quiet: peak {p}");
            assert!(p < 0.95, "{chime:?} too loud: peak {p}");
        }
    }

    #[test]
    fn chime_starts_and_ends_silently() {
        let tail = (f64::from(SAMPLE_RATE_HZ) * 0.01) as usize;
        for chime in Chime::ALL {
            let samples = render(chime);
            assert!(
                f64::from(samples[0]).abs() < 1e-9,
                "{chime:?} starts with a click"
            );
            let end = peak(&samples[samples.len() - tail..]);
            assert!(end < 0.01, "{chime:?} ends with a click: {end}");
        }
    }

    #[test]
    fn break_chime_descends() {
        let samples = render(Chime::BreakStart);
        // First strike: E5 dominates, C5 is silent.
        assert!(power_at(&samples, 0.0, 0.4, E5_HZ) > 100.0 * power_at(&samples, 0.0, 0.4, C5_HZ));
        // Second strike: C5 is louder than the decaying E5.
        assert!(power_at(&samples, 0.5, 0.9, C5_HZ) > 1.5 * power_at(&samples, 0.5, 0.9, E5_HZ));
    }

    #[test]
    fn focus_chime_ascends() {
        let samples = render(Chime::FocusStart);
        assert!(power_at(&samples, 0.0, 0.4, C5_HZ) > 100.0 * power_at(&samples, 0.0, 0.4, G5_HZ));
        assert!(power_at(&samples, 0.5, 0.9, G5_HZ) > 1.5 * power_at(&samples, 0.5, 0.9, C5_HZ));
    }

    #[test]
    fn chimes_differ() {
        assert_ne!(render(Chime::BreakStart), render(Chime::FocusStart));
    }

    #[test]
    fn zero_sample_rate_renders_nothing() {
        assert!(render_at(Chime::FocusStart, 0).is_empty());
        assert_eq!(sample_count(0), 0);
    }

    #[test]
    fn low_sample_rate_drops_partials_above_nyquist() {
        // 8 kHz: the 5.4x partial of G5 (~4.2 kHz) would alias; it must be skipped, output stays valid.
        let samples = render_at(Chime::FocusStart, 8_000);
        assert_eq!(samples.len(), sample_count(8_000));
        assert!(
            samples
                .iter()
                .all(|s| s.is_finite() && (-1.0..=1.0).contains(s))
        );
    }

    #[test]
    fn pcm_source_plays_buffer_once() {
        let pcm: Arc<[Sample]> = render(Chime::BreakStart).into();
        let mut source = PcmSource::new(Arc::clone(&pcm), SAMPLE_RATE);
        assert_eq!(source.channels().get(), 1);
        assert_eq!(source.sample_rate().get(), SAMPLE_RATE_HZ);
        assert_eq!(source.current_span_len(), None);
        assert_eq!(source.total_duration(), Some(CHIME_DURATION));
        assert_eq!(source.len(), pcm.len());
        assert_eq!(source.next(), Some(pcm[0]));
        assert_eq!(source.len(), pcm.len() - 1);
        let rest: Vec<Sample> = source.by_ref().collect();
        assert_eq!(rest.as_slice(), &pcm[1..]);
        assert_eq!(source.next(), None);
        assert_eq!(source.len(), 0);
    }

    #[test]
    fn empty_pcm_source_ends_immediately() {
        let mut source = PcmSource::new(Arc::from(Vec::<Sample>::new()), SAMPLE_RATE);
        assert_eq!(source.next(), None);
        assert_eq!(source.total_duration(), Some(Duration::ZERO));
    }
}
