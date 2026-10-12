//! Dedicated audio thread that plays chimes (ADR-003).
//!
//! The thread is the only owner of the output device. It opens the device lazily on the first chime and closes
//! it again after a few idle seconds: an open WASAPI stream registers an "audio stream in use" power request
//! that blocks idle sleep, and re-opening also follows a changed default device. After a device error the
//! device is dropped and re-opened on the next chime. Nothing here panics: errors are logged and sent as
//! [`AudioEvent::Error`].

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Sample};

use crate::chime::{self, CHIME_DURATION, Chime, PcmSource};

const THREAD_NAME: &str = "focus-audio";

/// At most this many chimes overlap; the oldest one is stopped when another starts (e.g. "test chime" spam).
const MAX_VOICES: usize = 4;

/// Events reported by the audio thread.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// The chime could not be played (no device, device lost, thread failure). Human-readable, for logs and
    /// the `audio.device` error toast.
    Error(String),
}

/// Audio failures. They are reported as [`AudioEvent::Error`] and never stop the session.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no usable audio output device: {0}")]
    Device(String),
    #[error("audio output device failed: {0}")]
    DeviceLost(String),
    #[error("could not start the audio thread: {0}")]
    Thread(String),
}

/// Handle to the chime thread. Cheap to clone; the thread exits when the last clone is dropped.
#[derive(Debug, Clone)]
pub struct ChimePlayer {
    commands: Sender<Command>,
}

#[derive(Debug)]
enum Command {
    Play { chime: Chime, volume: f32 },
}

impl ChimePlayer {
    /// Spawns the audio thread ("focus-audio"). The device is opened lazily on first play and re-opened after
    /// errors. Device problems arrive on the returned receiver.
    pub fn spawn() -> (ChimePlayer, Receiver<AudioEvent>) {
        let (player, events, _detached) = spawn_thread(RodioBackend, Timing::DEFAULT);
        (player, events)
    }

    /// Non-blocking. `volume` 0..=1 (clamped, NaN -> 0). Volume 0 plays nothing and does not open the device.
    pub fn play(&self, chime: Chime, volume: f32) {
        let volume = sanitize_volume(volume);
        if self.commands.send(Command::Play { chime, volume }).is_err() {
            log::warn!("audio thread is not running; chime {chime:?} dropped");
        }
    }
}

fn sanitize_volume(volume: f32) -> f32 {
    if volume.is_nan() {
        0.0
    } else {
        volume.clamp(0.0, 1.0)
    }
}

/// `err` followed by its sources. rodio's own messages are generic ("Error opening the stream with the
/// OS"); the cpal cause underneath carries the useful detail.
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

fn spawn_thread<B>(
    backend: B,
    timing: Timing,
) -> (ChimePlayer, Receiver<AudioEvent>, Option<JoinHandle<()>>)
where
    B: Backend + Send + 'static,
{
    let (commands_tx, commands_rx) = mpsc::channel::<Command>();
    let (events_tx, events_rx) = mpsc::channel::<AudioEvent>();
    let thread_events = events_tx.clone();
    let spawned = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || run(backend, commands_rx, thread_events, timing));
    let handle = match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            let err = AudioError::Thread(err.to_string());
            log::error!("{err}");
            let _ = events_tx.send(AudioEvent::Error(err.to_string()));
            None
        }
    };
    (
        ChimePlayer {
            commands: commands_tx,
        },
        events_rx,
        handle,
    )
}

/// Loop timings (shortened in tests).
#[derive(Debug, Clone, Copy)]
struct Timing {
    /// Poll interval while the device is open (device faults, finished chimes, idle close).
    poll: Duration,
    /// A chime still "playing" this long after its nominal end is dropped anyway (stalled device).
    overrun: Duration,
    /// The device is closed after this long without any chime playing.
    idle_close: Duration,
}

impl Timing {
    const DEFAULT: Timing = Timing {
        poll: Duration::from_millis(50),
        overrun: Duration::from_secs(5),
        idle_close: Duration::from_secs(10),
    };
}

/// Device error forwarded from the backend's stream thread. `generation` identifies the device instance, so
/// a late error from an already replaced device is ignored.
#[derive(Debug)]
struct Fault {
    generation: u64,
    message: String,
}

/// Output device abstraction: rodio in production, a recording fake in tests.
trait Backend {
    type Sink;
    type Voice;

    /// Opens the current default output device. Asynchronous device errors must be sent to `faults` tagged
    /// with `generation`, without blocking.
    fn open(&mut self, generation: u64, faults: &Sender<Fault>) -> Result<Self::Sink, AudioError>;
    /// Starts playing `pcm` (mono, [`chime::SAMPLE_RATE`]) at `volume`.
    fn start(&mut self, sink: &Self::Sink, pcm: Arc<[Sample]>, volume: f32) -> Self::Voice;
    /// True once the voice has played everything.
    fn is_finished(&self, voice: &Self::Voice) -> bool;
    /// Stops the voice immediately.
    fn stop(&mut self, voice: Self::Voice);
}

/// rodio 0.22 backend (API verified in `spikes/audio-spike`).
struct RodioBackend;

impl Backend for RodioBackend {
    type Sink = MixerDeviceSink;
    type Voice = Player;

    fn open(
        &mut self,
        generation: u64,
        faults: &Sender<Fault>,
    ) -> Result<MixerDeviceSink, AudioError> {
        let faults = faults.clone();
        let mut sink = DeviceSinkBuilder::from_default_device()
            .map_err(|e| AudioError::Device(error_chain(&e)))?
            .with_error_callback(move |err| {
                // A buffer underrun (reported by ALSA/JACK, never by WASAPI) is a short glitch, not a
                // lost device: closing the device and showing an error toast for it would be wrong.
                if matches!(err, rodio::cpal::StreamError::BufferUnderrun) {
                    return;
                }
                // Runs on cpal's stream thread: only forward the error, never block here.
                let _ = faults.send(Fault {
                    generation,
                    message: err.to_string(),
                });
            })
            .open_sink_or_fallback()
            .map_err(|e| AudioError::Device(error_chain(&e)))?;
        sink.log_on_drop(false);
        Ok(sink)
    }

    fn start(&mut self, sink: &MixerDeviceSink, pcm: Arc<[Sample]>, volume: f32) -> Player {
        let player = Player::connect_new(sink.mixer());
        player.set_volume(volume);
        player.append(PcmSource::new(pcm, chime::SAMPLE_RATE));
        player
    }

    fn is_finished(&self, voice: &Player) -> bool {
        voice.empty()
    }

    fn stop(&mut self, voice: Player) {
        voice.stop();
    }
}

struct ActiveVoice<V> {
    voice: V,
    started_at: Instant,
}

/// State of the audio thread. Time is passed in, so the logic is testable without a device or sleeping.
struct Engine<B: Backend> {
    backend: B,
    timing: Timing,
    events: Sender<AudioEvent>,
    faults: Sender<Fault>,
    sink: Option<B::Sink>,
    /// Incremented on every open attempt; tags device faults.
    generation: u64,
    voices: VecDeque<ActiveVoice<B::Voice>>,
    /// Since when the device has been open with nothing playing.
    idle_since: Option<Instant>,
    break_pcm: Option<Arc<[Sample]>>,
    focus_pcm: Option<Arc<[Sample]>>,
}

impl<B: Backend> Engine<B> {
    fn new(backend: B, timing: Timing, events: Sender<AudioEvent>, faults: Sender<Fault>) -> Self {
        Self {
            backend,
            timing,
            events,
            faults,
            sink: None,
            generation: 0,
            voices: VecDeque::new(),
            idle_since: None,
            break_pcm: None,
            focus_pcm: None,
        }
    }

    /// The device is open (the loop then polls; otherwise it sleeps until the next command).
    fn is_active(&self) -> bool {
        self.sink.is_some()
    }

    fn handle(&mut self, command: Command, now: Instant) {
        match command {
            Command::Play { chime, volume } => self.play(chime, volume, now),
        }
    }

    fn play(&mut self, chime: Chime, volume: f32, now: Instant) {
        if volume <= 0.0 || volume.is_nan() {
            log::debug!("chime {chime:?} muted (volume 0)");
            return;
        }
        if self.sink.is_none() && !self.open() {
            return;
        }
        let pcm = self.pcm(chime);
        while self.voices.len() >= MAX_VOICES {
            let Some(oldest) = self.voices.pop_front() else {
                break;
            };
            self.backend.stop(oldest.voice);
        }
        let Some(sink) = self.sink.as_ref() else {
            return;
        };
        let voice = self.backend.start(sink, pcm, volume.min(1.0));
        self.voices.push_back(ActiveVoice {
            voice,
            started_at: now,
        });
        self.idle_since = None;
        log::debug!("chime {chime:?} started at volume {volume}");
    }

    /// Opens the current default device. Returns false (and reports) on failure; the next chime retries.
    fn open(&mut self) -> bool {
        self.generation = self.generation.wrapping_add(1);
        match self.backend.open(self.generation, &self.faults) {
            Ok(sink) => {
                log::info!("audio output opened");
                self.sink = Some(sink);
                self.idle_since = None;
                true
            }
            Err(err) => {
                self.report(&err);
                false
            }
        }
    }

    /// Rendered chime, cached after the first use.
    fn pcm(&mut self, kind: Chime) -> Arc<[Sample]> {
        let slot = match kind {
            Chime::BreakStart => &mut self.break_pcm,
            Chime::FocusStart => &mut self.focus_pcm,
        };
        Arc::clone(slot.get_or_insert_with(|| Arc::from(chime::render(kind))))
    }

    fn on_fault(&mut self, fault: Fault) {
        if fault.generation != self.generation || self.sink.is_none() {
            log::debug!("ignoring stale audio device error: {}", fault.message);
            return;
        }
        self.close();
        self.report(&AudioError::DeviceLost(fault.message));
    }

    /// Releases finished chimes and closes the device after `idle_close` without playback.
    fn tick(&mut self, now: Instant) {
        if self.sink.is_none() {
            return;
        }
        let backend = &self.backend;
        let limit = CHIME_DURATION.saturating_add(self.timing.overrun);
        self.voices.retain(|v| {
            !backend.is_finished(&v.voice) && now.saturating_duration_since(v.started_at) < limit
        });
        if !self.voices.is_empty() {
            self.idle_since = None;
            return;
        }
        let idle_since = *self.idle_since.get_or_insert(now);
        if now.saturating_duration_since(idle_since) >= self.timing.idle_close {
            log::debug!("closing idle audio output");
            self.close();
        }
    }

    /// Stops every chime and drops the device.
    fn close(&mut self) {
        for active in self.voices.drain(..) {
            self.backend.stop(active.voice);
        }
        self.sink = None;
        self.idle_since = None;
    }

    fn report(&self, err: &AudioError) {
        log::warn!("{err}");
        let _ = self.events.send(AudioEvent::Error(err.to_string()));
    }
}

/// Thread body: runs until every [`ChimePlayer`] clone is dropped.
fn run<B: Backend>(
    backend: B,
    commands: Receiver<Command>,
    events: Sender<AudioEvent>,
    timing: Timing,
) {
    let (faults_tx, faults_rx) = mpsc::channel::<Fault>();
    let mut engine = Engine::new(backend, timing, events, faults_tx);
    loop {
        let received = if engine.is_active() {
            match commands.recv_timeout(timing.poll) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            // Device closed: nothing to poll, sleep until the next command.
            match commands.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            }
        };
        let now = Instant::now();
        // Faults first, so a chime right after a device error re-opens the device.
        while let Ok(fault) = faults_rx.try_recv() {
            engine.on_fault(fault);
        }
        if let Some(command) = received {
            engine.handle(command, now);
        }
        engine.tick(now);
    }
    engine.close();
    log::debug!("audio thread stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq)]
    struct Started {
        generation: u64,
        voice: u64,
        samples: usize,
        volume: f32,
    }

    #[derive(Debug, Default)]
    struct FakeState {
        opens: Vec<u64>,
        fail_next_opens: usize,
        started: Vec<Started>,
        stopped: Vec<u64>,
        finished: Vec<u64>,
        next_voice: u64,
    }

    #[derive(Debug, Clone, Default)]
    struct FakeBackend {
        state: Arc<Mutex<FakeState>>,
    }

    impl FakeBackend {
        fn state(&self) -> std::sync::MutexGuard<'_, FakeState> {
            self.state.lock().expect("fake state poisoned")
        }
    }

    struct FakeSink {
        generation: u64,
    }

    struct FakeVoice {
        id: u64,
    }

    impl Backend for FakeBackend {
        type Sink = FakeSink;
        type Voice = FakeVoice;

        fn open(
            &mut self,
            generation: u64,
            _faults: &Sender<Fault>,
        ) -> Result<FakeSink, AudioError> {
            let mut state = self.state();
            state.opens.push(generation);
            if state.fail_next_opens > 0 {
                state.fail_next_opens -= 1;
                return Err(AudioError::Device("no device".into()));
            }
            Ok(FakeSink { generation })
        }

        fn start(&mut self, sink: &FakeSink, pcm: Arc<[Sample]>, volume: f32) -> FakeVoice {
            let mut state = self.state();
            state.next_voice += 1;
            let id = state.next_voice;
            state.started.push(Started {
                generation: sink.generation,
                voice: id,
                samples: pcm.len(),
                volume,
            });
            FakeVoice { id }
        }

        fn is_finished(&self, voice: &FakeVoice) -> bool {
            self.state().finished.contains(&voice.id)
        }

        fn stop(&mut self, voice: FakeVoice) {
            self.state().stopped.push(voice.id);
        }
    }

    const TEST_TIMING: Timing = Timing {
        poll: Duration::from_millis(5),
        overrun: Duration::from_secs(3),
        idle_close: Duration::from_secs(10),
    };

    fn engine(fake: &FakeBackend) -> (Engine<FakeBackend>, Receiver<AudioEvent>) {
        let (events_tx, events_rx) = mpsc::channel();
        let (faults_tx, _faults_rx) = mpsc::channel();
        (
            Engine::new(fake.clone(), TEST_TIMING, events_tx, faults_tx),
            events_rx,
        )
    }

    fn errors(events: &Receiver<AudioEvent>) -> Vec<String> {
        events
            .try_iter()
            .map(|e| match e {
                AudioEvent::Error(message) => message,
            })
            .collect()
    }

    #[test]
    fn sanitize_volume_clamps_and_rejects_nan() {
        assert_eq!(sanitize_volume(f32::NAN), 0.0);
        assert_eq!(sanitize_volume(-0.5), 0.0);
        assert_eq!(sanitize_volume(0.25), 0.25);
        assert_eq!(sanitize_volume(1.5), 1.0);
        assert_eq!(sanitize_volume(f32::INFINITY), 1.0);
        assert_eq!(sanitize_volume(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn error_chain_includes_sources() {
        #[derive(Debug, thiserror::Error)]
        #[error("could not open")]
        struct Outer(#[source] AudioError);

        let err = Outer(AudioError::Device("busy".into()));
        assert_eq!(
            error_chain(&err),
            "could not open: no usable audio output device: busy"
        );
        assert_eq!(
            error_chain(&AudioError::DeviceLost("gone".into())),
            "audio output device failed: gone"
        );
    }

    #[test]
    fn device_is_opened_lazily_and_reused() {
        let fake = FakeBackend::default();
        let (mut engine, events) = engine(&fake);
        assert!(fake.state().opens.is_empty());
        assert!(!engine.is_active());

        let t0 = Instant::now();
        engine.play(Chime::FocusStart, 0.7, t0);
        engine.play(Chime::BreakStart, 0.5, t0 + Duration::from_secs(1));

        let state = fake.state();
        assert_eq!(state.opens, vec![1]);
        let expected = chime::sample_count(chime::SAMPLE_RATE_HZ);
        assert_eq!(
            state.started,
            vec![
                Started {
                    generation: 1,
                    voice: 1,
                    samples: expected,
                    volume: 0.7
                },
                Started {
                    generation: 1,
                    voice: 2,
                    samples: expected,
                    volume: 0.5
                },
            ]
        );
        assert!(engine.is_active());
        assert!(errors(&events).is_empty());
    }

    #[test]
    fn muted_chime_does_not_touch_the_device() {
        let fake = FakeBackend::default();
        let (mut engine, events) = engine(&fake);
        engine.play(Chime::FocusStart, 0.0, Instant::now());
        engine.play(Chime::FocusStart, f32::NAN, Instant::now());
        assert!(fake.state().opens.is_empty());
        assert!(fake.state().started.is_empty());
        assert!(!engine.is_active());
        assert!(errors(&events).is_empty());
    }

    #[test]
    fn open_failure_is_reported_and_retried_on_next_chime() {
        let fake = FakeBackend::default();
        fake.state().fail_next_opens = 1;
        let (mut engine, events) = engine(&fake);
        let t0 = Instant::now();

        engine.play(Chime::BreakStart, 0.8, t0);
        assert!(!engine.is_active());
        assert!(fake.state().started.is_empty());
        let reported = errors(&events);
        assert_eq!(reported.len(), 1);
        assert!(reported[0].contains("no device"), "{reported:?}");

        engine.play(Chime::BreakStart, 0.8, t0);
        assert!(engine.is_active());
        assert_eq!(fake.state().opens, vec![1, 2]);
        assert_eq!(fake.state().started.len(), 1);
        assert_eq!(fake.state().started[0].generation, 2);
    }

    #[test]
    fn device_fault_closes_device_and_next_chime_reopens_it() {
        let fake = FakeBackend::default();
        let (mut engine, events) = engine(&fake);
        let t0 = Instant::now();
        engine.play(Chime::FocusStart, 1.0, t0);

        engine.on_fault(Fault {
            generation: 1,
            message: "device is no longer available".into(),
        });
        assert!(!engine.is_active());
        assert_eq!(fake.state().stopped, vec![1]);
        let reported = errors(&events);
        assert_eq!(reported.len(), 1);
        assert!(reported[0].contains("no longer available"), "{reported:?}");

        // A second error from the same (already dropped) device is not reported again.
        engine.on_fault(Fault {
            generation: 1,
            message: "again".into(),
        });
        assert!(errors(&events).is_empty());

        engine.play(Chime::BreakStart, 1.0, t0);
        assert!(engine.is_active());
        assert_eq!(fake.state().opens, vec![1, 2]);
    }

    #[test]
    fn stale_fault_from_replaced_device_is_ignored() {
        let fake = FakeBackend::default();
        let (mut engine, events) = engine(&fake);
        let t0 = Instant::now();
        engine.play(Chime::FocusStart, 1.0, t0);
        engine.on_fault(Fault {
            generation: 1,
            message: "lost".into(),
        });
        engine.play(Chime::FocusStart, 1.0, t0);
        assert_eq!(errors(&events).len(), 1);

        engine.on_fault(Fault {
            generation: 1,
            message: "late error of the old device".into(),
        });
        assert!(engine.is_active());
        assert!(errors(&events).is_empty());
        assert_eq!(fake.state().stopped, vec![1]);
    }

    #[test]
    fn finished_chime_is_released_and_idle_device_closed() {
        let fake = FakeBackend::default();
        let (mut engine, _events) = engine(&fake);
        let t0 = Instant::now();
        engine.play(Chime::FocusStart, 1.0, t0);

        engine.tick(t0 + Duration::from_secs(1));
        assert_eq!(engine.voices.len(), 1);

        fake.state().finished.push(1);
        let t1 = t0 + Duration::from_secs(2);
        engine.tick(t1);
        assert!(engine.voices.is_empty());
        assert!(engine.is_active());

        engine.tick(t1 + TEST_TIMING.idle_close - Duration::from_millis(1));
        assert!(engine.is_active());
        engine.tick(t1 + TEST_TIMING.idle_close);
        assert!(!engine.is_active());
        // Finished voices are released, not stopped.
        assert!(fake.state().stopped.is_empty());

        // Next chime re-opens the (current default) device.
        engine.play(Chime::BreakStart, 1.0, t1 + Duration::from_secs(60));
        assert_eq!(fake.state().opens, vec![1, 2]);
    }

    #[test]
    fn new_chime_postpones_idle_close() {
        let fake = FakeBackend::default();
        let (mut engine, _events) = engine(&fake);
        let t0 = Instant::now();
        engine.play(Chime::FocusStart, 1.0, t0);
        fake.state().finished.push(1);
        engine.tick(t0 + Duration::from_secs(3));

        let t1 = t0 + Duration::from_secs(8);
        engine.play(Chime::BreakStart, 1.0, t1);
        engine.tick(t0 + Duration::from_secs(3) + TEST_TIMING.idle_close);
        assert!(engine.is_active());
        assert_eq!(fake.state().opens, vec![1]);
    }

    #[test]
    fn stalled_chime_is_dropped_after_overrun() {
        let fake = FakeBackend::default();
        let (mut engine, _events) = engine(&fake);
        let t0 = Instant::now();
        engine.play(Chime::FocusStart, 1.0, t0);
        let limit = CHIME_DURATION + TEST_TIMING.overrun;

        engine.tick(t0 + limit - Duration::from_millis(1));
        assert_eq!(engine.voices.len(), 1);
        engine.tick(t0 + limit);
        assert!(engine.voices.is_empty());
    }

    #[test]
    fn overlapping_chimes_are_capped() {
        let fake = FakeBackend::default();
        let (mut engine, _events) = engine(&fake);
        let t0 = Instant::now();
        for _ in 0..MAX_VOICES + 2 {
            engine.play(Chime::FocusStart, 1.0, t0);
        }
        assert_eq!(engine.voices.len(), MAX_VOICES);
        assert_eq!(fake.state().stopped, vec![1, 2]);
        assert_eq!(fake.state().started.len(), MAX_VOICES + 2);
        assert_eq!(fake.state().opens, vec![1]);
    }

    #[test]
    fn rendered_chimes_are_cached() {
        let fake = FakeBackend::default();
        let (mut engine, _events) = engine(&fake);
        let a = engine.pcm(Chime::BreakStart);
        let b = engine.pcm(Chime::BreakStart);
        let c = engine.pcm(Chime::FocusStart);
        assert!(Arc::ptr_eq(&a, &b));
        assert!(!Arc::ptr_eq(&a, &c));
    }

    #[test]
    fn thread_plays_commands_and_exits_when_handles_drop() {
        let fake = FakeBackend::default();
        let (player, events, handle) = spawn_thread(fake.clone(), TEST_TIMING);
        let second = player.clone();
        player.play(Chime::FocusStart, 0.8);
        second.play(Chime::BreakStart, f32::NAN); // muted -> ignored
        drop(player);
        drop(second);
        handle
            .expect("thread spawned")
            .join()
            .expect("audio thread panicked");

        let state = fake.state();
        assert_eq!(state.opens, vec![1]);
        assert_eq!(state.started.len(), 1);
        assert_eq!(state.started[0].volume, 0.8);
        // Shutting down stops the chime that was still playing.
        assert_eq!(state.stopped, vec![1]);
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn real_player_with_muted_chimes_never_opens_a_device() {
        // Safe without audio hardware: volume 0 never reaches the device.
        let (player, events) = ChimePlayer::spawn();
        player.play(Chime::FocusStart, 0.0);
        player.play(Chime::BreakStart, f32::NAN);
        drop(player);
        assert!(!matches!(
            events.recv_timeout(Duration::from_millis(200)),
            Ok(AudioEvent::Error(_))
        ));
    }
}
