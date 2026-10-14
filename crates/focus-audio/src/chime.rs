//! Chime kinds and the synthesized sound packs that match the app themes.
//!
//! Pure sample generation without any device access, so it is fully unit-tested. Every pack has a
//! [`Chime::BreakStart`] variant that moves down in pitch and a [`Chime::FocusStart`] variant that moves up.
//! Each lasts 1 to 2.5 s, starts and ends silently, and is scaled to the same peak ([`PEAK_LEVEL`]).
//!
//! The beta rewards (ADR-022) add three short sounds per pack ([`Chime::REWARDS`]), synthesized with the pack's own
//! instrument (no licensed audio): [`Chime::Pop`] (a soft high blip, a step ticked), [`Chime::Sparkle`] (a rising
//! triad and a high note) and [`Chime::Fanfare`] (a rising arpeggio and a held chord). They rise in pitch, last
//! 0.12 to 1.4 s, start and end silently and have the same peak.

use std::path::PathBuf;
use std::time::Duration;

use rodio::{Sample, SampleRate};

use crate::synth::{self, Instrument, Note, Pad, Partial, Pulse, Score, Struck};

/// Which chime to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chime {
    /// Focus ended, break begins: the pitch moves down.
    BreakStart,
    /// Break ended or the session started, focus begins: the pitch moves up.
    FocusStart,
    /// Beta reward (ADR-022), ~0.15 s: one soft high blip (a step ticked).
    Pop,
    /// Beta reward, ~0.7 s: a rising triad and a high note (daily goal, milestone, comeback, pearl).
    Sparkle,
    /// Beta reward, ~1.2 s: a rising arpeggio and a held chord (level up, golden pearl).
    Fanfare,
}

impl Chime {
    /// Every phase chime (the ones played at phase changes).
    pub const ALL: [Chime; 2] = [Chime::BreakStart, Chime::FocusStart];
    /// The beta reward sounds (ADR-022).
    pub const REWARDS: [Chime; 3] = [Chime::Pop, Chime::Sparkle, Chime::Fanfare];
}

/// Synthesized chime sounds. Midnight, Vaporwave, Lunar and Autumn match the themes of the same name and
/// Chiptune the 8-bit theme; Classic and Soft are neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SoundPack {
    /// Soft two-strike bell (the original chime).
    #[default]
    Midnight,
    /// Detuned saw and sine chords with a slow attack and a subtle chorus.
    Vaporwave,
    /// Glassy high partials with a long, shimmering decay.
    Lunar,
    /// Wooden marimba: fast decay with the bar's 4x overtone.
    Autumn,
    /// Two-tone "ding-dong".
    Classic,
    /// Gentle sine tones with a long fade.
    Soft,
    /// 8-bit square-wave arpeggio.
    Chiptune,
}

impl SoundPack {
    /// Every pack.
    pub const ALL: [SoundPack; 7] = [
        SoundPack::Midnight,
        SoundPack::Vaporwave,
        SoundPack::Lunar,
        SoundPack::Autumn,
        SoundPack::Classic,
        SoundPack::Soft,
        SoundPack::Chiptune,
    ];

    /// Length of `chime` in this pack (phase chimes 1 to 2.5 s, reward sounds 0.12 to 1.4 s).
    pub fn duration(self, chime: Chime) -> Duration {
        Duration::from_millis(score(self, chime).len_ms)
    }
}

/// What a chime plays: a built-in pack or the user's own sound file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChimeSource {
    /// A synthesized pack.
    Pack(SoundPack),
    /// An audio file (mp3, wav, ogg, flac or m4a). It plays for at most
    /// [`MAX_FILE_CHIME_DURATION`](crate::MAX_FILE_CHIME_DURATION) and falls back to [`SoundPack::Midnight`]
    /// (plus an [`AudioEvent::Error`](crate::AudioEvent::Error)) when it cannot be read or decoded.
    File(PathBuf),
}

impl Default for ChimeSource {
    fn default() -> Self {
        ChimeSource::Pack(SoundPack::default())
    }
}

impl From<SoundPack> for ChimeSource {
    fn from(pack: SoundPack) -> Self {
        ChimeSource::Pack(pack)
    }
}

/// Sample rate of rendered chimes (mono). The output mixer resamples to the device rate.
pub const SAMPLE_RATE_HZ: u32 = 44_100;

/// [`SAMPLE_RATE_HZ`] as rodio's non-zero type.
pub const SAMPLE_RATE: SampleRate = match SampleRate::new(SAMPLE_RATE_HZ) {
    Some(rate) => rate,
    // Evaluated at compile time: it can never trigger at runtime.
    None => panic!("SAMPLE_RATE_HZ must be non-zero"),
};

/// Largest sample of every rendered chime, before the user's chime volume. The same peak for every pack
/// keeps them at a similar level; it is the level of the original bell and leaves headroom for the mixer.
pub const PEAK_LEVEL: f32 = 0.55;

/// Renders `chime` from `pack` as mono samples at [`SAMPLE_RATE_HZ`]. Every sample is finite and within
/// [-1, 1]; the largest one is [`PEAK_LEVEL`].
pub fn render(chime: Chime, pack: SoundPack) -> Vec<Sample> {
    synth::render(&score(pack, chime), SAMPLE_RATE_HZ, f64::from(PEAK_LEVEL))
}

// ---------------------------------------------------------------------------------------------------------
// Notes (equal temperament, A4 = 440 Hz)
// ---------------------------------------------------------------------------------------------------------

const D4: f64 = 293.6648;
const E4: f64 = 329.6276;
const F4: f64 = 349.2282;
const A4: f64 = 440.0;
const C5: f64 = 523.2511;
const CS5: f64 = 554.3653;
const D5: f64 = 587.3295;
const E5: f64 = 659.2551;
const F5: f64 = 698.4565;
const FS5: f64 = 739.9888;
const G5: f64 = 783.9909;
const A5: f64 = 880.0;
const B5: f64 = 987.7666;
const C6: f64 = 1_046.502;
const D6: f64 = 1_174.659;
const E6: f64 = 1_318.510;
const F6: f64 = 1_396.913;
const G6: f64 = 1_567.982;

/// A struck note (rings until the end of the chime).
const fn hit(at_s: f64, freq_hz: f64, gain: f64) -> Note {
    Note {
        at_s,
        freq_hz,
        gain,
        hold_s: 0.0,
    }
}

/// A held note (pads and pulses release after `hold_s`).
const fn held(at_s: f64, freq_hz: f64, gain: f64, hold_s: f64) -> Note {
    Note {
        at_s,
        freq_hz,
        gain,
        hold_s,
    }
}

// ---------------------------------------------------------------------------------------------------------
// Instruments
// ---------------------------------------------------------------------------------------------------------

const fn partial(ratio: f64, amp: f64, decay_s: f64) -> Partial {
    Partial {
        ratio,
        amp,
        decay_s,
    }
}

/// Soft bell: partials 1, 2, 2.76 and 5.4 (the original chime).
const BELL: Struck = Struck {
    partials: &[
        partial(1.0, 1.0, 0.8),
        partial(2.0, 0.4, 0.5),
        partial(2.76, 0.25, 0.32),
        partial(5.4, 0.1, 0.15),
    ],
    attack_s: 0.005,
    shimmer_hz: 0.0,
    shimmer_mix: 0.0,
};

/// Glass: strong, slowly decaying inharmonic upper partials (free-bar modes), each beating slowly against
/// a twin a few hertz higher.
const GLASS: Struck = Struck {
    partials: &[
        partial(1.0, 1.0, 1.7),
        partial(2.76, 0.5, 1.0),
        partial(5.40, 0.3, 0.55),
        partial(8.93, 0.15, 0.3),
    ],
    attack_s: 0.006,
    shimmer_hz: 3.1,
    shimmer_mix: 0.45,
};

/// Marimba: the bar is tuned so its first overtone is two octaves up (4x); both die out quickly.
const MARIMBA: Struck = Struck {
    partials: &[
        partial(1.0, 1.0, 0.42),
        partial(4.0, 0.32, 0.07),
        partial(9.9, 0.08, 0.025),
    ],
    attack_s: 0.002,
    shimmer_hz: 0.0,
    shimmer_mix: 0.0,
};

/// Door chime bar: harmonic partials, long ring.
const DOOR_CHIME: Struck = Struck {
    partials: &[
        partial(1.0, 1.0, 1.1),
        partial(2.0, 0.3, 0.6),
        partial(3.0, 0.12, 0.35),
    ],
    attack_s: 0.003,
    shimmer_hz: 0.0,
    shimmer_mix: 0.0,
};

/// Almost pure sine with a slow attack.
const SINE: Struck = Struck {
    partials: &[partial(1.0, 1.0, 1.2), partial(2.0, 0.06, 0.6)],
    attack_s: 0.06,
    shimmer_hz: 0.0,
    shimmer_mix: 0.0,
};

/// Warm synth pad: three detuned saws (the chorus) over a sine.
const SYNTH_PAD: Pad = Pad {
    attack_s: 0.28,
    release_s: 0.45,
    detune_cents: &[-9.0, 0.0, 8.0],
    drift_cents: 4.0,
    drift_hz: 0.45,
    brightness_hz: 1_600.0,
    max_harmonic_hz: 7_000.0,
    sine_mix: 0.6,
};

/// 8-bit lead: square wave with a short pluck.
const SQUARE: Pulse = Pulse {
    duty: 0.5,
    attack_s: 0.004,
    release_s: 0.008,
    decay_s: 0.12,
    sustain: 0.35,
    max_harmonic_hz: 12_000.0,
};

// ---------------------------------------------------------------------------------------------------------
// Scores
// ---------------------------------------------------------------------------------------------------------

// Two bell strikes: E5 -> C5 / C5 -> G5.
const MIDNIGHT_BREAK: &[Note] = &[hit(0.0, E5, 1.0), hit(0.45, C5, 0.9)];
const MIDNIGHT_FOCUS: &[Note] = &[hit(0.0, C5, 0.85), hit(0.45, G5, 1.0)];

// Two chords that cross-fade: Fmaj7 -> Dm7 / Dm7 -> Fmaj7.
const VAPORWAVE_BREAK: &[Note] = &[
    held(0.0, F4, 0.8, 0.95),
    held(0.0, A4, 0.8, 0.95),
    held(0.0, C5, 0.85, 0.95),
    held(0.0, E5, 1.0, 0.95),
    held(0.85, D4, 0.8, 1.1),
    held(0.85, F4, 0.8, 1.1),
    held(0.85, A4, 0.85, 1.1),
    held(0.85, C5, 1.0, 1.1),
];
const VAPORWAVE_FOCUS: &[Note] = &[
    held(0.0, D4, 0.8, 0.95),
    held(0.0, F4, 0.8, 0.95),
    held(0.0, A4, 0.85, 0.95),
    held(0.0, C5, 1.0, 0.95),
    held(0.85, F4, 0.8, 1.1),
    held(0.85, A4, 0.8, 1.1),
    held(0.85, C5, 0.85, 1.1),
    held(0.85, E5, 1.0, 1.1),
];

// Three glass pings of an E minor triad, left to ring out: E6 B5 G5 / G5 B5 E6.
const LUNAR_BREAK: &[Note] = &[hit(0.0, E6, 0.9), hit(0.32, B5, 0.95), hit(0.64, G5, 1.0)];
const LUNAR_FOCUS: &[Note] = &[hit(0.0, G5, 0.9), hit(0.32, B5, 0.95), hit(0.64, E6, 1.0)];

// F major arpeggio with a small ritardando: F5 C5 A4 F4 / F4 A4 C5 F5.
const AUTUMN_BREAK: &[Note] = &[
    hit(0.0, F5, 0.8),
    hit(0.13, C5, 0.85),
    hit(0.26, A4, 0.9),
    hit(0.45, F4, 1.0),
];
const AUTUMN_FOCUS: &[Note] = &[
    hit(0.0, F4, 0.8),
    hit(0.13, A4, 0.85),
    hit(0.26, C5, 0.9),
    hit(0.45, F5, 1.0),
];

// Ding-dong down a major third (A5 -> F5) / up a fourth (E5 -> A5).
const CLASSIC_BREAK: &[Note] = &[hit(0.0, A5, 1.0), hit(0.5, F5, 0.95)];
const CLASSIC_FOCUS: &[Note] = &[hit(0.0, E5, 0.9), hit(0.5, A5, 1.0)];

// Two soft sine tones a fourth apart with a long fade: A4 -> E4 / E4 -> A4.
const SOFT_BREAK: &[Note] = &[hit(0.0, A4, 0.9), hit(0.6, E4, 1.0)];
const SOFT_FOCUS: &[Note] = &[hit(0.0, E4, 0.9), hit(0.6, A4, 1.0)];

// Fast C major arpeggio, last note held: C6 G5 E5 C5 / C5 E5 G5 C6.
const CHIPTUNE_BREAK: &[Note] = &[
    held(0.0, C6, 0.85, 0.075),
    held(0.085, G5, 0.9, 0.075),
    held(0.17, E5, 0.95, 0.075),
    held(0.255, C5, 1.0, 0.65),
];
const CHIPTUNE_FOCUS: &[Note] = &[
    held(0.0, C5, 0.85, 0.075),
    held(0.085, E5, 0.9, 0.075),
    held(0.17, G5, 0.95, 0.075),
    held(0.255, C6, 1.0, 0.65),
];

// Beta reward sounds (ADR-022), every one rising: a two-note blip, a triad with a high note, and a three-note
// arpeggio into a held chord, in the key and with the instrument of the pack.
const POP_MS: u64 = 160;
const POP_RELEASE_S: f64 = 0.12;
const SPARKLE_MS: u64 = 700;
const SPARKLE_RELEASE_S: f64 = 0.25;
const FANFARE_MS: u64 = 1_200;
const FANFARE_RELEASE_S: f64 = 0.35;

// Midnight: C major bell.
const MIDNIGHT_POP: &[Note] = &[hit(0.0, G5, 0.55), hit(0.035, C6, 1.0)];
const MIDNIGHT_SPARKLE: &[Note] = &[
    hit(0.0, C5, 0.8),
    hit(0.09, E5, 0.85),
    hit(0.18, G5, 0.9),
    hit(0.3, E6, 0.6),
];
const MIDNIGHT_FANFARE: &[Note] = &[
    hit(0.0, C5, 0.85),
    hit(0.14, E5, 0.9),
    hit(0.28, G5, 0.95),
    hit(0.46, C5, 0.55),
    hit(0.46, E5, 0.55),
    hit(0.46, G5, 0.65),
    hit(0.46, C6, 1.0),
];

// Vaporwave: Fmaj7 pad.
const VAPORWAVE_POP: &[Note] = &[held(0.0, C6, 0.6, 0.03), held(0.04, E6, 1.0, 0.05)];
const VAPORWAVE_SPARKLE: &[Note] = &[
    held(0.0, F5, 0.8, 0.5),
    held(0.1, A5, 0.85, 0.4),
    held(0.2, C6, 0.9, 0.3),
    held(0.3, E6, 0.7, 0.2),
];
const VAPORWAVE_FANFARE: &[Note] = &[
    held(0.0, F5, 0.8, 0.25),
    held(0.15, A5, 0.85, 0.25),
    held(0.3, C6, 0.9, 0.25),
    held(0.48, F5, 0.55, 0.35),
    held(0.48, A5, 0.55, 0.35),
    held(0.48, C6, 0.65, 0.35),
    held(0.48, E6, 1.0, 0.35),
];

// Lunar: E minor glass.
const LUNAR_POP: &[Note] = &[hit(0.0, B5, 0.55), hit(0.035, E6, 1.0)];
const LUNAR_SPARKLE: &[Note] = &[
    hit(0.0, E5, 0.8),
    hit(0.09, G5, 0.85),
    hit(0.18, B5, 0.9),
    hit(0.3, G6, 0.55),
];
const LUNAR_FANFARE: &[Note] = &[
    hit(0.0, E5, 0.85),
    hit(0.14, G5, 0.9),
    hit(0.28, B5, 0.95),
    hit(0.46, E5, 0.5),
    hit(0.46, G5, 0.5),
    hit(0.46, B5, 0.6),
    hit(0.46, E6, 1.0),
];

// Autumn: F major marimba.
const AUTUMN_POP: &[Note] = &[hit(0.0, C6, 0.6), hit(0.035, F6, 1.0)];
const AUTUMN_SPARKLE: &[Note] = &[
    hit(0.0, F5, 0.85),
    hit(0.09, A5, 0.9),
    hit(0.18, C6, 0.95),
    hit(0.3, F6, 0.7),
];
const AUTUMN_FANFARE: &[Note] = &[
    hit(0.0, F5, 0.85),
    hit(0.13, A5, 0.9),
    hit(0.26, C6, 0.95),
    hit(0.44, F5, 0.6),
    hit(0.44, A5, 0.6),
    hit(0.44, C6, 0.7),
    hit(0.44, F6, 1.0),
];

// Classic: A major door chime.
const CLASSIC_POP: &[Note] = &[hit(0.0, E5, 0.6), hit(0.035, A5, 1.0)];
const CLASSIC_SPARKLE: &[Note] = &[
    hit(0.0, A4, 0.8),
    hit(0.09, CS5, 0.85),
    hit(0.18, E5, 0.9),
    hit(0.3, A5, 0.7),
];
const CLASSIC_FANFARE: &[Note] = &[
    hit(0.0, A4, 0.85),
    hit(0.14, CS5, 0.9),
    hit(0.28, E5, 0.95),
    hit(0.46, A4, 0.55),
    hit(0.46, CS5, 0.55),
    hit(0.46, E5, 0.65),
    hit(0.46, A5, 1.0),
];

// Soft: D major sine tones.
const SOFT_POP: &[Note] = &[hit(0.0, A5, 0.6), hit(0.035, D6, 1.0)];
const SOFT_SPARKLE: &[Note] = &[
    hit(0.0, D5, 0.8),
    hit(0.1, FS5, 0.85),
    hit(0.2, A5, 0.9),
    hit(0.32, D6, 0.7),
];
const SOFT_FANFARE: &[Note] = &[
    hit(0.0, D5, 0.85),
    hit(0.15, FS5, 0.9),
    hit(0.3, A5, 0.95),
    hit(0.48, D5, 0.55),
    hit(0.48, FS5, 0.55),
    hit(0.48, A5, 0.65),
    hit(0.48, D6, 1.0),
];

// Chiptune: C major square-wave arpeggios.
const CHIPTUNE_POP: &[Note] = &[held(0.0, C6, 0.7, 0.03), held(0.04, G6, 1.0, 0.06)];
const CHIPTUNE_SPARKLE: &[Note] = &[
    held(0.0, C5, 0.85, 0.07),
    held(0.08, E5, 0.9, 0.07),
    held(0.16, G5, 0.95, 0.07),
    held(0.24, C6, 1.0, 0.3),
];
const CHIPTUNE_FANFARE: &[Note] = &[
    held(0.0, C5, 0.85, 0.1),
    held(0.12, E5, 0.9, 0.1),
    held(0.24, G5, 0.95, 0.1),
    held(0.4, C5, 0.5, 0.5),
    held(0.4, E5, 0.5, 0.5),
    held(0.4, G5, 0.6, 0.5),
    held(0.4, C6, 1.0, 0.5),
];

/// The score of `chime` in `pack`.
pub(crate) fn score(pack: SoundPack, chime: Chime) -> Score {
    match chime {
        Chime::BreakStart | Chime::FocusStart => phase_score(pack, chime),
        Chime::Pop | Chime::Sparkle | Chime::Fanfare => reward_score(pack, chime),
    }
}

/// The notes of one pack's reward sounds: pop, sparkle, fanfare.
type RewardNotes = (&'static [Note], &'static [Note], &'static [Note]);

/// The score of a beta reward sound (ADR-022) in `pack`, played with the pack's instrument.
fn reward_score(pack: SoundPack, chime: Chime) -> Score {
    let (instrument, (pop, sparkle, fanfare)): (Instrument, RewardNotes) = match pack {
        SoundPack::Midnight => (
            Instrument::Struck(&BELL),
            (MIDNIGHT_POP, MIDNIGHT_SPARKLE, MIDNIGHT_FANFARE),
        ),
        SoundPack::Vaporwave => (
            Instrument::Pad(&SYNTH_PAD),
            (VAPORWAVE_POP, VAPORWAVE_SPARKLE, VAPORWAVE_FANFARE),
        ),
        SoundPack::Lunar => (
            Instrument::Struck(&GLASS),
            (LUNAR_POP, LUNAR_SPARKLE, LUNAR_FANFARE),
        ),
        SoundPack::Autumn => (
            Instrument::Struck(&MARIMBA),
            (AUTUMN_POP, AUTUMN_SPARKLE, AUTUMN_FANFARE),
        ),
        SoundPack::Classic => (
            Instrument::Struck(&DOOR_CHIME),
            (CLASSIC_POP, CLASSIC_SPARKLE, CLASSIC_FANFARE),
        ),
        SoundPack::Soft => (
            Instrument::Struck(&SINE),
            (SOFT_POP, SOFT_SPARKLE, SOFT_FANFARE),
        ),
        SoundPack::Chiptune => (
            Instrument::Pulse(&SQUARE),
            (CHIPTUNE_POP, CHIPTUNE_SPARKLE, CHIPTUNE_FANFARE),
        ),
    };
    let (len_ms, release_s, notes) = match chime {
        Chime::Pop => (POP_MS, POP_RELEASE_S, pop),
        Chime::Sparkle => (SPARKLE_MS, SPARKLE_RELEASE_S, sparkle),
        _ => (FANFARE_MS, FANFARE_RELEASE_S, fanfare),
    };
    Score {
        len_ms,
        release_s,
        instrument,
        notes,
    }
}

/// The score of a phase chime in `pack`.
fn phase_score(pack: SoundPack, chime: Chime) -> Score {
    let pick = |break_notes, focus_notes| {
        if chime == Chime::BreakStart {
            break_notes
        } else {
            focus_notes
        }
    };
    let (len_ms, release_s, instrument, notes) = match pack {
        SoundPack::Midnight => (
            2_200,
            0.3,
            Instrument::Struck(&BELL),
            pick(MIDNIGHT_BREAK, MIDNIGHT_FOCUS),
        ),
        SoundPack::Vaporwave => (
            2_500,
            0.3,
            Instrument::Pad(&SYNTH_PAD),
            pick(VAPORWAVE_BREAK, VAPORWAVE_FOCUS),
        ),
        SoundPack::Lunar => (
            2_500,
            0.6,
            Instrument::Struck(&GLASS),
            pick(LUNAR_BREAK, LUNAR_FOCUS),
        ),
        SoundPack::Autumn => (
            1_600,
            0.3,
            Instrument::Struck(&MARIMBA),
            pick(AUTUMN_BREAK, AUTUMN_FOCUS),
        ),
        SoundPack::Classic => (
            2_000,
            0.3,
            Instrument::Struck(&DOOR_CHIME),
            pick(CLASSIC_BREAK, CLASSIC_FOCUS),
        ),
        SoundPack::Soft => (
            2_400,
            0.8,
            Instrument::Struck(&SINE),
            pick(SOFT_BREAK, SOFT_FOCUS),
        ),
        SoundPack::Chiptune => (
            1_200,
            0.15,
            Instrument::Pulse(&SQUARE),
            pick(CHIPTUNE_BREAK, CHIPTUNE_FOCUS),
        ),
    };
    Score {
        len_ms,
        release_s,
        instrument,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;
    use std::sync::OnceLock;

    /// Every pack and chime, rendered once for the whole test module (rendering is slow in debug builds).
    fn rendered(pack: SoundPack, chime: Chime) -> &'static [Sample] {
        static ALL: OnceLock<Vec<(SoundPack, Chime, Vec<Sample>)>> = OnceLock::new();
        let all = ALL.get_or_init(|| {
            SoundPack::ALL
                .iter()
                .flat_map(|&pack| Chime::ALL.map(|chime| (pack, chime, render(chime, pack))))
                .collect()
        });
        all.iter()
            .find(|(p, c, _)| *p == pack && *c == chime)
            .map(|(_, _, samples)| samples.as_slice())
            .unwrap_or(&[])
    }

    fn variants() -> impl Iterator<Item = (SoundPack, Chime)> {
        SoundPack::ALL
            .into_iter()
            .flat_map(|pack| Chime::ALL.map(|chime| (pack, chime)))
    }

    fn peak(samples: &[Sample]) -> f64 {
        samples
            .iter()
            .map(|s| f64::from(*s).abs())
            .fold(0.0, f64::max)
    }

    fn index(at_s: f64) -> usize {
        (at_s * f64::from(SAMPLE_RATE_HZ)) as usize
    }

    /// Power of `samples[from_s..to_s]` at `freq_hz` (single DFT bin at an arbitrary frequency).
    fn power_at(samples: &[Sample], from_s: f64, to_s: f64, freq_hz: f64) -> f64 {
        let rate = f64::from(SAMPLE_RATE_HZ);
        let (mut re, mut im) = (0.0_f64, 0.0_f64);
        for (n, s) in samples[index(from_s)..index(to_s)].iter().enumerate() {
            let phase = TAU * freq_hz * n as f64 / rate;
            re += f64::from(*s) * phase.cos();
            im += f64::from(*s) * phase.sin();
        }
        re * re + im * im
    }

    /// Power-weighted mean pitch (MIDI note number) of `samples[from_s..to_s]`, measured with a Hann window
    /// on the semitone grid between `lo_hz` and `hi_hz`.
    fn pitch_centroid(samples: &[Sample], from_s: f64, to_s: f64, lo_hz: f64, hi_hz: f64) -> f64 {
        let rate = f64::from(SAMPLE_RATE_HZ);
        let window = &samples[index(from_s)..index(to_s)];
        let len = window.len() as f64;
        let midi = |f: f64| 69.0 + 12.0 * (f / 440.0).log2();
        let (mut weighted, mut total) = (0.0, 0.0);
        for note in midi(lo_hz).ceil() as i32..=midi(hi_hz).floor() as i32 {
            let freq = 440.0 * ((f64::from(note) - 69.0) / 12.0).exp2();
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for (n, s) in window.iter().enumerate() {
                let hann = 0.5 - 0.5 * (TAU * n as f64 / len).cos();
                let phase = TAU * freq * n as f64 / rate;
                re += f64::from(*s) * hann * phase.cos();
                im += f64::from(*s) * hann * phase.sin();
            }
            let power = re * re + im * im;
            weighted += power * f64::from(note);
            total += power;
        }
        weighted / total
    }

    /// When a held note has faded out completely (`None` for struck notes, which ring to the end).
    fn note_end(instrument: Instrument, note: &Note) -> Option<f64> {
        match instrument {
            Instrument::Struck(_) => None,
            Instrument::Pad(pad) => Some(note.at_s + note.hold_s + pad.release_s),
            Instrument::Pulse(pulse) => Some(note.at_s + note.hold_s + pulse.release_s),
        }
    }

    /// Onsets of the score and the highest note of each (the melody), in time order.
    fn melody(score: &Score) -> Vec<(f64, f64)> {
        let mut steps: Vec<(f64, f64)> = Vec::new();
        for note in score.notes {
            match steps.iter_mut().find(|(at, _)| *at == note.at_s) {
                Some(step) => step.1 = step.1.max(note.freq_hz),
                None => steps.push((note.at_s, note.freq_hz)),
            }
        }
        steps.sort_by(|a, b| a.0.total_cmp(&b.0));
        steps
    }

    #[test]
    fn every_chime_lasts_one_to_two_and_a_half_seconds() {
        for (pack, chime) in variants() {
            let duration = pack.duration(chime);
            assert!(
                (Duration::from_secs(1)..=Duration::from_millis(2_500)).contains(&duration),
                "{pack:?} {chime:?}: {duration:?}"
            );
            let expected = synth::sample_count(score(pack, chime).len_ms, SAMPLE_RATE_HZ);
            assert_eq!(rendered(pack, chime).len(), expected, "{pack:?} {chime:?}");
        }
        assert_eq!(
            rendered(SoundPack::Midnight, Chime::BreakStart).len(),
            97_020
        );
    }

    #[test]
    fn samples_are_finite_and_in_range() {
        for (pack, chime) in variants() {
            for (n, s) in rendered(pack, chime).iter().enumerate() {
                assert!(
                    s.is_finite() && (-1.0..=1.0).contains(s),
                    "{pack:?} {chime:?}: sample {n} out of range: {s}"
                );
            }
        }
    }

    #[test]
    fn every_chime_peaks_at_the_same_level() {
        for (pack, chime) in variants() {
            let p = peak(rendered(pack, chime));
            assert!(
                (p - f64::from(PEAK_LEVEL)).abs() < 1e-6,
                "{pack:?} {chime:?}: peak {p}"
            );
        }
    }

    #[test]
    fn every_chime_starts_and_ends_silently() {
        let tail = index(0.01);
        for (pack, chime) in variants() {
            let samples = rendered(pack, chime);
            assert!(
                f64::from(samples[0]).abs() < 1e-9,
                "{pack:?} {chime:?} starts with a click"
            );
            let end = peak(&samples[samples.len() - tail..]);
            assert!(end < 0.01, "{pack:?} {chime:?} ends with a click: {end}");
        }
    }

    #[test]
    fn onsets_are_gentle() {
        // No sample jumps more than a sine at full level could within the first millisecond of a note: the
        // attack envelopes start every note from zero.
        for (pack, chime) in variants() {
            let samples = rendered(pack, chime);
            for note in score(pack, chime).notes {
                let at = index(note.at_s);
                if at == 0 {
                    continue;
                }
                let before = f64::from(samples[at - 1]);
                let after = f64::from(samples[at]);
                assert!(
                    (after - before).abs() < 0.2,
                    "{pack:?} {chime:?}: jump at {}s",
                    note.at_s
                );
            }
        }
    }

    #[test]
    fn melody_moves_down_for_breaks_and_up_for_focus() {
        for (pack, chime) in variants() {
            let steps = melody(&score(pack, chime));
            assert!(
                steps.len() >= 2,
                "{pack:?} {chime:?}: needs at least two steps"
            );
            for pair in steps.windows(2) {
                let (from, to) = (pair[0].1, pair[1].1);
                match chime {
                    Chime::BreakStart => assert!(to < from, "{pack:?} {chime:?}: {steps:?}"),
                    _ => assert!(to > from, "{pack:?} {chime:?}: {steps:?}"),
                }
            }
        }
    }

    #[test]
    fn rendered_pitch_falls_for_breaks_and_rises_for_focus() {
        for (pack, chime) in variants() {
            let score = score(pack, chime);
            let samples = rendered(pack, chime);
            let steps = melody(&score);
            let settle = score.instrument.attack_s();
            let (first, second, last) = (steps[0].0, steps[1].0, steps[steps.len() - 1].0);
            // The first window ends before the second step; the second one starts once the last step has
            // reached full level and every earlier held note has been released (struck notes ring on).
            // Windows are up to 0.6 s long, so the slow beating of the detuned pad voices averages out.
            let start_a = first + settle;
            let width_a = (second - start_a - 0.005).min(0.6);
            let start_b = score
                .notes
                .iter()
                .filter(|n| n.at_s < last)
                .filter_map(|n| note_end(score.instrument, n))
                .fold(last + settle, f64::max);
            let width_b = (score.len_s() - start_b - score.release_s).min(0.6);
            assert!(width_a > 0.05 && width_b > 0.05, "{pack:?} {chime:?}");
            // Measure around the notes played (a semitone of margin), not their overtones.
            let lo = score
                .notes
                .iter()
                .map(|n| n.freq_hz)
                .fold(f64::MAX, f64::min)
                / 1.06;
            let hi = score.notes.iter().map(|n| n.freq_hz).fold(0.0, f64::max) * 1.06;
            let start = pitch_centroid(samples, start_a, start_a + width_a, lo, hi);
            let end = pitch_centroid(samples, start_b, start_b + width_b, lo, hi);
            match chime {
                Chime::BreakStart => assert!(
                    end < start - 1.0,
                    "{pack:?} {chime:?} does not fall: {start:.2} -> {end:.2}"
                ),
                _ => assert!(
                    end > start + 1.0,
                    "{pack:?} {chime:?} does not rise: {start:.2} -> {end:.2}"
                ),
            }
        }
    }

    #[test]
    fn midnight_break_descends_from_e5_to_c5() {
        let samples = rendered(SoundPack::Midnight, Chime::BreakStart);
        // First strike: E5 dominates, C5 is silent.
        assert!(power_at(samples, 0.0, 0.4, E5) > 100.0 * power_at(samples, 0.0, 0.4, C5));
        // Second strike: C5 is louder than the decaying E5.
        assert!(power_at(samples, 0.5, 0.9, C5) > 1.5 * power_at(samples, 0.5, 0.9, E5));
    }

    #[test]
    fn midnight_focus_ascends_from_c5_to_g5() {
        let samples = rendered(SoundPack::Midnight, Chime::FocusStart);
        assert!(power_at(samples, 0.0, 0.4, C5) > 100.0 * power_at(samples, 0.0, 0.4, G5));
        assert!(power_at(samples, 0.5, 0.9, G5) > 1.5 * power_at(samples, 0.5, 0.9, C5));
    }

    #[test]
    fn every_chime_sounds_different() {
        let all: Vec<_> = variants().collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(rendered(a.0, a.1), rendered(b.0, b.1), "{a:?} == {b:?}");
            }
        }
    }

    #[test]
    fn low_sample_rate_drops_partials_above_nyquist() {
        // 8 kHz: the high partials and harmonics of every pack would alias; they must be skipped.
        for (pack, chime) in variants() {
            let score = score(pack, chime);
            let samples = synth::render(&score, 8_000, f64::from(PEAK_LEVEL));
            assert_eq!(samples.len(), synth::sample_count(score.len_ms, 8_000));
            assert!(
                samples
                    .iter()
                    .all(|s| s.is_finite() && (-1.0..=1.0).contains(s)),
                "{pack:?} {chime:?}"
            );
        }
    }

    // --- Beta reward sounds (ADR-022) ---

    fn reward_variants() -> impl Iterator<Item = (SoundPack, Chime)> {
        SoundPack::ALL
            .into_iter()
            .flat_map(|pack| Chime::REWARDS.map(|chime| (pack, chime)))
    }

    /// Every pack's reward sounds, rendered once.
    fn rendered_reward(pack: SoundPack, chime: Chime) -> &'static [Sample] {
        static ALL: OnceLock<Vec<(SoundPack, Chime, Vec<Sample>)>> = OnceLock::new();
        let all = ALL.get_or_init(|| {
            reward_variants()
                .map(|(pack, chime)| (pack, chime, render(chime, pack)))
                .collect()
        });
        all.iter()
            .find(|(p, c, _)| *p == pack && *c == chime)
            .map(|(_, _, samples)| samples.as_slice())
            .unwrap_or(&[])
    }

    #[test]
    fn phase_chimes_and_reward_sounds_are_separate() {
        assert_eq!(Chime::ALL, [Chime::BreakStart, Chime::FocusStart]);
        assert_eq!(Chime::REWARDS, [Chime::Pop, Chime::Sparkle, Chime::Fanfare]);
    }

    #[test]
    fn reward_sounds_are_short_and_ordered_by_length() {
        for pack in SoundPack::ALL {
            let [pop, sparkle, fanfare] = Chime::REWARDS.map(|chime| pack.duration(chime));
            for (chime, duration) in [
                (Chime::Pop, pop),
                (Chime::Sparkle, sparkle),
                (Chime::Fanfare, fanfare),
            ] {
                assert!(
                    (Duration::from_millis(120)..=Duration::from_millis(1_400)).contains(&duration),
                    "{pack:?} {chime:?}: {duration:?}"
                );
                let expected = synth::sample_count(score(pack, chime).len_ms, SAMPLE_RATE_HZ);
                assert_eq!(
                    rendered_reward(pack, chime).len(),
                    expected,
                    "{pack:?} {chime:?}"
                );
            }
            assert!(pop < sparkle && sparkle < fanfare, "{pack:?}");
        }
    }

    #[test]
    fn reward_sounds_start_and_end_silently_at_the_common_peak() {
        let tail = index(0.005);
        for (pack, chime) in reward_variants() {
            let samples = rendered_reward(pack, chime);
            assert!(
                samples
                    .iter()
                    .all(|s| s.is_finite() && (-1.0..=1.0).contains(s))
            );
            let p = peak(samples);
            assert!(
                (p - f64::from(PEAK_LEVEL)).abs() < 1e-6,
                "{pack:?} {chime:?}: peak {p}"
            );
            assert!(
                f64::from(samples[0]).abs() < 1e-9,
                "{pack:?} {chime:?} starts with a click"
            );
            let end = peak(&samples[samples.len() - tail..]);
            assert!(end < 0.01, "{pack:?} {chime:?} ends with a click: {end}");
            for note in score(pack, chime).notes {
                let at = index(note.at_s);
                if at > 0 {
                    let jump = (f64::from(samples[at]) - f64::from(samples[at - 1])).abs();
                    assert!(jump < 0.2, "{pack:?} {chime:?}: jump at {}s", note.at_s);
                }
            }
        }
    }

    #[test]
    fn reward_sounds_rise() {
        for (pack, chime) in reward_variants() {
            let steps = melody(&score(pack, chime));
            assert!(steps.len() >= 2, "{pack:?} {chime:?}");
            assert!(
                steps.windows(2).all(|pair| pair[1].1 > pair[0].1),
                "{pack:?} {chime:?}: {steps:?}"
            );
        }
        // Rendered: the end sounds higher than the first note (long enough to measure: sparkle and fanfare).
        for pack in SoundPack::ALL {
            for chime in [Chime::Sparkle, Chime::Fanfare] {
                let score = score(pack, chime);
                let samples = rendered_reward(pack, chime);
                let steps = melody(&score);
                let (first, second) = (steps[0].0, steps[1].0);
                let lo = score
                    .notes
                    .iter()
                    .map(|n| n.freq_hz)
                    .fold(f64::MAX, f64::min)
                    / 1.06;
                let hi = score.notes.iter().map(|n| n.freq_hz).fold(0.0, f64::max) * 1.06;
                let start = pitch_centroid(samples, first + 0.005, second - 0.005, lo, hi);
                let end_at = score.len_s() - score.release_s;
                let end = pitch_centroid(samples, end_at - 0.2, end_at, lo, hi);
                assert!(
                    end > start + 1.0,
                    "{pack:?} {chime:?} does not rise: {start:.2} -> {end:.2}"
                );
            }
        }
    }

    #[test]
    fn reward_sounds_differ_by_kind() {
        for pack in SoundPack::ALL {
            assert_ne!(
                rendered_reward(pack, Chime::Pop),
                rendered_reward(pack, Chime::Sparkle)
            );
            assert_ne!(
                rendered_reward(pack, Chime::Sparkle),
                rendered_reward(pack, Chime::Fanfare)
            );
        }
    }

    #[test]
    fn default_source_is_the_midnight_pack() {
        assert_eq!(SoundPack::default(), SoundPack::Midnight);
        assert_eq!(
            ChimeSource::default(),
            ChimeSource::Pack(SoundPack::Midnight)
        );
        assert_eq!(
            ChimeSource::from(SoundPack::Lunar),
            ChimeSource::Pack(SoundPack::Lunar)
        );
    }
}
