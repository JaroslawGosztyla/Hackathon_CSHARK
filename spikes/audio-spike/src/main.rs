//! T0-10 audio spike (throwaway). Verifies rodio 0.22 for FocusMode:
//! brown noise as a custom Source, volume = user volume x fade factor, Stop interrupting a fade,
//! file decoding, device errors and reconnect, and a long soak run.
//!
//! Usage:
//!   cargo run --release -- devices            list output devices
//!   cargo run --release -- scenario           ~20 s scripted check (fade, postpone, stop mid-fade, file)
//!   cargo run --release -- soak <secs>        play brown noise, report every 60 s (40 min = 2400)
//!   cargo run --release [--features boost] -- diag <secs> <interval>
//!                                             metronome (beep every audio second) + callback pause probe

use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use rodio::source::noise::WhiteUniform;
use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, Sample, SampleRate, Source};

// ---------------------------------------------------------------- brown noise

/// Brown (red) noise: leaky integration of white noise. Mono, infinite.
pub struct BrownNoise {
    white: WhiteUniform,
    last: f32,
    sample_rate: SampleRate,
}

impl BrownNoise {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            white: WhiteUniform::new(sample_rate),
            last: 0.0,
            sample_rate,
        }
    }
}

impl Iterator for BrownNoise {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        let w = self.white.next()?;
        self.last = (self.last + 0.02 * w) / 1.02;
        Some((self.last * 3.5).clamp(-1.0, 1.0))
    }
}

impl Source for BrownNoise {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Counts samples actually pulled by the output device (detects stalls during the soak).
struct Counted<S> {
    inner: S,
    pulled: Arc<AtomicU64>,
}

impl<S: Source> Iterator for Counted<S> {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        self.pulled.fetch_add(1, Ordering::Relaxed);
        self.inner.next()
    }
}

impl<S: Source> Source for Counted<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }
    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

// ---------------------------------------------------------------- audio thread

enum AudioCmd {
    PlayNoise,
    PlayFile(std::path::PathBuf),
    SetVolume(f32),
    FadeTo { factor: f32, ms: u64 },
    Resume,
    Stop,
    Quit,
}

#[derive(Debug)]
#[allow(dead_code)] // fields are only read through Debug output
enum AudioEvent {
    Opened(String),
    DeviceError(String),
    Reconnected,
    StopHandled(Duration),
}

struct Fade {
    from: f32,
    to: f32,
    start: Instant,
    dur: Duration,
}

const NOISE_RATE: u32 = 44_100;

fn open_sink(events: &Sender<AudioEvent>) -> Result<MixerDeviceSink, String> {
    let tx = events.clone();
    let mut sink = DeviceSinkBuilder::from_default_device()
        .map_err(|e| e.to_string())?
        .with_error_callback(move |err| {
            let _ = tx.send(AudioEvent::DeviceError(err.to_string()));
        })
        .open_sink_or_fallback()
        .map_err(|e| e.to_string())?;
    sink.log_on_drop(false);
    Ok(sink)
}

fn spawn_audio(
    pulled: Arc<AtomicU64>,
) -> (
    Sender<AudioCmd>,
    Receiver<AudioEvent>,
    thread::JoinHandle<()>,
) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCmd>();
    let (ev_tx, ev_rx) = mpsc::channel::<AudioEvent>();
    // Device errors come from cpal's thread; we forward them into our own loop.
    let (err_tx, err_rx) = mpsc::channel::<AudioEvent>();

    let handle = thread::spawn(move || {
        let mut sink: Option<MixerDeviceSink> = None;
        let mut player: Option<Player> = None;
        let mut user_volume = 0.5_f32;
        let mut fade_factor = 1.0_f32;
        let mut fade: Option<Fade> = None;
        let mut playing_noise = false;
        let mut retry_at: Option<Instant> = None;

        let ensure_sink = |sink: &mut Option<MixerDeviceSink>| -> bool {
            if sink.is_none() {
                match open_sink(&err_tx) {
                    Ok(s) => {
                        let _ = ev_tx.send(AudioEvent::Opened(format!("{:?}", s.config())));
                        *sink = Some(s);
                    }
                    Err(e) => {
                        let _ = ev_tx.send(AudioEvent::DeviceError(format!("open failed: {e}")));
                    }
                }
            }
            sink.is_some()
        };
        let start_noise = |sink: &MixerDeviceSink, vol: f32| -> Player {
            let p = Player::connect_new(sink.mixer());
            p.set_volume(vol);
            p.append(Counted {
                inner: BrownNoise::new(NonZero::new(NOISE_RATE).unwrap()),
                pulled: pulled.clone(),
            });
            p
        };

        loop {
            // Short timeout so fades advance and Stop is handled within ~20 ms even mid-fade.
            match cmd_rx.recv_timeout(Duration::from_millis(20)) {
                Ok(cmd) => match cmd {
                    AudioCmd::PlayNoise => {
                        fade_factor = 1.0;
                        fade = None;
                        if ensure_sink(&mut sink) {
                            player = Some(start_noise(sink.as_ref().unwrap(), user_volume));
                            playing_noise = true;
                        }
                    }
                    AudioCmd::PlayFile(path) => {
                        if ensure_sink(&mut sink) {
                            match std::fs::File::open(&path)
                                .map_err(|e| e.to_string())
                                .and_then(|f| {
                                    rodio::Decoder::try_from(f).map_err(|e| e.to_string())
                                }) {
                                Ok(src) => {
                                    let p = Player::connect_new(sink.as_ref().unwrap().mixer());
                                    p.set_volume(user_volume * fade_factor);
                                    p.append(src);
                                    player = Some(p);
                                    playing_noise = false;
                                }
                                Err(e) => {
                                    let _ =
                                        ev_tx.send(AudioEvent::DeviceError(format!("decode: {e}")));
                                }
                            }
                        }
                    }
                    AudioCmd::SetVolume(v) => user_volume = v.clamp(0.0, 1.0),
                    AudioCmd::FadeTo { factor, ms } => {
                        fade = Some(Fade {
                            from: fade_factor,
                            to: factor,
                            start: Instant::now(),
                            dur: Duration::from_millis(ms),
                        });
                    }
                    AudioCmd::Resume => {
                        fade = None;
                        fade_factor = 1.0;
                    }
                    AudioCmd::Stop => {
                        let t = Instant::now();
                        fade = None;
                        if let Some(p) = player.take() {
                            p.stop();
                        }
                        playing_noise = false;
                        let _ = ev_tx.send(AudioEvent::StopHandled(t.elapsed()));
                    }
                    AudioCmd::Quit => break,
                },
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            // Device errors: drop the sink and reopen the (possibly new) default device.
            while let Ok(ev) = err_rx.try_recv() {
                let _ = ev_tx.send(ev);
                player = None;
                sink = None;
                retry_at = Some(Instant::now());
            }
            if let Some(at) = retry_at
                && Instant::now() >= at
            {
                if ensure_sink(&mut sink) {
                    if playing_noise {
                        player = Some(start_noise(
                            sink.as_ref().unwrap(),
                            user_volume * fade_factor,
                        ));
                    }
                    retry_at = None;
                    let _ = ev_tx.send(AudioEvent::Reconnected);
                } else {
                    retry_at = Some(Instant::now() + Duration::from_secs(2));
                }
            }

            if let Some(f) = &fade {
                let t = (f.start.elapsed().as_secs_f32() / f.dur.as_secs_f32().max(0.001)).min(1.0);
                fade_factor = f.from + (f.to - f.from) * t;
                if t >= 1.0 {
                    fade = None;
                }
            }
            if let Some(p) = &player {
                p.set_volume(user_volume * fade_factor);
            }
        }
    });
    (cmd_tx, ev_rx, handle)
}

// ---------------------------------------------------------------- modes

fn drain(ev: &Receiver<AudioEvent>) {
    while let Ok(e) = ev.try_recv() {
        println!("  event: {e:?}");
    }
}

fn devices() -> Result<(), String> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait};
    let host = rodio::cpal::default_host();
    let default = host
        .default_output_device()
        .and_then(|d| d.description().ok().map(|x| x.to_string()));
    println!("host: {:?}\ndefault: {default:?}", host.id());
    for d in host.output_devices().map_err(|e| e.to_string())? {
        println!("  - {:?}", d.description().map(|x| x.to_string()));
    }
    Ok(())
}

fn scenario() -> Result<(), String> {
    let pulled = Arc::new(AtomicU64::new(0));
    let (cmd, ev, h) = spawn_audio(pulled);
    let step = |msg: &str, secs: f32| {
        println!("{msg}");
        thread::sleep(Duration::from_secs_f32(secs));
        drain(&ev);
    };

    cmd.send(AudioCmd::PlayNoise).ok();
    step("1. brown noise, volume 0.5 (3 s)", 3.0);
    cmd.send(AudioCmd::FadeTo {
        factor: 0.3,
        ms: 2000,
    })
    .ok();
    step("2. warning: fade to 0.3 over 2 s", 2.5);
    cmd.send(AudioCmd::Resume).ok();
    step(
        "3. postpone -> Resume: fade factor back to 1.0 (should be loud again)",
        2.0,
    );
    cmd.send(AudioCmd::SetVolume(0.2)).ok();
    step("4. user volume 0.2 (quieter, fade untouched)", 2.0);
    cmd.send(AudioCmd::SetVolume(0.5)).ok();
    cmd.send(AudioCmd::FadeTo {
        factor: 0.0,
        ms: 10_000,
    })
    .ok();
    step("5. break: 10 s fade-out started ... Stop after 1.5 s", 1.5);
    cmd.send(AudioCmd::Stop).ok();
    step("6. Stop mid-fade (StopHandled should be ~immediate)", 1.0);

    let wav = std::env::temp_dir().join("focusmode-spike-tone.wav");
    let tone = rodio::source::SineWave::new(440.0)
        .take_duration(Duration::from_secs(2))
        .amplify(0.3);
    rodio::wav_to_file(tone, &wav).map_err(|e| e.to_string())?;
    cmd.send(AudioCmd::PlayFile(wav.clone())).ok();
    step("7. decode + play WAV file (2 s, 440 Hz)", 2.5);
    cmd.send(AudioCmd::PlayFile(wav.with_extension("missing")))
        .ok();
    step("8. missing file -> error event, no crash", 0.5);

    cmd.send(AudioCmd::Quit).ok();
    h.join().map_err(|_| "audio thread panicked".to_string())?;
    println!("scenario done");
    Ok(())
}

fn soak(secs: u64) -> Result<(), String> {
    let pulled = Arc::new(AtomicU64::new(0));
    let (cmd, ev, h) = spawn_audio(pulled.clone());
    cmd.send(AudioCmd::PlayNoise).ok();
    let start = Instant::now();
    let mut errors = 0u32;
    let mut reconnects = 0u32;
    let mut last_report = Instant::now();
    let mut last_pulled = 0u64;
    println!(
        "soak {secs} s - unplug/replug headphones or switch the default device to test recovery"
    );
    while start.elapsed() < Duration::from_secs(secs) {
        thread::sleep(Duration::from_millis(250));
        while let Ok(e) = ev.try_recv() {
            match &e {
                AudioEvent::DeviceError(_) => errors += 1,
                AudioEvent::Reconnected => reconnects += 1,
                _ => {}
            }
            println!("[{:>5}s] event: {e:?}", start.elapsed().as_secs());
        }
        if last_report.elapsed() >= Duration::from_secs(60) {
            let now = pulled.load(Ordering::Relaxed);
            let window = last_report.elapsed().as_secs_f64();
            // Mono noise source is pulled at its own rate after resampling; ~1.00 means no stalls.
            let ratio = (now - last_pulled) as f64 / (window * NOISE_RATE as f64);
            println!(
                "[{:>5}s] pulled/expected = {ratio:.3}, errors = {errors}, reconnects = {reconnects}",
                start.elapsed().as_secs()
            );
            last_pulled = now;
            last_report = Instant::now();
        }
    }
    cmd.send(AudioCmd::Quit).ok();
    h.join().map_err(|_| "audio thread panicked".to_string())?;
    println!(
        "soak done: {} s, errors = {errors}, reconnects = {reconnects}",
        start.elapsed().as_secs()
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let res = match args.first().map(String::as_str) {
        Some("devices") => devices(),
        Some("scenario") => scenario(),
        Some("diag") => diag(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10),
        ),
        Some("rawcpal") => rawcpal(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(60)),
        Some("soak") => soak(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2400)),
        _ => Err(
            "usage: audio-spike devices | scenario | soak <secs> | diag <secs> <interval>".into(),
        ),
    };
    if let Err(e) = res {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brown_noise_in_range_and_finite() {
        let mut n = BrownNoise::new(NonZero::new(NOISE_RATE).unwrap());
        let mut sum_sq = 0.0f64;
        let count = NOISE_RATE as usize * 60;
        for _ in 0..count {
            let s = n.next().unwrap();
            assert!(
                s.is_finite() && (-1.0..=1.0).contains(&s),
                "sample out of range: {s}"
            );
            sum_sq += (s as f64) * (s as f64);
        }
        let rms = (sum_sq / count as f64).sqrt();
        assert!(rms > 0.01 && rms < 0.9, "unexpected rms {rms}");
    }
}

// ---------------------------------------------------------------- diagnostics (T0-10 follow-up)

/// Quiet brown noise + a 50 ms 1 kHz beep at every full second of *audio* time.
/// Compare beeps with a stopwatch: if 60 beeps take ~80 s, the device consumes audio too slowly.
struct Metronome {
    noise: BrownNoise,
    n: u64,
    rate: u32,
}

impl Iterator for Metronome {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        let in_second = self.n % self.rate as u64;
        let mut s = self.noise.next()? * 0.15;
        if in_second < (self.rate as u64 / 20) {
            let t = in_second as f32 / self.rate as f32;
            s += 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        }
        self.n += 1;
        Some(s.clamp(-1.0, 1.0))
    }
}

impl Source for Metronome {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        NonZero::new(self.rate).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Default)]
struct ProbeStats {
    pulled: AtomicU64,
    max_gap_us: AtomicU64,
    long_gaps: AtomicU64,
    long_gap_ms: AtomicU64,
}

/// Runs inside the device callback thread: counts samples and measures pauses between pulls.
/// A pause much longer than the device period means the callback thread was starved (audible glitch).
struct Probe<S> {
    inner: S,
    stats: Arc<ProbeStats>,
    last: Option<Instant>,
    k: u32,
}

impl<S: Source> Iterator for Probe<S> {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        self.k = self.k.wrapping_add(1);
        if self.k.is_multiple_of(64) {
            let now = Instant::now();
            if let Some(last) = self.last {
                let gap = now - last;
                self.stats
                    .max_gap_us
                    .fetch_max(gap.as_micros() as u64, Ordering::Relaxed);
                if gap > Duration::from_millis(100) {
                    self.stats.long_gaps.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .long_gap_ms
                        .fetch_add(gap.as_millis() as u64, Ordering::Relaxed);
                }
            }
            self.last = Some(now);
        }
        self.stats.pulled.fetch_add(1, Ordering::Relaxed);
        self.inner.next()
    }
}

impl<S: Source> Source for Probe<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }
    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[cfg(all(windows, feature = "boost"))]
fn disable_power_throttling() {
    use windows::Win32::System::Threading::{
        GetCurrentProcess, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
        ProcessPowerThrottling, SetProcessInformation,
    };
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: 0, // control the policy and turn throttling OFF
    };
    let res = unsafe {
        SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            &state as *const _ as *const core::ffi::c_void,
            size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
    };
    println!(
        "boost: power throttling off -> {res:?}; audio thread uses MMCSS (cpal feature audio_thread_priority)"
    );
}

fn diag(secs: u64, interval: u64) -> Result<(), String> {
    #[cfg(all(windows, feature = "boost"))]
    disable_power_throttling();
    #[cfg(not(feature = "boost"))]
    println!("boost: OFF (cpal default: audio thread THREAD_PRIORITY_TIME_CRITICAL)");

    let (tx, rx) = mpsc::channel::<String>();
    let mut sink = DeviceSinkBuilder::from_default_device()
        .map_err(|e| e.to_string())?
        .with_error_callback(move |e| {
            let _ = tx.send(e.to_string());
        })
        .open_sink_or_fallback()
        .map_err(|e| e.to_string())?;
    sink.log_on_drop(false);
    println!("device config: {:?}", sink.config());

    let stats = Arc::new(ProbeStats::default());
    let player = Player::connect_new(sink.mixer());
    player.set_volume(0.5);
    player.append(Probe {
        inner: Metronome {
            noise: BrownNoise::new(NonZero::new(NOISE_RATE).unwrap()),
            n: 0,
            rate: NOISE_RATE,
        },
        stats: stats.clone(),
        last: None,
        k: 0,
    });

    let start = Instant::now();
    let mut last_t = start;
    let mut last_pulled = 0u64;
    println!(" wall[s]  audio[s]  window  max_pause[ms]  pauses>100ms(total ms)");
    while start.elapsed() < Duration::from_secs(secs) {
        thread::sleep(Duration::from_secs(interval));
        while let Ok(e) = rx.try_recv() {
            println!("  device error: {e}");
        }
        let pulled = stats.pulled.load(Ordering::Relaxed);
        let window = last_t.elapsed().as_secs_f64();
        let ratio = (pulled - last_pulled) as f64 / (window * NOISE_RATE as f64);
        let max_pause = stats.max_gap_us.swap(0, Ordering::Relaxed) as f64 / 1000.0;
        println!(
            "{:>8.1}  {:>8.1}  {:>6.3}  {:>13.1}  {:>3} ({} ms)",
            start.elapsed().as_secs_f64(),
            pulled as f64 / NOISE_RATE as f64,
            ratio,
            max_pause,
            stats.long_gaps.load(Ordering::Relaxed),
            stats.long_gap_ms.load(Ordering::Relaxed),
        );
        last_pulled = pulled;
        last_t = Instant::now();
    }
    let total_audio = stats.pulled.load(Ordering::Relaxed) as f64 / NOISE_RATE as f64;
    println!(
        "diag done: wall {:.1} s, audio {:.1} s, overall ratio {:.3}",
        start.elapsed().as_secs_f64(),
        total_audio,
        total_audio / start.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Raw cpal stream without rodio: counts frames the device asks for vs wall time.
/// Isolates "device/WASAPI clock" from "rodio pipeline" as the cause of a low pull ratio.
fn rawcpal(secs: u64) -> Result<(), String> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = rodio::cpal::default_host();
    let device = host.default_output_device().ok_or("no output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let config: rodio::cpal::StreamConfig = supported.config();
    let channels = config.channels as u64;
    let rate = config.sample_rate as f64;
    println!(
        "raw cpal: {config:?}, format {:?}",
        supported.sample_format()
    );

    let frames = Arc::new(AtomicU64::new(0));
    let f2 = frames.clone();
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                data.fill(0.0);
                f2.fetch_add(data.len() as u64 / channels, Ordering::Relaxed);
            },
            |e| eprintln!("stream error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    let start = Instant::now();
    let (mut last_t, mut last_f) = (start, 0u64);
    while start.elapsed() < Duration::from_secs(secs) {
        thread::sleep(Duration::from_secs(10));
        let f = frames.load(Ordering::Relaxed);
        println!(
            "{:>6.1}s  device frames/s = {:>8.0}  ratio = {:.3}",
            start.elapsed().as_secs_f64(),
            (f - last_f) as f64 / last_t.elapsed().as_secs_f64(),
            (f - last_f) as f64 / (last_t.elapsed().as_secs_f64() * rate)
        );
        last_f = f;
        last_t = Instant::now();
    }
    Ok(())
}
