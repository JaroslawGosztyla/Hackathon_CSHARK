//! X on the main window (`Config::close_to_tray`): hide to the tray, quit, or ask once.
//!
//! The close is always prevented first. Undecided (`None`), the first X sends `app://close-requested` to
//! the main window and the UI answers with `resolve_close`. A second X while that question is still open
//! (more than [`SECOND_CLOSE_AFTER`] later, so a double click does not count) hides the window: the safe
//! default that keeps the music and the session running. The UI can close the question without answering
//! (Escape) and cannot tell the shell, so a question older than [`QUESTION_EXPIRES`] counts as closed and
//! the next X asks again. Quitting goes through the normal shutdown, which stops the session and lowers
//! every shield.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use focus_core::api::events;
use tauri::{AppHandle, Emitter, EventTarget, Manager};

use crate::{AppState, MAIN_WINDOW, lock, read_config};

/// A second X sooner than this after the question is part of the same click (ignored).
const SECOND_CLOSE_AFTER: Duration = Duration::from_secs(1);
/// An X later than this after the question asks again (the question was dismissed or ignored).
const QUESTION_EXPIRES: Duration = Duration::from_secs(60);

/// The open "minimize to tray?" question, if any.
#[derive(Default)]
pub struct ClosePrompt {
    asked_at: Mutex<Option<Instant>>,
}

impl ClosePrompt {
    /// The UI answered (`resolve_close`).
    pub fn resolved(&self) {
        *lock(&self.asked_at) = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseAction {
    Hide,
    Quit,
    Ask,
    Ignore,
}

fn decide(close_to_tray: Option<bool>, asked_at: Option<Instant>, now: Instant) -> CloseAction {
    match (close_to_tray, asked_at) {
        (Some(true), _) => CloseAction::Hide,
        (Some(false), _) => CloseAction::Quit,
        (None, None) => CloseAction::Ask,
        (None, Some(at)) => {
            let since = now.saturating_duration_since(at);
            if since <= SECOND_CLOSE_AFTER {
                CloseAction::Ignore
            } else if since <= QUESTION_EXPIRES {
                CloseAction::Hide
            } else {
                CloseAction::Ask
            }
        }
    }
}

/// `CloseRequested` of the main window, after `prevent_close` (main thread).
pub fn on_main_close_requested(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        // Still starting up: keep the old behaviour (hide).
        hide_main_window(app);
        return;
    };
    let close_to_tray = read_config(&state.config).close_to_tray;
    let action = {
        let mut asked_at = lock(&state.close.asked_at);
        let now = Instant::now();
        let action = decide(close_to_tray, *asked_at, now);
        if action == CloseAction::Ask {
            *asked_at = Some(now);
        }
        action
    };
    match action {
        CloseAction::Hide => hide_main_window(app),
        CloseAction::Quit => {
            log::info!("main window closed: quitting (closeToTray = false)");
            crate::shutdown(app);
            app.exit(0);
        }
        CloseAction::Ask => {
            let asked = app.emit_to(
                EventTarget::webview_window(MAIN_WINDOW),
                events::APP_CLOSE_REQUESTED,
                (),
            );
            if let Err(err) = asked {
                log::warn!("cannot ask how to close the window, hiding it: {err}");
                state.close.resolved();
                hide_main_window(app);
            }
        }
        CloseAction::Ignore => {}
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW)
        && let Err(err) = window.hide()
    {
        log::warn!("cannot hide the main window: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decided_close_hides_or_quits() {
        let now = Instant::now();
        assert_eq!(decide(Some(true), None, now), CloseAction::Hide);
        assert_eq!(decide(Some(true), Some(now), now), CloseAction::Hide);
        assert_eq!(decide(Some(false), None, now), CloseAction::Quit);
    }

    #[test]
    fn undecided_close_asks_once_then_hides() {
        let asked = Instant::now();
        assert_eq!(decide(None, None, asked), CloseAction::Ask);
        // Double click: same gesture.
        assert_eq!(
            decide(None, Some(asked), asked + Duration::from_millis(300)),
            CloseAction::Ignore
        );
        // A deliberate second X while the question is open: safe default.
        assert_eq!(
            decide(None, Some(asked), asked + Duration::from_secs(3)),
            CloseAction::Hide
        );
        assert_eq!(
            decide(None, Some(asked), asked + QUESTION_EXPIRES),
            CloseAction::Hide
        );
        // The question was dismissed (Escape) or ignored long ago: ask again.
        assert_eq!(
            decide(
                None,
                Some(asked),
                asked + QUESTION_EXPIRES + Duration::from_secs(1)
            ),
            CloseAction::Ask
        );
    }
}
