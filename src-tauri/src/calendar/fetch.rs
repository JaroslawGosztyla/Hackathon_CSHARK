//! The blocking side of the calendars, over `focus_platform::calendar`: one fetch thread (the sources of a
//! sync in parallel, every ICS link on its own scoped thread), the Google sign-in threads and the sealed
//! files. Never runs on the async runtime (the platform uses blocking HTTP).
//!
//! Results carry the epoch of their source; the calendar thread drops results of a source that was
//! reconfigured or disconnected meanwhile. Error texts are redacted ([`super::model::redact`]) before they
//! leave this module: links and tokens never reach the status, the UI or the log.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, CalendarSource, Meeting};
use focus_platform::PlatformError;
use focus_platform::calendar::CalEvent;
use focus_platform::calendar::google::{GoogleClient, GooglePoll, GoogleSettings};
use focus_platform::calendar::ics::{self, IcsClient};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;

use super::CalendarCmd;
use super::model::redact;
use crate::lock;
use crate::teams::TeamsHandle;

/// Longest wait for the Teams worker to read the Microsoft calendar.
const MICROSOFT_TIMEOUT: Duration = Duration::from_secs(45);
/// A Google sign-in checks this often whether it was cancelled (a newer sign-in, a disconnect).
const LOGIN_POLL: Duration = Duration::from_secs(1);

/// One sync: the sources to read (each with its epoch) and the time window.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
    pub from_ms: i64,
    pub to_ms: i64,
    /// Epoch of the Microsoft source, when it is read.
    pub microsoft: Option<u64>,
    pub google: Option<GoogleJob>,
    pub ics: Option<IcsJob>,
}

impl Job {
    pub fn is_empty(&self) -> bool {
        self.microsoft.is_none() && self.google.is_none() && self.ics.is_none()
    }
}

#[derive(Clone)]
pub struct GoogleJob {
    pub epoch: u64,
    pub client_id: String,
    /// `calendar.googleClientSecret` (may be empty).
    pub client_secret: String,
    pub token_path: PathBuf,
}

impl std::fmt::Debug for GoogleJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Not confidential for a Desktop client, but still kept out of the log.
        f.debug_struct("GoogleJob")
            .field("epoch", &self.epoch)
            .field("client_id", &self.client_id)
            .field("client_secret", &!self.client_secret.trim().is_empty())
            .field("token_path", &self.token_path)
            .finish()
    }
}

#[derive(Clone)]
pub struct IcsJob {
    pub epoch: u64,
    pub urls: Vec<String>,
}

impl std::fmt::Debug for IcsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The links often carry a private token.
        f.debug_struct("IcsJob")
            .field("epoch", &self.epoch)
            .field("urls", &self.urls.len())
            .finish()
    }
}

/// The answer to a [`Job`].
#[derive(Debug)]
pub struct Fetched {
    pub id: u64,
    /// Wall clock when the sync finished.
    pub at_ms: i64,
    pub results: Vec<SourceResult>,
}

#[derive(Debug)]
pub struct SourceResult {
    pub source: CalendarSource,
    pub epoch: u64,
    /// The meetings, or a redacted English error.
    pub outcome: Result<Vec<Meeting>, String>,
    /// The signed-in account, when the source reports it (Google).
    pub account: Option<String>,
}

/// Handle to the fetch thread.
pub struct Fetcher {
    tx: Sender<Job>,
}

impl Fetcher {
    pub fn spawn(teams: TeamsHandle, done: Sender<CalendarCmd>) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("focus-calendar-fetch".into())
            .spawn(move || run(&teams, &rx, &done))?;
        Ok(Self { tx })
    }

    /// Queues a sync; `false` when the fetch thread is gone.
    pub fn fetch(&self, job: Job) -> bool {
        self.tx.send(job).is_ok()
    }
}

/// The Google client of the fetch thread (its access token lives in memory), kept while its source
/// epoch and client id stay the same.
struct GoogleSlot {
    epoch: u64,
    client: GoogleClient,
}

fn run(teams: &TeamsHandle, rx: &Receiver<Job>, done: &Sender<CalendarCmd>) {
    let ics_client = IcsClient::new();
    let mut google: Option<GoogleSlot> = None;
    while let Ok(job) = rx.recv() {
        if let Some(google_job) = &job.google {
            let settings = google_settings(
                &google_job.client_id,
                &google_job.client_secret,
                &google_job.token_path,
            );
            let fresh = google.as_ref().is_none_or(|slot| {
                slot.epoch != google_job.epoch || *slot.client.settings() != settings
            });
            if fresh {
                google = Some(GoogleSlot {
                    epoch: google_job.epoch,
                    client: GoogleClient::new(settings),
                });
            }
        }
        let google_client = match job.google {
            Some(_) => google.as_mut().map(|slot| &mut slot.client),
            None => None,
        };
        let results = fetch_all(teams, &ics_client, google_client, &job);
        let fetched = Fetched {
            id: job.id,
            at_ms: wall_ms(),
            results,
        };
        if done.send(CalendarCmd::Fetched(fetched)).is_err() {
            return;
        }
    }
}

/// Reads the sources of `job` in parallel.
fn fetch_all(
    teams: &TeamsHandle,
    ics_client: &IcsClient,
    google_client: Option<&mut GoogleClient>,
    job: &Job,
) -> Vec<SourceResult> {
    let (from_ms, to_ms) = (job.from_ms, job.to_ms);
    std::thread::scope(|scope| {
        let microsoft = job.microsoft.map(|epoch| {
            scope.spawn(move || SourceResult {
                source: CalendarSource::Microsoft,
                epoch,
                outcome: teams
                    .calendar_view(from_ms, to_ms, MICROSOFT_TIMEOUT)
                    .map(|events| meetings(CalendarSource::Microsoft, &events))
                    .map_err(|err| redact(&err)),
                account: None,
            })
        });
        let google = job
            .google
            .as_ref()
            .zip(google_client)
            .map(|(google_job, client)| {
                scope.spawn(move || {
                    let outcome = client
                        .events(from_ms, to_ms)
                        .map(|events| meetings(CalendarSource::Google, &events))
                        .map_err(|err| redact(&err.to_string()));
                    let account = outcome.as_ref().ok().and_then(|_| client.account());
                    SourceResult {
                        source: CalendarSource::Google,
                        epoch: google_job.epoch,
                        outcome,
                        account,
                    }
                })
            });
        let ics = job.ics.as_ref().map(|ics_job| {
            let links: Vec<_> = ics_job
                .urls
                .iter()
                .map(|url| {
                    scope.spawn(move || {
                        ics_client
                            .events(url, from_ms, to_ms)
                            .map(|events| meetings(CalendarSource::Ics, &events))
                            .map_err(|err| redact(&err.to_string()))
                    })
                })
                .collect();
            (ics_job.epoch, links)
        });

        let mut results = Vec::new();
        for handle in [microsoft, google].into_iter().flatten() {
            match handle.join() {
                Ok(result) => results.push(result),
                Err(_) => log::error!("calendar: a fetch thread panicked"),
            }
        }
        if let Some((epoch, links)) = ics {
            let outcomes = links
                .into_iter()
                .map(|link| {
                    link.join()
                        .unwrap_or_else(|_| Err("the download failed unexpectedly".to_owned()))
                })
                .collect();
            results.push(SourceResult {
                source: CalendarSource::Ics,
                epoch,
                outcome: combine_links(outcomes),
                account: None,
            });
        }
        results
    })
}

fn meetings(source: CalendarSource, events: &[CalEvent]) -> Vec<Meeting> {
    events
        .iter()
        .map(|event| event.to_meeting(source))
        .collect()
}

/// All links' meetings; an error only when every link failed (the working links count; the failed ones
/// are logged by number, `link 2: …`).
fn combine_links(outcomes: Vec<Result<Vec<Meeting>, String>>) -> Result<Vec<Meeting>, String> {
    let total = outcomes.len();
    let mut meetings = Vec::new();
    let mut errors = Vec::new();
    for (index, outcome) in outcomes.into_iter().enumerate() {
        match outcome {
            Ok(found) => meetings.extend(found),
            Err(error) => errors.push(format!("link {}: {error}", index + 1)),
        }
    }
    if errors.is_empty() {
        Ok(meetings)
    } else if errors.len() == total {
        Err(errors.join("; "))
    } else {
        log::warn!(
            "calendar: {} of {total} ICS links failed: {}",
            errors.len(),
            errors.join("; ")
        );
        Ok(meetings)
    }
}

/// Settings from `calendar.googleClientId` / `googleClientSecret` (trimmed; an empty secret is not sent).
fn google_settings(client_id: &str, client_secret: &str, token_path: &Path) -> GoogleSettings {
    GoogleSettings {
        client_id: client_id.trim().to_owned(),
        client_secret: client_secret.trim().to_owned(),
        token_path: token_path.to_path_buf(),
    }
}

// -- sealed files ------------------------------------------------------------------------------------

/// The stored ICS links (empty when there are none or they cannot be read).
pub fn load_ics_urls(path: &Path) -> Vec<String> {
    ics::load_urls(path).unwrap_or_else(|err| {
        log::warn!(
            "calendar: cannot read the stored ICS links: {}",
            redact(&err.to_string())
        );
        Vec::new()
    })
}

/// Stores the links sealed (an empty list deletes the file); returns them as stored (`webcal://` saved as
/// `https://`).
pub fn save_ics_urls(path: &Path, urls: &[String]) -> Result<Vec<String>, AppError> {
    ics::save_urls(path, urls).map_err(|err| match err {
        PlatformError::Invalid(detail) => AppError {
            code: "calendar.icsUrl".into(),
            message: redact(&detail),
            field: Some("urls".into()),
        },
        other => AppError::new(
            "calendar.icsSave",
            format!("cannot store the ICS links: {}", redact(&other.to_string())),
        ),
    })
}

pub fn delete_ics_urls(path: &Path) -> Result<(), String> {
    ics::delete_urls(path).map_err(|err| err.to_string())
}

/// `(has a stored token, its account)`; reads the sealed file.
pub fn google_token(token_path: &Path) -> (bool, Option<String>) {
    let client = GoogleClient::new(google_settings("", "", token_path));
    (client.has_token(), client.account())
}

/// Revokes the Google token (best effort, up to the platform's 10 s timeout) and deletes it. Revoking needs no
/// client credentials.
pub fn disconnect_google(token_path: &Path) -> Result<(), String> {
    GoogleClient::new(google_settings("", "", token_path))
        .disconnect()
        .map_err(|err| redact(&err.to_string()))
}

// -- Google sign-in ------------------------------------------------------------------------------------

/// What a Google sign-in thread needs.
pub struct GoogleLogin {
    pub app: AppHandle,
    pub client_id: String,
    /// `calendar.googleClientSecret` (may be empty).
    pub client_secret: String,
    pub token_path: PathBuf,
    /// This sign-in's number; it gives up as soon as `current` moves on.
    pub attempt: u64,
    pub current: Arc<AtomicU64>,
    /// Gets `CalendarCmd::GoogleDone` when the sign-in ends.
    pub done: Sender<CalendarCmd>,
    /// Answered once the browser was opened (or the sign-in could not start).
    pub reply: oneshot::Sender<Result<(), AppError>>,
}

/// Starts the sign-in on its own thread: loopback listener + PKCE, the consent page in the system
/// browser, then the token exchange. `false` when the thread could not start (the reply says why).
pub fn spawn_google_login(login: GoogleLogin) -> bool {
    let GoogleLogin {
        app,
        client_id,
        client_secret,
        token_path,
        attempt,
        current,
        done,
        reply,
    } = login;
    let settings = google_settings(&client_id, &client_secret, &token_path);
    // Shared so the reply can still be answered when the thread does not start.
    let reply = Arc::new(Mutex::new(Some(reply)));
    let thread_reply = Arc::clone(&reply);
    let spawned = std::thread::Builder::new()
        .name("focus-google-login".into())
        .spawn(move || {
            let Some(reply) = lock(&thread_reply).take() else {
                return;
            };
            let cancelled = || current.load(Ordering::SeqCst) != attempt;
            let result = google_login(&app, settings, reply, &cancelled);
            let _ = done.send(CalendarCmd::GoogleDone { attempt, result });
        });
    match spawned {
        Ok(_) => true,
        Err(err) => {
            log::warn!("calendar: cannot start the Google sign-in: {err}");
            if let Some(reply) = lock(&reply).take() {
                let _ = reply.send(Err(AppError::new(
                    "calendar.googleAuth",
                    format!("cannot start the Google sign-in: {err}"),
                )));
            }
            false
        }
    }
}

/// The whole sign-in, blocking. `reply` is answered as soon as the browser was opened.
fn google_login(
    app: &AppHandle,
    settings: GoogleSettings,
    reply: oneshot::Sender<Result<(), AppError>>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<String>, String> {
    let mut client = GoogleClient::new(settings);
    let login = match client.begin_login() {
        Ok(login) => login,
        Err(err) => {
            let err = redact(&err.to_string());
            let _ = reply.send(Err(AppError::new(
                "calendar.googleAuth",
                format!("cannot start the Google sign-in: {err}"),
            )));
            return Err(err);
        }
    };
    if let Err(err) = app.opener().open_url(login.auth_url.clone(), None::<&str>) {
        let err = format!("cannot open the browser: {err}");
        let _ = reply.send(Err(AppError::new("calendar.googleAuth", err.clone())));
        return Err(err);
    }
    let _ = reply.send(Ok(()));
    loop {
        if cancelled() {
            return Err("the sign-in was cancelled".into());
        }
        match client.poll_login(&login, LOGIN_POLL) {
            Ok(GooglePoll::Pending) => {}
            Ok(GooglePoll::Done { account }) => return Ok(account),
            Ok(GooglePoll::Failed(message)) => return Err(redact(&message)),
            Err(err) => return Err(redact(&err.to_string())),
        }
    }
}

fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meeting(id: &str) -> Meeting {
        CalEvent {
            id: id.into(),
            subject: format!("Meeting {id}"),
            start_ms: 1,
            end_ms: 2,
            online: false,
            all_day: false,
            busy: true,
        }
        .to_meeting(CalendarSource::Ics)
    }

    #[test]
    fn events_become_meetings_of_their_source() {
        let events = [CalEvent {
            id: "AAMk".into(),
            subject: "Planowanie".into(),
            start_ms: 10,
            end_ms: 20,
            online: true,
            all_day: false,
            busy: true,
        }];
        let found = meetings(CalendarSource::Microsoft, &events);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, CalendarSource::Microsoft);
        assert!(found[0].id.starts_with("ms-"), "{}", found[0].id);
        assert_eq!(
            (found[0].start_ms, found[0].end_ms, found[0].online),
            (10, 20, true)
        );
    }

    #[test]
    fn ics_links_fail_only_together() {
        assert_eq!(
            combine_links(vec![Ok(vec![meeting("a")]), Err("HTTP 404".into())]),
            Ok(vec![meeting("a")])
        );
        assert_eq!(
            combine_links(vec![Err("HTTP 404".into()), Err("timed out".into())]),
            Err("link 1: HTTP 404; link 2: timed out".into())
        );
        assert_eq!(combine_links(Vec::new()), Ok(Vec::new()));
    }

    #[test]
    fn debug_hides_the_links() {
        let job = IcsJob {
            epoch: 3,
            urls: vec!["https://calendar.example/private-token.ics".into()],
        };
        let debug = format!("{job:?}");
        assert!(!debug.contains("private-token"));
        assert!(debug.contains("epoch: 3"));
    }

    #[test]
    fn google_settings_carry_the_trimmed_client_secret() {
        let settings = google_settings(
            " 1-x.apps.googleusercontent.com ",
            " GOCSPX-abc_123 ",
            Path::new("t.bin"),
        );
        assert_eq!(settings.client_id, "1-x.apps.googleusercontent.com");
        assert_eq!(settings.client_secret, "GOCSPX-abc_123");
        assert_eq!(settings.token_path, PathBuf::from("t.bin"));
        assert!(
            google_settings("1-x", "", Path::new("t.bin"))
                .client_secret
                .is_empty()
        );
    }

    #[test]
    fn debug_hides_the_google_client_secret() {
        let job = GoogleJob {
            epoch: 1,
            client_id: "1-x.apps.googleusercontent.com".into(),
            client_secret: "GOCSPX-hidden".into(),
            token_path: PathBuf::from("t.bin"),
        };
        let debug = format!("{job:?}");
        assert!(!debug.contains("GOCSPX-hidden"), "{debug}");
        assert!(debug.contains("client_secret: true"), "{debug}");
    }

    #[test]
    fn invalid_links_map_to_a_field_error() {
        let path =
            std::env::temp_dir().join(format!("focusmode-ics-invalid-{}.bin", std::process::id()));
        let err = save_ics_urls(&path, &["http://calendar.example/secret.ics".to_owned()])
            .expect_err("plain http is refused");
        assert_eq!(err.code, "calendar.icsUrl");
        assert_eq!(err.field.as_deref(), Some("urls"));
        assert!(!err.message.contains("secret"), "{}", err.message);
        assert!(!path.exists());
    }
}
