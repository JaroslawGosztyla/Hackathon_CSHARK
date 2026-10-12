//! Session actor: the only owner of the core `Session` (ADR-012).
//!
//! A tokio task selects over the command channel and a 1 s ticker, feeds `Input`s with `Now`
//! (monotonic `Instant` since process start + wall clock) to `Session::handle`, dispatches the
//! returned effects without waiting for slow work, and publishes `session://snapshot` with a
//! growing `revision`. It also mirrors the state into the extension bridge and the tray.

use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, Phase, SessionSnapshot, events};
use focus_core::session::{Input, Now, Session};
use focus_core::timing::TimingProfile;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};
use tokio::time::MissedTickBehavior;

use crate::bridge::SharedBridge;
use crate::effects::Effects;
use crate::tracker::TrackerHandle;
use crate::tray::Tray;
use crate::{SharedConfig, lock, read_config};

pub enum ActorCmd {
    Start,
    Pause,
    Resume,
    /// Tray convenience: pause when running, resume when paused.
    TogglePause,
    Stop,
    Skip,
    /// Config saved: refresh the profile shown while idle.
    ConfigChanged,
    GetSnapshot(oneshot::Sender<SessionSnapshot>),
    /// App exit: stop the session (history written synchronously), then ack.
    Shutdown(std_mpsc::Sender<()>),
}

#[derive(Clone)]
pub struct ActorHandle {
    tx: mpsc::UnboundedSender<ActorCmd>,
}

impl ActorHandle {
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<ActorCmd>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    pub fn send(&self, cmd: ActorCmd) -> Result<(), AppError> {
        self.tx
            .send(cmd)
            .map_err(|_| AppError::new("session.unavailable", "session actor is not running"))
    }

    pub async fn snapshot(&self) -> Result<SessionSnapshot, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(ActorCmd::GetSnapshot(reply))?;
        response
            .await
            .map_err(|_| AppError::new("session.unavailable", "session actor is not running"))
    }

    /// Stops the running session and waits (bounded) until its summary is saved.
    pub fn shutdown_blocking(&self, timeout: Duration) {
        let (ack_tx, ack_rx) = std_mpsc::channel();
        if self.tx.send(ActorCmd::Shutdown(ack_tx)).is_ok() && ack_rx.recv_timeout(timeout).is_err()
        {
            log::warn!("session did not stop within {timeout:?}");
        }
    }
}

pub struct ActorContext {
    pub app: AppHandle,
    pub demo: bool,
    pub config: SharedConfig,
    pub effects: Effects,
    pub tracker: TrackerHandle,
    pub bridge: SharedBridge,
    pub tray: Option<Tray>,
}

pub fn spawn(ctx: ActorContext, rx: mpsc::UnboundedReceiver<ActorCmd>) {
    tauri::async_runtime::spawn(run(ctx, rx));
}

async fn run(ctx: ActorContext, mut rx: mpsc::UnboundedReceiver<ActorCmd>) {
    let mut actor = Actor {
        ctx,
        session: Session::new(),
        clock: Clock {
            origin: Instant::now(),
        },
        revision: 0,
        last_published: None,
        started: 0,
        shutting_down: false,
    };
    let profile = actor.profile();
    actor.session.set_idle_profile(profile);
    actor.publish(true);

    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            cmd = rx.recv() => match cmd {
                Some(cmd) => actor.handle(cmd),
                None => break,
            },
            _ = ticker.tick() => actor.tick(),
        }
    }
    log::info!("session actor stopped");
}

struct Clock {
    origin: Instant,
}

impl Clock {
    fn now(&self) -> Now {
        Now {
            mono_ms: u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX),
            wall_ms: wall_ms(),
        }
    }
}

fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

struct Actor {
    ctx: ActorContext,
    session: Session,
    clock: Clock,
    revision: u32,
    /// Last emitted snapshot (with its revision).
    last_published: Option<SessionSnapshot>,
    /// Sessions started by this process (part of the session id).
    started: u32,
    shutting_down: bool,
}

impl Actor {
    fn handle(&mut self, cmd: ActorCmd) {
        match cmd {
            ActorCmd::Start => self.start(),
            ActorCmd::Pause => self.apply(Input::Pause),
            ActorCmd::Resume => self.apply(Input::Resume),
            ActorCmd::TogglePause => {
                if self.session.phase() == Phase::Idle {
                    return;
                }
                if self.session.is_paused() {
                    self.apply(Input::Resume);
                } else {
                    self.apply(Input::Pause);
                }
            }
            ActorCmd::Stop => self.apply(Input::Stop),
            ActorCmd::Skip => self.apply(Input::SkipPhase),
            ActorCmd::ConfigChanged => {
                let profile = self.profile();
                self.session.set_idle_profile(profile);
                self.publish(false);
            }
            ActorCmd::GetSnapshot(reply) => {
                let mut snapshot = self.session.snapshot(self.clock.now());
                snapshot.revision = self.revision;
                let _ = reply.send(snapshot);
            }
            ActorCmd::Shutdown(ack) => {
                self.shutting_down = true;
                self.apply(Input::Stop);
                let _ = ack.send(());
            }
        }
    }

    fn start(&mut self) {
        if self.shutting_down {
            return;
        }
        if self.session.phase() != Phase::Idle {
            // Already running: keep the input idempotent, just re-publish.
            self.publish(true);
            return;
        }
        let profile = self.profile();
        self.started = self.started.wrapping_add(1);
        let session_id = format!("{}-{}-{}", wall_ms(), std::process::id(), self.started);
        log::info!(
            "starting session {session_id}: focus {} s, break {} s, demo {}",
            profile.focus_secs,
            profile.break_secs,
            profile.demo
        );
        // Reset before the core emits `Tracking(true)`; the tracker channel keeps the order.
        self.ctx.tracker.reset(profile.idle_after_secs);
        self.apply(Input::Start {
            session_id,
            profile,
        });
    }

    /// Demo profile, or the profile from the current config.
    fn profile(&self) -> TimingProfile {
        if self.ctx.demo {
            TimingProfile::demo()
        } else {
            TimingProfile::from_config(&read_config(&self.ctx.config))
        }
    }

    fn apply(&mut self, input: Input) {
        let effects = self.session.handle(self.clock.now(), input);
        self.ctx.effects.dispatch(effects, self.shutting_down);
        self.publish(true);
    }

    fn tick(&mut self) {
        if self.session.phase() == Phase::Idle {
            return;
        }
        let effects = self.session.handle(self.clock.now(), Input::Tick);
        let changed = !effects.is_empty();
        self.ctx.effects.dispatch(effects, self.shutting_down);
        self.publish(changed);
    }

    /// Emits `session://snapshot` when forced (after an input) or when the view changed.
    fn publish(&mut self, force: bool) {
        let mut snapshot = self.session.snapshot(self.clock.now());
        if !force {
            let unchanged = self.last_published.as_ref().is_some_and(|last| {
                let mut last = last.clone();
                last.revision = snapshot.revision;
                last == snapshot
            });
            if unchanged {
                return;
            }
        }
        self.revision = self.revision.wrapping_add(1);
        snapshot.revision = self.revision;
        if let Err(err) = self
            .ctx
            .app
            .emit(events::SESSION_SNAPSHOT, snapshot.clone())
        {
            log::warn!("cannot emit snapshot: {err}");
        }
        {
            let mut bridge = lock(&self.ctx.bridge);
            bridge.phase = snapshot.phase;
            bridge.remaining_secs = snapshot.remaining_secs;
        }
        // Non-blocking: the tray changes are posted to the main thread. Skipped while quitting,
        // when the main thread is busy shutting down and the menu is about to disappear.
        if !self.shutting_down
            && let Some(tray) = &self.ctx.tray
        {
            tray.update(&snapshot);
        }
        self.last_published = Some(snapshot);
    }
}
