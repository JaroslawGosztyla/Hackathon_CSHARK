//! Additive synthesis engine for the chime packs.
//!
//! Pure and deterministic, with no device access. A [`Score`] is a list of [`Note`]s played on one
//! [`Instrument`]. Every waveform is a sum of sines below the Nyquist frequency, so nothing aliases.
//! Every note starts at zero (attack envelope) and the whole score ends with a smooth release, so a chime
//! never clicks. The result is scaled to an exact peak, so every pack plays at a similar level.

use std::f64::consts::{PI, TAU};

use rodio::Sample;

/// One sine component of a struck tone.
pub(crate) struct Partial {
    /// Frequency as a multiple of the note's fundamental.
    pub ratio: f64,
    /// Relative amplitude.
    pub amp: f64,
    /// Time constant of the exponential decay (higher partials usually die out faster).
    pub decay_s: f64,
}

/// Struck or plucked tone (bell, glass, marimba, plain sine): decaying sine partials. A struck note
/// rings until the end of the score.
pub(crate) struct Struck {
    pub partials: &'static [Partial],
    /// Linear attack of each note (avoids a click at the onset).
    pub attack_s: f64,
    /// Each partial also sounds `shimmer_hz` higher at `shimmer_mix`, so it beats slowly (0 = off).
    pub shimmer_hz: f64,
    pub shimmer_mix: f64,
}

/// Sustained chord voice: a chorus of detuned band-limited saws plus a pure sine.
pub(crate) struct Pad {
    /// Smooth fade-in of each note.
    pub attack_s: f64,
    /// Smooth fade-out after the note's `hold_s`.
    pub release_s: f64,
    /// Unison saw voices, in cents from the note.
    pub detune_cents: &'static [f64],
    /// Slow pitch drift of every voice (chorus movement): depth in cents and base rate in Hz.
    pub drift_cents: f64,
    pub drift_hz: f64,
    /// Saw harmonics are rolled off above this frequency (keeps the pad warm).
    pub brightness_hz: f64,
    /// No saw harmonic above this frequency.
    pub max_harmonic_hz: f64,
    /// Level of the sine layer relative to the saws.
    pub sine_mix: f64,
}

/// Band-limited pulse wave (the 8-bit "square" family) with a pluck-like envelope.
pub(crate) struct Pulse {
    /// Fraction of the period the wave is high (0.5 = square).
    pub duty: f64,
    /// Linear fade-in of each note.
    pub attack_s: f64,
    /// Linear fade-out after the note's `hold_s`.
    pub release_s: f64,
    /// After the attack the level falls from 1 to `sustain` with this time constant.
    pub decay_s: f64,
    pub sustain: f64,
    /// No harmonic above this frequency.
    pub max_harmonic_hz: f64,
}

#[derive(Clone, Copy)]
pub(crate) enum Instrument {
    Struck(&'static Struck),
    Pad(&'static Pad),
    Pulse(&'static Pulse),
}

/// One note of a score.
pub(crate) struct Note {
    /// Onset from the start of the chime.
    pub at_s: f64,
    pub freq_hz: f64,
    pub gain: f64,
    /// How long a pad or pulse note is held before its release. Struck notes ignore it.
    pub hold_s: f64,
}

/// A complete chime.
pub(crate) struct Score {
    pub len_ms: u64,
    /// Smooth fade over the end of the chime, so a still-ringing note never clicks when the buffer ends.
    pub release_s: f64,
    pub instrument: Instrument,
    pub notes: &'static [Note],
}

impl Score {
    pub fn len_s(&self) -> f64 {
        self.len_ms as f64 / 1000.0
    }
}

impl Instrument {
    /// Time from a note's onset until it reaches full level.
    #[cfg(test)]
    pub fn attack_s(self) -> f64 {
        match self {
            Instrument::Struck(s) => s.attack_s,
            Instrument::Pad(p) => p.attack_s,
            Instrument::Pulse(p) => p.attack_s,
        }
    }

    /// Value of `note` `local_s` seconds after its onset, before the note's gain.
    fn value(self, note: &Note, local_s: f64, nyquist: f64) -> f64 {
        if local_s < 0.0 {
            return 0.0;
        }
        match self {
            Instrument::Struck(s) => s.value(note.freq_hz, local_s, nyquist),
            Instrument::Pad(p) => p.value(note, local_s, nyquist),
            Instrument::Pulse(p) => p.value(note, local_s, nyquist),
        }
    }
}

impl Struck {
    fn value(&self, freq_hz: f64, t: f64, nyquist: f64) -> f64 {
        let attack = (t / self.attack_s).min(1.0);
        let shimmer = self.shimmer_mix > 0.0 && self.shimmer_hz > 0.0;
        let partials: f64 = self
            .partials
            .iter()
            .map(|p| (p, freq_hz * p.ratio))
            .filter(|&(_, freq)| freq < nyquist)
            .map(|(p, freq)| {
                let mut tone = (TAU * freq * t).sin();
                let beat = freq + self.shimmer_hz;
                if shimmer && beat < nyquist {
                    tone = (tone + self.shimmer_mix * (TAU * beat * t).sin())
                        / (1.0 + self.shimmer_mix);
                }
                p.amp * (-t / p.decay_s).exp() * tone
            })
            .sum();
        attack * partials
    }
}

impl Pad {
    fn value(&self, note: &Note, t: f64, nyquist: f64) -> f64 {
        let env = smoothstep(t / self.attack_s)
            * smoothstep((note.hold_s + self.release_s - t) / self.release_s);
        if env <= 0.0 || self.detune_cents.is_empty() {
            return 0.0;
        }
        let depth = cents_to_ratio(self.drift_cents) - 1.0;
        let saws: f64 = self
            .detune_cents
            .iter()
            .enumerate()
            .map(|(i, cents)| {
                let freq = note.freq_hz * cents_to_ratio(*cents);
                // Every voice drifts at its own rate, so the chorus never lines up. Phase of a sine-modulated
                // frequency f·(1 + d·sin(2π·r·t)) is 2π·f·t + f·d·(1 − cos(2π·r·t)) / r.
                let rate = self.drift_hz * (1.0 + 0.37 * i as f64);
                let phase = TAU * freq * t + freq * depth * (1.0 - (TAU * rate * t).cos()) / rate;
                let top = self.max_harmonic_hz.min(nyquist);
                let harmonics = harmonic_count(freq, top);
                sine_series(phase, harmonics, |k| {
                    let f = freq * f64::from(k);
                    1.0 / (f64::from(k) * (1.0 + (f / self.brightness_hz).powi(2)))
                })
            })
            .sum::<f64>()
            / self.detune_cents.len() as f64;
        let sine = if note.freq_hz < nyquist {
            (TAU * note.freq_hz * t).sin()
        } else {
            0.0
        };
        env * (saws + self.sine_mix * sine) / (1.0 + self.sine_mix)
    }
}

impl Pulse {
    fn value(&self, note: &Note, t: f64, nyquist: f64) -> f64 {
        let attack = (t / self.attack_s).min(1.0);
        let decay = self.sustain
            + (1.0 - self.sustain) * (-(t - self.attack_s).max(0.0) / self.decay_s).exp();
        let release = ((note.hold_s + self.release_s - t) / self.release_s).clamp(0.0, 1.0);
        let env = attack * decay * release;
        if env <= 0.0 {
            return 0.0;
        }
        // Fourier series of a zero-mean pulse: Σ (2 / kπ)·sin(kπ·duty)·cos(k·ωt). sin(kπ·duty) follows the
        // same recurrence as the harmonics, so no trigonometry runs per harmonic.
        let step = PI * self.duty;
        let twice_cos_step = 2.0 * step.cos();
        let (mut prev, mut cur) = (0.0, step.sin());
        let harmonics = harmonic_count(note.freq_hz, self.max_harmonic_hz.min(nyquist));
        let wave = cosine_series(TAU * note.freq_hz * t, harmonics, |k| {
            let duty_term = cur;
            let next = twice_cos_step * cur - prev;
            prev = cur;
            cur = next;
            2.0 * duty_term / (f64::from(k) * PI)
        });
        env * wave
    }
}

/// Number of harmonics of `freq_hz` strictly below `limit_hz`.
fn harmonic_count(freq_hz: f64, limit_hz: f64) -> u32 {
    if freq_hz <= 0.0 || limit_hz <= freq_hz {
        return 0;
    }
    let count = (limit_hz / freq_hz).ceil() - 1.0;
    count.clamp(0.0, 256.0) as u32
}

/// Σ_{k=1..=n} weight(k)·sin(k·x), using the Chebyshev recurrence (one sin and one cos per call).
/// `weight` is called in order k = 1, 2, …, n.
fn sine_series(x: f64, n: u32, mut weight: impl FnMut(u32) -> f64) -> f64 {
    let twice_cos = 2.0 * x.cos();
    let (mut prev, mut cur) = (0.0, x.sin());
    let mut sum = 0.0;
    for k in 1..=n {
        sum += weight(k) * cur;
        let next = twice_cos * cur - prev;
        prev = cur;
        cur = next;
    }
    sum
}

/// Σ_{k=1..=n} weight(k)·cos(k·x), like [`sine_series`].
fn cosine_series(x: f64, n: u32, mut weight: impl FnMut(u32) -> f64) -> f64 {
    let cos = x.cos();
    let twice_cos = 2.0 * cos;
    let (mut prev, mut cur) = (1.0, cos);
    let mut sum = 0.0;
    for k in 1..=n {
        sum += weight(k) * cur;
        let next = twice_cos * cur - prev;
        prev = cur;
        cur = next;
    }
    sum
}

fn cents_to_ratio(cents: f64) -> f64 {
    (cents / 1200.0).exp2()
}

/// Smooth 0..1 ramp (input clamped to 0..=1).
pub(crate) fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Number of samples of a `len_ms` chime at `rate_hz`.
pub(crate) fn sample_count(len_ms: u64, rate_hz: u32) -> usize {
    let count = u64::from(rate_hz).saturating_mul(len_ms) / 1000;
    usize::try_from(count).unwrap_or(usize::MAX)
}

/// Renders `score` as mono samples at `rate_hz`, scaled so the largest sample is exactly `peak` (silence
/// stays silent). Every sample is finite and within [-1, 1].
pub(crate) fn render(score: &Score, rate_hz: u32, peak: f64) -> Vec<Sample> {
    if rate_hz == 0 {
        return Vec::new();
    }
    let rate = f64::from(rate_hz);
    let nyquist = rate / 2.0;
    let len_s = score.len_s();
    let raw: Vec<f64> = (0..sample_count(score.len_ms, rate_hz))
        .map(|n| {
            let t = n as f64 / rate;
            let notes: f64 = score
                .notes
                .iter()
                .map(|note| note.gain * score.instrument.value(note, t - note.at_s, nyquist))
                .sum();
            notes * smoothstep((len_s - t) / score.release_s)
        })
        .collect();
    let max = raw.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let scale = if max.is_finite() && max > 0.0 {
        peak.clamp(0.0, 1.0) / max
    } else {
        0.0
    };
    raw.into_iter()
        .map(|v| {
            let v = v * scale;
            if v.is_finite() {
                v.clamp(-1.0, 1.0) as Sample
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_match_direct_sums() {
        let x = 1.234_f64;
        let direct_sin: f64 = (1..=12)
            .map(|k| (f64::from(k) * x).sin() / f64::from(k))
            .sum();
        let direct_cos: f64 = (1..=12).map(|k| (f64::from(k) * x).cos() * 0.5).sum();
        assert!((sine_series(x, 12, |k| 1.0 / f64::from(k)) - direct_sin).abs() < 1e-9);
        assert!((cosine_series(x, 12, |_| 0.5) - direct_cos).abs() < 1e-9);
        assert_eq!(sine_series(x, 0, |_| 1.0), 0.0);
    }

    #[test]
    fn harmonics_stay_below_the_limit() {
        assert_eq!(harmonic_count(1_000.0, 4_000.0), 3);
        assert_eq!(harmonic_count(1_000.0, 4_000.5), 4);
        assert_eq!(harmonic_count(1_000.0, 900.0), 0);
        assert_eq!(harmonic_count(0.0, 900.0), 0);
        assert_eq!(harmonic_count(1.0, f64::MAX), 256);
    }

    #[test]
    fn pulse_series_has_the_duty_cycle() {
        // A 25 % pulse is high a quarter of the period: its mean over the high part is positive and over the
        // low part negative, and the time-average is zero.
        let pulse = Pulse {
            duty: 0.25,
            attack_s: 1e-9,
            release_s: 1.0,
            decay_s: 1.0,
            sustain: 1.0,
            max_harmonic_hz: 20_000.0,
        };
        let note = Note {
            at_s: 0.0,
            freq_hz: 100.0,
            gain: 1.0,
            hold_s: 10.0,
        };
        let period = 0.01;
        let steps = 400;
        let values: Vec<f64> = (0..steps)
            .map(|i| {
                pulse.value(
                    &note,
                    1.0 + period * f64::from(i) / f64::from(steps),
                    22_050.0,
                )
            })
            .collect();
        let mean: f64 = values.iter().sum::<f64>() / f64::from(steps);
        assert!(mean.abs() < 0.02, "mean {mean}");
        // The pulse is centred on phase 0: high around the start/end of the period, low in the middle.
        assert!(values[0] > 0.5, "{}", values[0]);
        assert!(
            values[steps as usize / 2] < -0.1,
            "{}",
            values[steps as usize / 2]
        );
    }

    #[test]
    fn zero_rate_renders_nothing() {
        const NOTES: &[Note] = &[Note {
            at_s: 0.0,
            freq_hz: 440.0,
            gain: 1.0,
            hold_s: 0.0,
        }];
        const SINE: Struck = Struck {
            partials: &[Partial {
                ratio: 1.0,
                amp: 1.0,
                decay_s: 1.0,
            }],
            attack_s: 0.01,
            shimmer_hz: 0.0,
            shimmer_mix: 0.0,
        };
        let score = Score {
            len_ms: 100,
            release_s: 0.05,
            instrument: Instrument::Struck(&SINE),
            notes: NOTES,
        };
        assert!(render(&score, 0, 0.5).is_empty());
        let samples = render(&score, 8_000, 0.5);
        assert_eq!(samples.len(), 800);
        let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.5).abs() < 1e-6, "{peak}");
    }

    #[test]
    fn silent_score_stays_silent() {
        let score = Score {
            len_ms: 50,
            release_s: 0.01,
            instrument: Instrument::Pulse(&Pulse {
                duty: 0.5,
                attack_s: 0.001,
                release_s: 0.001,
                decay_s: 0.1,
                sustain: 0.5,
                max_harmonic_hz: 10_000.0,
            }),
            notes: &[],
        };
        assert!(render(&score, 8_000, 0.7).iter().all(|s| *s == 0.0));
    }
}
