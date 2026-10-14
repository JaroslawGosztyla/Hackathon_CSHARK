//! Weekly stats and heatmaps (ADR-023): `get_weekly_stats` reads `history.jsonl` line by line (bounded, off the async
//! workers) into `focus_core::stats::WeekAggregator`, and every change of the stored history or of the app labels
//! (`Config.app_categories`) is announced as `history://changed` with a growing revision (ADR-012: listen, then the
//! getter, then only newer revisions).
//!
//! Only local data: the sessions in `history.jsonl`, the labels in `config.json` and the time zone of this computer
//! (`focus_platform::clock::utc_offset_minutes_at`). Nothing is logged except counts.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, HistoryChanged, WeeklyStats, events};
use focus_core::stats::{WeekAggregator, WeekQuery, parse_local_date};
use focus_platform::clock::utc_offset_minutes_at;
use tauri::{AppHandle, Emitter};

use crate::{AppState, read_config};

/// `AppError.code` for a `weekStart` that is not a `YYYY-MM-DD` date (field `weekStart`).
pub const ERR_WEEK: &str = "stats.week";
/// `AppError.code` when the history cannot be read.
const ERR_HISTORY: &str = "persistence.history";
/// Longest `weekStart` looked at (a date is 10 characters).
const MAX_WEEK_START_LEN: usize = 32;

/// The history revision: grows with every change (the first answer of a run already has 1).
static REVISION: AtomicU32 = AtomicU32::new(1);

/// The stored history or the app labels changed: a newer revision, announced to every window.
pub fn changed(app: &AppHandle) {
    let revision = REVISION.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
    if let Err(err) = app.emit(events::HISTORY_CHANGED, HistoryChanged { revision }) {
        log::warn!("cannot emit history change: {err}");
    }
}

/// The ISO week that contains `week_start` (`YYYY-MM-DD`), from the stored history.
pub async fn weekly_stats(state: &AppState, week_start: &str) -> Result<WeeklyStats, AppError> {
    let day = (week_start.len() <= MAX_WEEK_START_LEN)
        .then(|| parse_local_date(week_start.trim()))
        .flatten()
        .ok_or_else(|| AppError {
            code: ERR_WEEK.to_owned(),
            message: "weekStart must be a date YYYY-MM-DD".to_owned(),
            field: Some("weekStart".to_owned()),
        })?;
    // Read before the file: a change while it is read comes with a newer revision, so the UI asks again.
    let revision = REVISION.load(Ordering::SeqCst);
    let config = read_config(&state.config);
    let persistence = state.persistence.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let now_ms = wall_ms();
        let query = WeekQuery {
            day,
            now_ms,
            revision,
            history_off: !config.keep_history,
        };
        // Outside Windows (or for an instant Windows cannot convert) the offset of now, else UTC: best effort (ADR-008).
        let fallback = utc_offset_minutes_at(now_ms).unwrap_or(0);
        let mut aggregator = WeekAggregator::new(query, &config.app_categories, |utc_ms| {
            utc_offset_minutes_at(utc_ms).unwrap_or(fallback)
        });
        persistence.scan_history(|summary| aggregator.add(&summary))?;
        Ok::<_, anyhow::Error>(aggregator.finish())
    })
    .await
    .map_err(|err| AppError::new(ERR_HISTORY, err.to_string()))?
    .map_err(|err| {
        AppError::new(
            ERR_HISTORY,
            format!("cannot read the session history: {err:#}"),
        )
    })
}

/// Wall clock in Unix milliseconds (which week is the current one).
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
