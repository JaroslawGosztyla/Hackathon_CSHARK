//! Shared blocking HTTPS client settings and transport error mapping.
//!
//! Error messages never contain request URLs (calendar links and OAuth redirects carry secrets).

use std::sync::OnceLock;
use std::time::Duration;

use crate::PlatformError;

/// Timeout of every request (connect and whole request).
pub(crate) const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// A blocking client built on first use (building it has a cost: a runtime thread).
pub(crate) struct LazyClient {
    cell: OnceLock<reqwest::blocking::Client>,
    max_redirects: usize,
    same_origin: bool,
}

impl std::fmt::Debug for LazyClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyClient")
            .field("max_redirects", &self.max_redirects)
            .field("same_origin", &self.same_origin)
            .finish_non_exhaustive()
    }
}

impl LazyClient {
    pub(crate) const fn new(max_redirects: usize) -> Self {
        Self {
            cell: OnceLock::new(),
            max_redirects,
            same_origin: false,
        }
    }

    /// Like [`LazyClient::new`], but a redirect is followed only to the origin (scheme, host, port) of the first
    /// request; any other redirect is returned as the 3xx response. reqwest drops `Authorization` on a cross-host
    /// redirect but not custom credential headers (GitLab's `PRIVATE-TOKEN`), so token-carrying clients use this.
    pub(crate) const fn same_origin(max_redirects: usize) -> Self {
        Self {
            cell: OnceLock::new(),
            max_redirects,
            same_origin: true,
        }
    }

    /// HTTPS only (also for redirects), 10 s timeouts, at most `max_redirects` redirects.
    pub(crate) fn get(&self) -> Result<&reqwest::blocking::Client, PlatformError> {
        if let Some(client) = self.cell.get() {
            return Ok(client);
        }
        let policy = if self.same_origin {
            let max_redirects = self.max_redirects;
            reqwest::redirect::Policy::custom(move |attempt| {
                let same = attempt
                    .previous()
                    .first()
                    .is_some_and(|first| first.origin() == attempt.url().origin());
                if same && attempt.previous().len() <= max_redirects {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            })
        } else {
            reqwest::redirect::Policy::limited(self.max_redirects)
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .connect_timeout(HTTP_TIMEOUT)
            .user_agent(concat!("FocusMode/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .redirect(policy)
            .build()
            .map_err(network_error)?;
        // Another initializer cannot race us (`&self` on one thread), but stay correct either way.
        let _ = self.cell.set(client);
        self.cell
            .get()
            .ok_or_else(|| PlatformError::Network("HTTP client unavailable".into()))
    }
}

/// Transport error → [`PlatformError::Network`] with the cause chain, without the URL.
pub(crate) fn network_error(err: reqwest::Error) -> PlatformError {
    let kind = if err.is_timeout() {
        "timed out"
    } else if err.is_connect() {
        "connection failed"
    } else if err.is_redirect() {
        "too many redirects"
    } else {
        "request failed"
    };
    let err = err.without_url();
    let mut detail = err.to_string();
    let mut source = std::error::Error::source(&err);
    let mut depth = 0;
    while let Some(cause) = source
        && depth < 4
    {
        detail.push_str(": ");
        detail.push_str(&cause.to_string());
        source = std::error::Error::source(cause);
        depth += 1;
    }
    PlatformError::Network(format!("{kind}: {detail}"))
}

/// The first line of a server-provided description (identity platforms put trace ids on later lines).
pub(crate) fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("").trim()
}
