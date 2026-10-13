//! Pre-rendered sample buffers and the rodio source that plays them.

use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

/// Interleaved samples ready to play. Cheap to clone (the samples are shared).
#[derive(Debug, Clone)]
pub(crate) struct Pcm {
    pub samples: Arc<[Sample]>,
    pub channels: ChannelCount,
    pub sample_rate: SampleRate,
}

impl Pcm {
    /// Mono buffer.
    pub fn mono(samples: Arc<[Sample]>, sample_rate: SampleRate) -> Self {
        Self {
            samples,
            channels: ChannelCount::MIN,
            sample_rate,
        }
    }

    /// Playing time of the whole buffer.
    pub fn duration(&self) -> Duration {
        let frames = self.samples.len() / usize::from(self.channels.get());
        frames_to_duration(frames, self.sample_rate)
    }
}

/// `frames` at `sample_rate` as a duration (saturating).
pub(crate) fn frames_to_duration(frames: usize, sample_rate: SampleRate) -> Duration {
    let frames = u64::try_from(frames).unwrap_or(u64::MAX);
    let nanos = frames.saturating_mul(1_000_000_000) / u64::from(sample_rate.get());
    Duration::from_nanos(nanos)
}

/// A rodio [`Source`] over a [`Pcm`] buffer.
///
/// The buffer is shared (`Arc`), so starting a chime does not copy or re-render it, and no synthesis or
/// decoding runs on the device callback thread.
#[derive(Debug, Clone)]
pub(crate) struct PcmSource {
    pcm: Pcm,
    pos: usize,
}

impl PcmSource {
    pub fn new(pcm: Pcm) -> Self {
        Self { pcm, pos: 0 }
    }
}

impl Iterator for PcmSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.pcm.samples.get(self.pos).copied()?;
        self.pos += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.pcm.samples.len().saturating_sub(self.pos);
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
        self.pcm.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.pcm.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.pcm.duration())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chime::{self, Chime, SAMPLE_RATE, SoundPack};

    #[test]
    fn pcm_source_plays_buffer_once() {
        let samples: Arc<[Sample]> = chime::render(Chime::BreakStart, SoundPack::Midnight).into();
        let mut source = PcmSource::new(Pcm::mono(Arc::clone(&samples), SAMPLE_RATE));
        assert_eq!(source.channels().get(), 1);
        assert_eq!(source.sample_rate(), SAMPLE_RATE);
        assert_eq!(source.current_span_len(), None);
        assert_eq!(
            source.total_duration(),
            Some(SoundPack::Midnight.duration(Chime::BreakStart))
        );
        assert_eq!(source.len(), samples.len());
        assert_eq!(source.next(), Some(samples[0]));
        assert_eq!(source.len(), samples.len() - 1);
        let rest: Vec<Sample> = source.by_ref().collect();
        assert_eq!(rest.as_slice(), &samples[1..]);
        assert_eq!(source.next(), None);
        assert_eq!(source.len(), 0);
    }

    #[test]
    fn empty_pcm_source_ends_immediately() {
        let mut source = PcmSource::new(Pcm::mono(Arc::from(Vec::<Sample>::new()), SAMPLE_RATE));
        assert_eq!(source.next(), None);
        assert_eq!(source.total_duration(), Some(Duration::ZERO));
    }

    #[test]
    fn stereo_duration_counts_frames() {
        let rate = SampleRate::new(1_000).expect("non-zero");
        let pcm = Pcm {
            samples: Arc::from(vec![0.0; 2_000]),
            channels: ChannelCount::new(2).expect("non-zero"),
            sample_rate: rate,
        };
        assert_eq!(pcm.duration(), Duration::from_secs(1));
        assert_eq!(PcmSource::new(pcm).channels().get(), 2);
    }
}
