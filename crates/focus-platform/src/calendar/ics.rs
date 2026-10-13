//! ICS / iCal subscription links (RFC 5545): any calendar that can publish one — Exchange on-premises and
//! Outlook "publish calendar", Google "secret address", iCloud, Zoho, Zimbra, Nextcloud, Proton, …
//!
//! * [`normalize_url`]: `webcal://` → `https://`; only HTTPS links are accepted.
//! * [`IcsClient::fetch`]: HTTPS only (also for redirects, at most 3), 10 s timeout, at most 5 MB.
//! * [`parse_events`] (pure): line unfolding, VEVENT (`DTSTART`/`DTEND`/`DURATION`/`SUMMARY`/`STATUS`/`TRANSP`/
//!   `UID`/`RRULE`/`RDATE`/`EXDATE`/`RECURRENCE-ID`), VTIMEZONE (STANDARD/DAYLIGHT with their own RRULEs),
//!   `TZID`s with or without a VTIMEZONE (Windows names too, see [`TimeZone::by_name`]), UTC `Z` times and
//!   floating times (= the given local zone). Recurring events are expanded within the range (at most
//!   [`super::MAX_EVENTS`] occurrences in total), without the `EXDATE`s and with `RECURRENCE-ID` overrides.
//!   Hostile or broken files stay cheap: all rules of a file share one expansion budget, `RDATE` / `EXDATE` values
//!   far from the range are skipped before any conversion, nesting is followed 16 deep and an overly complex
//!   VTIMEZONE is replaced by its TZID name.
//! * [`load_urls`] / [`save_urls`]: the links themselves, DPAPI-encrypted (they are secrets: whoever has the link
//!   can read the calendar). Links are never logged and never part of an error message.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use super::rrule::{self, Rule};
use super::time::{MS_PER_DAY, parse_duration, parse_ical_date, parse_ical_date_time};
use super::tz::{Observance, TimeZone};
use super::{CalEvent, MAX_EVENTS, clean_subject, fnv1a, looks_online, meetings_only};
use crate::PlatformError;
use crate::net::{LazyClient, network_error};
use crate::secrets;

/// Largest calendar file accepted.
pub const MAX_BYTES: usize = 5 * 1024 * 1024;
/// Most links stored.
pub const MAX_URLS: usize = 20;
/// Longest link accepted.
pub const MAX_URL_LEN: usize = 2048;
const MAX_REDIRECTS: usize = 3;
/// Expansion margin around the requested range (UTC offsets are at most ±14 h).
const WALL_MARGIN_MS: i64 = 2 * MS_PER_DAY;
/// Extra occurrences expanded per event for that margin (at most daily rules: a few days' worth).
const EXPANSION_SLACK: usize = 8;
/// Recurrence periods walked for one whole file (all rules together, see [`rrule::expand_budgeted`]): far more
/// than real calendars need, small enough that a hostile file is parsed in about a second.
const EXPANSION_BUDGET: u64 = 3_000_000;
/// Deepest component nesting followed (VCALENDAR > VTIMEZONE > STANDARD is 3); deeper BEGINs are ignored.
const MAX_NESTING: usize = 16;
/// Most complex VTIMEZONE used (see [`zone_cost`]); a more complex one falls back to its TZID name. Every time
/// conversion walks all of a zone's onsets, so this bounds the cost per event.
const MAX_ZONE_COST: usize = 512;

/// Turns a user-entered link into the HTTPS URL to fetch (`webcal://` / `webcals://` → `https://`).
/// The error never repeats the link.
pub fn normalize_url(raw: &str) -> Result<String, PlatformError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PlatformError::Invalid("the calendar link is empty".into()));
    }
    if trimmed.len() > MAX_URL_LEN {
        return Err(PlatformError::Invalid(format!(
            "the calendar link is longer than {MAX_URL_LEN} characters"
        )));
    }
    let lower = trimmed.to_ascii_lowercase();
    let with_https = ["webcals://", "webcal://"]
        .iter()
        .find_map(|scheme| {
            lower
                .starts_with(scheme)
                .then(|| format!("https://{}", &trimmed[scheme.len()..]))
        })
        .unwrap_or_else(|| trimmed.to_owned());
    let url = reqwest::Url::parse(&with_https)
        .map_err(|_| PlatformError::Invalid("the calendar link is not a valid URL".into()))?;
    if url.scheme() != "https" {
        return Err(PlatformError::Invalid(
            "the calendar link must start with https:// or webcal://".into(),
        ));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(PlatformError::Invalid(
            "the calendar link has no host".into(),
        ));
    }
    Ok(url.to_string())
}

/// The stored links (empty when none are stored).
pub fn load_urls(path: &Path) -> Result<Vec<String>, PlatformError> {
    let Some(plain) = secrets::read_sealed(path)? else {
        return Ok(Vec::new());
    };
    serde_json::from_slice(&plain)
        .map_err(|err| PlatformError::Io(format!("stored calendar links are corrupted: {err}")))
}

/// Validates ([`normalize_url`]), de-duplicates and stores the links encrypted; an empty list deletes the file.
/// Returns the stored links.
pub fn save_urls(path: &Path, urls: &[String]) -> Result<Vec<String>, PlatformError> {
    let mut stored: Vec<String> = Vec::new();
    for (index, raw) in urls.iter().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let url = normalize_url(raw).map_err(|err| match err {
            PlatformError::Invalid(detail) => {
                PlatformError::Invalid(format!("link {}: {detail}", index + 1))
            }
            other => other,
        })?;
        if !stored.contains(&url) {
            stored.push(url);
        }
    }
    if stored.len() > MAX_URLS {
        return Err(PlatformError::Invalid(format!(
            "at most {MAX_URLS} calendar links"
        )));
    }
    if stored.is_empty() {
        secrets::delete_sealed(path)?;
        return Ok(stored);
    }
    if !secrets::SUPPORTED {
        return Err(PlatformError::Unsupported);
    }
    let plain = serde_json::to_vec(&stored)
        .map_err(|err| PlatformError::Io(format!("calendar links encode: {err}")))?;
    secrets::write_sealed(path, &plain)?;
    Ok(stored)
}

/// Deletes the stored links.
pub fn delete_urls(path: &Path) -> Result<(), PlatformError> {
    secrets::delete_sealed(path)
}

/// Fetches ICS links. Blocking; create, use and drop it on a worker thread.
pub struct IcsClient {
    http: LazyClient,
}

impl Default for IcsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl IcsClient {
    /// Cheap: no I/O (the HTTP client is built on first use).
    pub fn new() -> Self {
        Self {
            http: LazyClient::new(MAX_REDIRECTS),
        }
    }

    /// Downloads one calendar file (≤ [`MAX_BYTES`]) and checks that it is iCalendar data.
    pub fn fetch(&self, url: &str) -> Result<Vec<u8>, PlatformError> {
        let url = normalize_url(url)?;
        let response = self
            .http
            .get()?
            .get(url.as_str())
            .header(reqwest::header::ACCEPT, "text/calendar, */*;q=0.5")
            .send()
            .map_err(network_error)?;
        let status = response.status();
        if !status.is_success() {
            let code = status.as_u16();
            return Err(match code {
                401 | 403 => PlatformError::Auth(format!(
                    "the calendar link was rejected (HTTP {code}); publish it again and paste the new link"
                )),
                404 | 410 => PlatformError::Network(format!(
                    "the calendar link no longer exists (HTTP {code})"
                )),
                _ => PlatformError::Network(format!("the calendar server answered HTTP {code}")),
            });
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_BYTES as u64)
        {
            return Err(too_large());
        }
        let mut body = Vec::new();
        response
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|err| {
                // The underlying error may carry the URL; keep only its kind.
                PlatformError::Network(format!("reading the calendar failed ({})", err.kind()))
            })?;
        if body.len() > MAX_BYTES {
            return Err(too_large());
        }
        if !looks_like_icalendar(&body) {
            return Err(PlatformError::Network(
                "the link did not return an iCalendar (.ics) file".into(),
            ));
        }
        Ok(body)
    }

    /// Meetings of one link that overlap `[start_ms, end_ms)` (floating times in the system zone).
    pub fn events(
        &self,
        url: &str,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<CalEvent>, PlatformError> {
        let data = self.fetch(url)?;
        let local = TimeZone::local();
        Ok(meetings_only(
            parse_events(&data, start_ms, end_ms, &local),
            start_ms,
            end_ms,
        ))
    }
}

fn too_large() -> PlatformError {
    PlatformError::Network(format!(
        "the calendar file is larger than {} MB",
        MAX_BYTES / 1024 / 1024
    ))
}

/// The data starts (after a BOM / whitespace) with `BEGIN:VCALENDAR`.
pub fn looks_like_icalendar(data: &[u8]) -> bool {
    let data = data.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(data);
    let start = data
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(data.len());
    data[start..]
        .get(..15)
        .is_some_and(|head| head.eq_ignore_ascii_case(b"BEGIN:VCALENDAR"))
}

/// All event occurrences that overlap `[start_ms, end_ms)`, sorted by start, at most [`MAX_EVENTS`].
/// Cancelled events / occurrences are left out; all-day and free events are included with their flags
/// (use [`meetings_only`] to drop them). Floating times are in `local`.
pub fn parse_events(data: &[u8], start_ms: i64, end_ms: i64, local: &TimeZone) -> Vec<CalEvent> {
    if end_ms <= start_ms {
        return Vec::new();
    }
    let document = Document::parse(data);
    let zones = document.zones(local);
    let mut out = Vec::new();
    let mut budget = EXPANSION_BUDGET;

    // RECURRENCE-ID overrides replace the occurrence they name.
    let mut overridden: HashMap<&str, Exclusions> = HashMap::new();
    for event in &document.events {
        if let Some(recurrence_id) = &event.recurrence_id {
            let zone = zones.zone_of(event.start.as_ref().unwrap_or(recurrence_id));
            overridden
                .entry(event.uid.as_str())
                .or_default()
                .add(recurrence_id, &zones, zone);
        }
    }

    for event in &document.events {
        if out.len() >= MAX_EVENTS {
            break;
        }
        if event.cancelled {
            continue;
        }
        let Some(start) = &event.start else {
            continue;
        };
        let zone = zones.zone_of(start);
        let start_wall = start.wall_ms();
        let span = event.span(&zones, zone);
        let lo_wall = start_ms
            .saturating_sub(span.max_ms())
            .saturating_sub(WALL_MARGIN_MS);
        let hi_wall = end_ms.saturating_add(WALL_MARGIN_MS);

        let mut starts: Vec<i64> = Vec::new();
        let rule = event
            .recurrence_id
            .is_none()
            .then(|| event.rrule.as_deref().and_then(Rule::parse))
            .flatten();
        match (&rule, &event.rrule) {
            (Some(rule), _) => {
                let to_utc = |wall: i64| zone.wall_to_utc(wall);
                // A few more than the cap: occurrences in the margin before the range are dropped below.
                let max = MAX_EVENTS + EXPANSION_SLACK;
                let had_budget = budget > 0;
                starts = rrule::expand_budgeted(
                    rule,
                    start_wall,
                    lo_wall,
                    hi_wall,
                    &to_utc,
                    max,
                    &mut budget,
                );
                if had_budget && budget == 0 {
                    log::debug!(
                        "calendar: recurrence expansion budget used up, later rules keep only DTSTART"
                    );
                }
            }
            (None, Some(_)) if event.recurrence_id.is_none() => {
                log::debug!("calendar: unsupported RRULE, keeping only the first occurrence");
                starts.push(start_wall);
            }
            _ => starts.push(start_wall),
        }
        // Values more than a day (> any UTC offset) outside the expansion window cannot matter; skipping them
        // before converting keeps huge RDATE / EXDATE lists linear.
        let near = |value: &DateValue| {
            let wall = value.wall_ms();
            wall >= lo_wall.saturating_sub(MS_PER_DAY) && wall <= hi_wall.saturating_add(MS_PER_DAY)
        };
        if event.recurrence_id.is_none() {
            for rdate in event.rdates.iter().filter(|rdate| near(rdate)) {
                let wall = match rdate {
                    DateValue::Utc(utc) => zone.utc_to_wall(*utc),
                    other => other.wall_ms(),
                };
                if (lo_wall..=hi_wall).contains(&wall) {
                    starts.push(wall);
                }
            }
            starts.sort_unstable();
            starts.dedup();
        }

        let mut excluded = Exclusions::default();
        for exdate in event.exdates.iter().filter(|exdate| near(exdate)) {
            excluded.add(exdate, &zones, zone);
        }
        let overrides = if event.recurrence_id.is_none() {
            overridden.get(event.uid.as_str())
        } else {
            None
        };

        for wall in starts {
            let utc_start = zone.wall_to_utc(wall);
            let day = wall.div_euclid(MS_PER_DAY);
            if excluded.contains(utc_start, day)
                || overrides.is_some_and(|o| o.contains(utc_start, day))
            {
                continue;
            }
            let utc_end = span.end_for(wall, utc_start, zone);
            if utc_start >= end_ms || utc_end.max(utc_start + 1) <= start_ms {
                continue;
            }
            out.push(CalEvent {
                id: event.uid.clone(),
                subject: event.summary.clone(),
                start_ms: utc_start,
                end_ms: utc_end,
                online: event.online,
                all_day: event.all_day || matches!(start, DateValue::Date(_)),
                busy: event.busy(),
            });
            if out.len() >= MAX_EVENTS {
                break;
            }
        }
    }
    out.sort_by(|a, b| (a.start_ms, a.end_ms, &a.id).cmp(&(b.start_ms, b.end_ms, &b.id)));
    out.truncate(MAX_EVENTS);
    out
}

// -- document model ------------------------------------------------------------------------------------------

/// A DATE / DATE-TIME value as written.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DateValue {
    Utc(i64),
    Wall { wall_ms: i64, tzid: Option<String> },
    Date(i64),
}

impl DateValue {
    /// Local wall time (UTC values: the UTC time itself).
    fn wall_ms(&self) -> i64 {
        match self {
            Self::Utc(ms) => *ms,
            Self::Wall { wall_ms, .. } => *wall_ms,
            Self::Date(days) => days * MS_PER_DAY,
        }
    }
}

/// Excluded occurrences: instants, plus whole days for DATE values.
#[derive(Default)]
struct Exclusions {
    instants: HashSet<i64>,
    days: HashSet<i64>,
}

impl Exclusions {
    fn add(&mut self, value: &DateValue, zones: &Zones, event_zone: &TimeZone) {
        match value {
            DateValue::Date(days) => {
                self.days.insert(*days);
            }
            other => {
                self.instants.insert(zones.to_utc(other, event_zone));
            }
        }
    }

    fn contains(&self, utc_start: i64, wall_day: i64) -> bool {
        self.instants.contains(&utc_start) || self.days.contains(&wall_day)
    }
}

/// How long each occurrence lasts.
#[derive(Debug, Clone, Copy)]
enum Span {
    /// Exact duration (DTEND - DTSTART of timed events, DURATION).
    Exact(i64),
    /// Whole days of local wall time (all-day events).
    Days(i64),
}

impl Span {
    fn max_ms(self) -> i64 {
        match self {
            Self::Exact(ms) => ms.max(0),
            Self::Days(days) => days.max(0).saturating_mul(MS_PER_DAY),
        }
    }

    fn end_for(self, wall_start: i64, utc_start: i64, zone: &TimeZone) -> i64 {
        match self {
            Self::Exact(ms) => utc_start.saturating_add(ms.max(0)),
            Self::Days(days) => {
                zone.wall_to_utc(wall_start.saturating_add(days.max(0).saturating_mul(MS_PER_DAY)))
            }
        }
    }
}

#[derive(Debug, Default)]
struct RawEvent {
    uid: String,
    summary: String,
    start: Option<DateValue>,
    end: Option<DateValue>,
    duration_ms: Option<i64>,
    cancelled: bool,
    transparent: bool,
    /// `X-MICROSOFT-CDO-BUSYSTATUS`: `Some(false)` for FREE / WORKINGELSEWHERE.
    busy_status: Option<bool>,
    all_day: bool,
    rrule: Option<String>,
    rdates: Vec<DateValue>,
    exdates: Vec<DateValue>,
    recurrence_id: Option<DateValue>,
    online: bool,
}

impl RawEvent {
    fn busy(&self) -> bool {
        self.busy_status.unwrap_or(!self.transparent)
    }

    fn span(&self, zones: &Zones, zone: &TimeZone) -> Span {
        match (&self.start, &self.end, self.duration_ms) {
            (Some(DateValue::Date(start)), Some(DateValue::Date(end)), _) => {
                Span::Days(end - start)
            }
            (Some(start), Some(end), _) => {
                let start_utc = zones.to_utc(start, zone);
                let end_utc = zones.to_utc(end, zone);
                Span::Exact(end_utc - start_utc)
            }
            (Some(DateValue::Date(_)), None, Some(ms)) => Span::Days((ms / MS_PER_DAY).max(1)),
            (_, None, Some(ms)) => Span::Exact(ms),
            (Some(DateValue::Date(_)), None, None) => Span::Days(1),
            _ => Span::Exact(0),
        }
    }

    fn apply(&mut self, prop: &Prop<'_>) {
        let value = prop.value;
        match prop.name.as_str() {
            "UID" => self.uid = value.trim().to_owned(),
            "SUMMARY" => self.summary = clean_subject(&unescape_text(value)),
            "DTSTART" => self.start = parse_date_values(prop).into_iter().next(),
            "DTEND" => self.end = parse_date_values(prop).into_iter().next(),
            "DURATION" => self.duration_ms = parse_duration(value),
            "STATUS" => self.cancelled = value.trim().eq_ignore_ascii_case("CANCELLED"),
            "TRANSP" => self.transparent = value.trim().eq_ignore_ascii_case("TRANSPARENT"),
            "X-MICROSOFT-CDO-BUSYSTATUS" => {
                let status = value.trim().to_ascii_uppercase();
                self.busy_status = Some(!matches!(status.as_str(), "FREE" | "WORKINGELSEWHERE"));
            }
            "RRULE" => self.rrule = Some(value.trim().to_owned()),
            "RDATE" => self.rdates.extend(parse_date_values(prop)),
            "EXDATE" => self.exdates.extend(parse_date_values(prop)),
            "RECURRENCE-ID" => self.recurrence_id = parse_date_values(prop).into_iter().next(),
            name if name.ends_with("-ALLDAYEVENT") => {
                self.all_day |= value.trim().eq_ignore_ascii_case("TRUE");
            }
            "LOCATION" | "DESCRIPTION" | "URL" | "CONFERENCE" => {
                self.online |= looks_online(value);
            }
            name if name.starts_with("X-")
                && (name.contains("MEETING") || name.contains("CONFERENCE")) =>
            {
                self.online |= looks_online(value);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
struct RawObservance {
    offset_from: Option<i32>,
    offset_to: Option<i32>,
    start_wall_ms: Option<i64>,
    rrule: Option<String>,
    rdates_wall_ms: Vec<i64>,
}

impl RawObservance {
    fn apply(&mut self, prop: &Prop<'_>) {
        match prop.name.as_str() {
            "TZOFFSETFROM" => self.offset_from = parse_utc_offset(prop.value),
            "TZOFFSETTO" => self.offset_to = parse_utc_offset(prop.value),
            "DTSTART" => {
                self.start_wall_ms = parse_date_values(prop).first().map(DateValue::wall_ms)
            }
            "RRULE" => self.rrule = Some(prop.value.trim().to_owned()),
            "RDATE" => self
                .rdates_wall_ms
                .extend(parse_date_values(prop).iter().map(DateValue::wall_ms)),
            _ => {}
        }
    }

    fn finish(self) -> Option<Observance> {
        let offset_to = self.offset_to?;
        Some(Observance {
            offset_from: self.offset_from.unwrap_or(offset_to),
            offset_to,
            start_wall_ms: self.start_wall_ms.unwrap_or(0),
            rule: self.rrule.as_deref().and_then(Rule::parse),
            rdates_wall_ms: self.rdates_wall_ms,
        })
    }
}

#[derive(Debug, Default)]
struct RawZone {
    tzid: String,
    observances: Vec<Observance>,
}

#[derive(Debug, Default)]
struct Document {
    events: Vec<RawEvent>,
    zones: Vec<RawZone>,
}

impl Document {
    fn parse(data: &[u8]) -> Self {
        let mut document = Document::default();
        let mut stack: Vec<String> = Vec::new();
        let mut event: Option<RawEvent> = None;
        let mut zone: Option<RawZone> = None;
        let mut observance: Option<RawObservance> = None;

        for line in unfold(data) {
            let Some(prop) = parse_line(&line) else {
                continue;
            };
            match prop.name.as_str() {
                "BEGIN" => {
                    if stack.len() >= MAX_NESTING {
                        // Real calendars nest 3 deep; this also keeps the END search below short.
                        continue;
                    }
                    let component = prop.value.trim().to_ascii_uppercase();
                    match component.as_str() {
                        "VEVENT" if event.is_none() => event = Some(RawEvent::default()),
                        "VTIMEZONE" if zone.is_none() => zone = Some(RawZone::default()),
                        "STANDARD" | "DAYLIGHT"
                            if zone.is_some() && stack.last().is_some_and(|c| c == "VTIMEZONE") =>
                        {
                            observance = Some(RawObservance::default());
                        }
                        _ => {}
                    }
                    stack.push(component);
                }
                "END" => {
                    let component = prop.value.trim().to_ascii_uppercase();
                    // Tolerate missing END lines: pop up to the matching BEGIN.
                    if let Some(at) = stack.iter().rposition(|c| *c == component) {
                        stack.truncate(at);
                    }
                    match component.as_str() {
                        "VEVENT" => {
                            if let Some(mut finished) = event.take() {
                                if finished.uid.is_empty() {
                                    finished.uid = synthetic_uid(&finished);
                                }
                                document.events.push(finished);
                            }
                        }
                        "STANDARD" | "DAYLIGHT" => {
                            if let (Some(raw), Some(zone)) = (observance.take(), zone.as_mut())
                                && let Some(finished) = raw.finish()
                            {
                                zone.observances.push(finished);
                            }
                        }
                        "VTIMEZONE" => {
                            if let Some(finished) = zone.take()
                                && !finished.tzid.is_empty()
                            {
                                document.zones.push(finished);
                            }
                            observance = None;
                        }
                        _ => {}
                    }
                }
                _ => match stack.last().map(String::as_str) {
                    Some("VEVENT") => {
                        if let Some(event) = event.as_mut() {
                            event.apply(&prop);
                        }
                    }
                    Some("STANDARD" | "DAYLIGHT") => {
                        if let Some(observance) = observance.as_mut() {
                            observance.apply(&prop);
                        }
                    }
                    Some("VTIMEZONE") => {
                        if prop.name == "TZID"
                            && let Some(zone) = zone.as_mut()
                        {
                            zone.tzid = prop.value.trim().to_owned();
                        }
                    }
                    _ => {}
                },
            }
        }
        document
    }

    /// Resolves every TZID used by the events (VTIMEZONE of this file first, then [`TimeZone::by_name`], then
    /// the local zone).
    fn zones(&self, local: &TimeZone) -> Zones {
        let mut defined: HashMap<String, TimeZone> = HashMap::new();
        for raw in &self.zones {
            if zone_cost(&raw.observances) > MAX_ZONE_COST {
                log::debug!("calendar: VTIMEZONE too complex, resolving its TZID by name");
                continue;
            }
            if let Some(zone) = TimeZone::from_observances(raw.observances.clone()) {
                defined.insert(raw.tzid.clone(), zone);
            }
        }
        let mut by_tzid = HashMap::new();
        let used = self.events.iter().flat_map(|e| {
            e.start
                .iter()
                .chain(e.end.iter())
                .chain(e.exdates.iter())
                .chain(e.rdates.iter())
                .chain(e.recurrence_id.iter())
        });
        for value in used {
            if let DateValue::Wall {
                tzid: Some(tzid), ..
            } = value
                && !by_tzid.contains_key(tzid)
            {
                let zone = defined
                    .get(tzid)
                    .cloned()
                    .or_else(|| TimeZone::by_name(tzid))
                    .unwrap_or_else(|| {
                        log::debug!(
                            "calendar: unknown time zone in a calendar file, using local time"
                        );
                        local.clone()
                    });
                by_tzid.insert(tzid.clone(), zone);
            }
        }
        Zones {
            by_tzid,
            local: local.clone(),
            utc: TimeZone::utc(),
        }
    }
}

/// Rough work of one offset lookup in a VTIMEZONE: every observance and RDATE onset is visited, and a yearly
/// rule computes the onsets of three years.
fn zone_cost(observances: &[Observance]) -> usize {
    observances.iter().fold(0usize, |cost, observance| {
        let rule = if observance.rule.is_some() { 16 } else { 0 };
        cost.saturating_add(1)
            .saturating_add(observance.rdates_wall_ms.len())
            .saturating_add(rule)
    })
}

struct Zones {
    by_tzid: HashMap<String, TimeZone>,
    local: TimeZone,
    utc: TimeZone,
}

impl Zones {
    /// The zone a value's wall time is in (UTC values: UTC; floating times and dates: local).
    fn zone_of(&self, value: &DateValue) -> &TimeZone {
        match value {
            DateValue::Utc(_) => &self.utc,
            DateValue::Wall {
                tzid: Some(tzid), ..
            } => self.by_tzid.get(tzid).unwrap_or(&self.local),
            DateValue::Wall { tzid: None, .. } | DateValue::Date(_) => &self.local,
        }
    }

    /// Like [`Zones::zone_of`], but floating values use `fallback` (an EXDATE without TZID of a zoned event is
    /// in the event's zone).
    fn zone_of_or<'a>(&'a self, value: &DateValue, fallback: &'a TimeZone) -> &'a TimeZone {
        match value {
            DateValue::Wall { tzid: None, .. } => fallback,
            other => self.zone_of(other),
        }
    }

    fn to_utc(&self, value: &DateValue, fallback: &TimeZone) -> i64 {
        match value {
            DateValue::Utc(ms) => *ms,
            other => self
                .zone_of_or(other, fallback)
                .wall_to_utc(other.wall_ms()),
        }
    }
}

/// Stable id for an event without UID.
fn synthetic_uid(event: &RawEvent) -> String {
    let start = event.start.as_ref().map_or(0, DateValue::wall_ms);
    format!(
        "no-uid-{:016x}",
        fnv1a(format!("{}|{start}", event.summary).as_bytes())
    )
}

// -- content lines --------------------------------------------------------------------------------------------

/// Joins folded lines (CRLF or LF followed by a space or tab) at the byte level, so a fold inside a multi-byte
/// UTF-8 character is repaired before decoding.
fn unfold(data: &[u8]) -> Vec<String> {
    let data = data.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(data);
    let mut lines = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let newline_len = match byte {
            b'\r' if data.get(i + 1) == Some(&b'\n') => 2,
            b'\n' | b'\r' => 1,
            _ => 0,
        };
        if newline_len == 0 {
            current.push(byte);
            i += 1;
            continue;
        }
        i += newline_len;
        if matches!(data.get(i), Some(b' ' | b'\t')) {
            // Continuation: drop the line break and one whitespace character.
            i += 1;
            continue;
        }
        if !current.is_empty() {
            lines.push(String::from_utf8_lossy(&current).into_owned());
            current.clear();
        }
    }
    if !current.is_empty() {
        lines.push(String::from_utf8_lossy(&current).into_owned());
    }
    lines
}

/// `NAME;PARAM=value;PARAM="quoted:value":VALUE`.
#[derive(Debug)]
struct Prop<'a> {
    name: String,
    params: Vec<(String, String)>,
    value: &'a str,
}

impl Prop<'_> {
    fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

fn parse_line(line: &str) -> Option<Prop<'_>> {
    let bytes = line.as_bytes();
    let name_end = line.find([';', ':'])?;
    let name = line[..name_end].trim().to_ascii_uppercase();
    if name.is_empty() {
        return None;
    }
    let mut params = Vec::new();
    let mut i = name_end;
    while bytes.get(i) == Some(&b';') {
        i += 1;
        let eq = i + line[i..].find(['=', ':', ';'])?;
        if bytes[eq] != b'=' {
            // A parameter without a value: skip it.
            i = eq;
            continue;
        }
        let param_name = line[i..eq].trim().to_ascii_uppercase();
        i = eq + 1;
        let start = i;
        let mut quoted = false;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => quoted = !quoted,
                b';' | b':' if !quoted => break,
                _ => {}
            }
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        let raw = line[start..i].trim();
        let value = raw
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(raw);
        params.push((param_name, value.to_owned()));
    }
    if bytes.get(i) != Some(&b':') {
        return None;
    }
    Some(Prop {
        name,
        params,
        value: &line[i + 1..],
    })
}

/// DATE / DATE-TIME values of a property (comma-separated; `VALUE=PERIOD` keeps the start).
fn parse_date_values(prop: &Prop<'_>) -> Vec<DateValue> {
    let is_date = prop
        .param("VALUE")
        .is_some_and(|v| v.eq_ignore_ascii_case("DATE"));
    let tzid = prop
        .param("TZID")
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty());
    prop.value
        .split(',')
        .filter_map(|raw| {
            let raw = raw.trim();
            let raw = raw.split('/').next().unwrap_or(raw);
            if is_date || raw.len() == 8 {
                return parse_ical_date(raw).map(DateValue::Date);
            }
            let (wall_ms, utc) = parse_ical_date_time(raw)?;
            Some(if utc {
                DateValue::Utc(wall_ms)
            } else {
                DateValue::Wall {
                    wall_ms,
                    tzid: tzid.clone(),
                }
            })
        })
        .collect()
}

/// `+0100`, `-0500`, `+053000` → minutes east of UTC.
fn parse_utc_offset(text: &str) -> Option<i32> {
    let text = text.trim();
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits = &text[1..];
    if !(digits.len() == 4 || digits.len() == 6) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hours: i32 = digits[0..2].parse().ok()?;
    let minutes: i32 = digits[2..4].parse().ok()?;
    (hours <= 18 && minutes < 60).then_some(sign * (hours * 60 + minutes))
}

/// TEXT value unescaping (`\,` `\;` `\\` `\n`).
fn unescape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::time::{MS_PER_HOUR, MS_PER_MINUTE, days_from_civil};

    fn utc(y: i64, m: u32, d: u32, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * MS_PER_DAY + h * MS_PER_HOUR + min * MS_PER_MINUTE
    }

    fn crlf(text: &str) -> Vec<u8> {
        text.lines()
            .map(|l| l.trim_start_matches('|'))
            .collect::<Vec<_>>()
            .join("\r\n")
            .into_bytes()
    }

    /// (subject, start, end) in UTC.
    fn summary(events: &[CalEvent]) -> Vec<(String, i64, i64)> {
        events
            .iter()
            .map(|e| (e.subject.clone(), e.start_ms, e.end_ms))
            .collect()
    }

    // Outlook / Exchange "publish calendar" export: Windows zone names with a VTIMEZONE, a daily stand-up on
    // workdays with an EXDATE, a moved occurrence, a cancelled occurrence, a free event, an all-day event.
    const OUTLOOK: &str = "BEGIN:VCALENDAR
|METHOD:PUBLISH
|PRODID:Microsoft Exchange Server 2010
|VERSION:2.0
|X-WR-CALNAME:Kalendarz
|BEGIN:VTIMEZONE
|TZID:Central European Standard Time
|BEGIN:STANDARD
|DTSTART:16010101T030000
|TZOFFSETFROM:+0200
|TZOFFSETTO:+0100
|RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=10
|END:STANDARD
|BEGIN:DAYLIGHT
|DTSTART:16010101T020000
|TZOFFSETFROM:+0100
|TZOFFSETTO:+0200
|RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=3
|END:DAYLIGHT
|END:VTIMEZONE
|BEGIN:VEVENT
|DESCRIPTION:\\n__________________________________________________________________
| ______________\\nSpotkanie w aplikacji Microsoft Teams\\nDołącz: https://teams.mi
| crosoft.com/l/meetup-join/19%3ameeting_abc%40thread.v2/0?context=%7b%7d\\n
|RRULE:FREQ=WEEKLY;UNTIL=20261231T083000Z;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR;WKST=MO
|EXDATE;TZID=Central European Standard Time:20261014T093000
|UID:040000008200E00074C5B7101A82E00800000000A0B1C2D3E4F5A6B7000000000000000010000000
|SUMMARY;LANGUAGE=pl-PL:Daily standup
|DTSTART;TZID=Central European Standard Time:20260105T093000
|DTEND;TZID=Central European Standard Time:20260105T094500
|CLASS:PUBLIC
|PRIORITY:5
|DTSTAMP:20261012T120000Z
|TRANSP:OPAQUE
|STATUS:CONFIRMED
|SEQUENCE:0
|LOCATION;LANGUAGE=pl-PL:Spotkanie w aplikacji Microsoft Teams
|X-MICROSOFT-CDO-APPT-SEQUENCE:0
|X-MICROSOFT-CDO-BUSYSTATUS:BUSY
|X-MICROSOFT-CDO-INTENDEDSTATUS:BUSY
|X-MICROSOFT-CDO-ALLDAYEVENT:FALSE
|X-MICROSOFT-CDO-IMPORTANCE:1
|X-MICROSOFT-CDO-INSTTYPE:1
|X-MICROSOFT-DONOTFORWARDMEETING:FALSE
|X-MICROSOFT-DISALLOW-COUNTER:FALSE
|BEGIN:VALARM
|DESCRIPTION:REMINDER
|TRIGGER;RELATED=START:-PT15M
|ACTION:DISPLAY
|END:VALARM
|END:VEVENT
|BEGIN:VEVENT
|UID:040000008200E00074C5B7101A82E00800000000A0B1C2D3E4F5A6B7000000000000000010000000
|RECURRENCE-ID;TZID=Central European Standard Time:20261015T093000
|SUMMARY;LANGUAGE=pl-PL:Daily standup (przesunięty)
|DTSTART;TZID=Central European Standard Time:20261015T113000
|DTEND;TZID=Central European Standard Time:20261015T120000
|X-MICROSOFT-CDO-BUSYSTATUS:BUSY
|END:VEVENT
|BEGIN:VEVENT
|UID:040000008200E00074C5B7101A82E00800000000A0B1C2D3E4F5A6B7000000000000000010000000
|RECURRENCE-ID;TZID=Central European Standard Time:20261016T093000
|SUMMARY;LANGUAGE=pl-PL:Anulowano: Daily standup
|STATUS:CANCELLED
|DTSTART;TZID=Central European Standard Time:20261016T093000
|DTEND;TZID=Central European Standard Time:20261016T094500
|END:VEVENT
|BEGIN:VEVENT
|UID:free-lunch
|SUMMARY:Lunch
|DTSTART;TZID=Central European Standard Time:20261013T120000
|DTEND;TZID=Central European Standard Time:20261013T130000
|TRANSP:TRANSPARENT
|X-MICROSOFT-CDO-BUSYSTATUS:FREE
|END:VEVENT
|BEGIN:VEVENT
|UID:holiday
|SUMMARY:Urlop
|DTSTART;VALUE=DATE:20261013
|DTEND;VALUE=DATE:20261014
|X-MICROSOFT-CDO-ALLDAYEVENT:TRUE
|X-MICROSOFT-CDO-BUSYSTATUS:OOF
|END:VEVENT
|BEGIN:VEVENT
|UID:review
|SUMMARY:Przegląd sprintu\\, zespół A
|DTSTART;TZID=\"Central European Standard Time\":20261026T140000
|DURATION:PT1H30M
|END:VEVENT
|END:VCALENDAR
";

    #[test]
    fn outlook_publish_feed() {
        let local = TimeZone::utc();
        let start = utc(2026, 10, 12, 0, 0);
        let end = utc(2026, 10, 27, 0, 0);
        let events = parse_events(&crlf(OUTLOOK), start, end, &local);
        let standups: Vec<(String, i64, i64)> = summary(&events)
            .into_iter()
            .filter(|(s, _, _)| s.starts_with("Daily standup"))
            .collect();
        let cest = |d: u32, h: i64, m: i64| utc(2026, 10, d, h - 2, m);
        let cet = |d: u32, h: i64, m: i64| utc(2026, 10, d, h - 1, m);
        assert_eq!(
            standups,
            vec![
                ("Daily standup".into(), cest(12, 9, 30), cest(12, 9, 45)),
                ("Daily standup".into(), cest(13, 9, 30), cest(13, 9, 45)),
                // 14th: EXDATE. 15th: moved to 11:30. 16th: cancelled.
                (
                    "Daily standup (przesunięty)".into(),
                    cest(15, 11, 30),
                    cest(15, 12, 0)
                ),
                ("Daily standup".into(), cest(19, 9, 30), cest(19, 9, 45)),
                ("Daily standup".into(), cest(20, 9, 30), cest(20, 9, 45)),
                ("Daily standup".into(), cest(21, 9, 30), cest(21, 9, 45)),
                ("Daily standup".into(), cest(22, 9, 30), cest(22, 9, 45)),
                ("Daily standup".into(), cest(23, 9, 30), cest(23, 9, 45)),
                // Winter time from 25 October: still 09:30 local, one hour later in UTC.
                ("Daily standup".into(), cet(26, 9, 30), cet(26, 9, 45)),
            ]
        );
        let first = events
            .iter()
            .find(|e| e.subject == "Daily standup")
            .expect("standup");
        assert!(first.online, "Teams link in the folded description");
        assert!(first.busy && !first.all_day);
        assert!(first.id.starts_with("040000008200E000"));

        let lunch = events.iter().find(|e| e.id == "free-lunch").expect("lunch");
        assert!(!lunch.busy);
        let holiday = events.iter().find(|e| e.id == "holiday").expect("holiday");
        assert!(holiday.all_day);
        let review = events.iter().find(|e| e.id == "review").expect("review");
        assert_eq!(review.subject, "Przegląd sprintu, zespół A");
        assert_eq!(
            (review.start_ms, review.end_ms),
            (cet(26, 14, 0), cet(26, 15, 30))
        );

        let meetings = meetings_only(events, start, end);
        assert!(
            meetings
                .iter()
                .all(|e| e.id != "free-lunch" && e.id != "holiday")
        );
        assert_eq!(meetings.len(), standups.len() + 1);
    }

    // Google Calendar "secret address in iCal format": IANA zone with a VTIMEZONE, UTC times, a weekly event with
    // UNTIL, a transparent event, a cancelled event, a description folded in the middle of a link and of a
    // multi-byte character.
    const GOOGLE: &str = "BEGIN:VCALENDAR
|PRODID:-//Google Inc//Google Calendar 70.9054//EN
|VERSION:2.0
|CALSCALE:GREGORIAN
|METHOD:PUBLISH
|X-WR-CALNAME:jan.kowalski@example.com
|X-WR-TIMEZONE:Europe/Warsaw
|BEGIN:VTIMEZONE
|TZID:Europe/Warsaw
|X-LIC-LOCATION:Europe/Warsaw
|BEGIN:DAYLIGHT
|TZOFFSETFROM:+0100
|TZOFFSETTO:+0200
|TZNAME:CEST
|DTSTART:19700329T020000
|RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU
|END:DAYLIGHT
|BEGIN:STANDARD
|TZOFFSETFROM:+0200
|TZOFFSETTO:+0100
|TZNAME:CET
|DTSTART:19701025T030000
|RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU
|END:STANDARD
|END:VTIMEZONE
|BEGIN:VEVENT
|DTSTART;TZID=Europe/Warsaw:20260907T150000
|DTEND;TZID=Europe/Warsaw:20260907T160000
|RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20261102T225959Z;BYDAY=MO
|DTSTAMP:20261012T101010Z
|UID:7kukuqrfedlm2f9t0vr42q2kpd@google.com
|CREATED:20260901T090000Z
|DESCRIPTION:Dołącz przez Google Meet: https://meet.goog
| le.com/abc-defg-hij\\nSpotkanie zespołu – omówienie postępów i za
|  pytań.
|LAST-MODIFIED:20260901T090000Z
|LOCATION:
|SEQUENCE:0
|STATUS:CONFIRMED
|SUMMARY:Planowanie tygodnia
|TRANSP:OPAQUE
|END:VEVENT
|BEGIN:VEVENT
|DTSTART:20261013T070000Z
|DTEND:20261013T073000Z
|DTSTAMP:20261012T101010Z
|UID:abc123@google.com
|SUMMARY:1:1 z Anią
|STATUS:CONFIRMED
|TRANSP:OPAQUE
|END:VEVENT
|BEGIN:VEVENT
|DTSTART:20261013T090000Z
|DTEND:20261013T100000Z
|UID:transparent@google.com
|SUMMARY:Czas na skupienie
|TRANSP:TRANSPARENT
|END:VEVENT
|BEGIN:VEVENT
|DTSTART:20261013T110000Z
|DTEND:20261013T120000Z
|UID:cancelled@google.com
|SUMMARY:Odwołane
|STATUS:CANCELLED
|END:VEVENT
|END:VCALENDAR
";

    #[test]
    fn google_secret_address_feed() {
        // Bytes with the fold inside "ę" (0xC4 0x99) to check byte-level unfolding.
        let mut data = crlf(GOOGLE);
        let marker = "zespołu".as_bytes().to_vec();
        if let Some(at) = data
            .windows(marker.len())
            .position(|w| w == marker.as_slice())
        {
            let split = at + "zespo".len() + 1; // inside "ł"
            data.splice(split..split, b"\r\n ".iter().copied());
        }
        let local = TimeZone::fixed(-300);
        let start = utc(2026, 10, 1, 0, 0);
        let end = utc(2026, 11, 30, 0, 0);
        let events = parse_events(&data, start, end, &local);
        let weekly: Vec<i64> = events
            .iter()
            .filter(|e| e.subject == "Planowanie tygodnia")
            .map(|e| e.start_ms)
            .collect();
        assert_eq!(
            weekly,
            vec![
                utc(2026, 10, 5, 13, 0),
                utc(2026, 10, 12, 13, 0),
                utc(2026, 10, 19, 13, 0),
                utc(2026, 10, 26, 14, 0),
                utc(2026, 11, 2, 14, 0),
            ],
            "15:00 Warsaw time, UNTIL 2 November 22:59:59 UTC"
        );
        let planning = events
            .iter()
            .find(|e| e.subject == "Planowanie tygodnia")
            .expect("planning");
        assert!(planning.online, "Meet link folded across lines");
        assert_eq!(planning.end_ms - planning.start_ms, MS_PER_HOUR);

        let one_on_one = events
            .iter()
            .find(|e| e.id == "abc123@google.com")
            .expect("1:1");
        assert_eq!(one_on_one.subject, "1:1 z Anią");
        assert_eq!(
            (one_on_one.start_ms, one_on_one.end_ms),
            (utc(2026, 10, 13, 7, 0), utc(2026, 10, 13, 7, 30))
        );
        assert!(!events.iter().any(|e| e.id == "cancelled@google.com"));
        let transparent = events
            .iter()
            .find(|e| e.id == "transparent@google.com")
            .expect("transparent");
        assert!(!transparent.busy);
    }

    #[test]
    fn unfolding_repairs_split_characters() {
        let lines = unfold("SUMMARY:Zespo\u{142}u".as_bytes());
        assert_eq!(lines, vec!["SUMMARY:Zespołu"]);
        let mut bytes = b"SUMMARY:Zespo".to_vec();
        bytes.extend_from_slice(&[0xC5]);
        bytes.extend_from_slice(b"\r\n\t");
        bytes.extend_from_slice(&[0x82]);
        bytes.extend_from_slice(b"u\nDTSTART:20261013\r\n");
        assert_eq!(
            unfold(&bytes),
            vec!["SUMMARY:Zespołu".to_owned(), "DTSTART:20261013".to_owned()]
        );
        assert_eq!(
            unfold(b"\xEF\xBB\xBFBEGIN:VCALENDAR\n\nEND:VCALENDAR"),
            vec!["BEGIN:VCALENDAR", "END:VCALENDAR"]
        );
    }

    #[test]
    fn content_lines_parse() {
        let prop =
            parse_line("DTSTART;TZID=\"W. Europe Standard Time\";VALUE=DATE-TIME:20261013T093000")
                .expect("line");
        assert_eq!(prop.name, "DTSTART");
        assert_eq!(prop.param("TZID"), Some("W. Europe Standard Time"));
        assert_eq!(prop.param("VALUE"), Some("DATE-TIME"));
        assert_eq!(prop.value, "20261013T093000");
        let prop =
            parse_line("attendee;CN=\"Kowalski: Jan\";ROLE=REQ-PARTICIPANT:mailto:jan@example.com")
                .expect("line");
        assert_eq!(prop.name, "ATTENDEE");
        assert_eq!(prop.param("CN"), Some("Kowalski: Jan"));
        assert_eq!(prop.value, "mailto:jan@example.com");
        assert!(parse_line("no colon here").is_none());
        assert!(parse_line(":value").is_none());
        assert!(parse_line("X;Y=\"unterminated:value").is_none());
        assert_eq!(unescape_text("a\\, b\\; c\\\\d\\ne"), "a, b; c\\d\ne");
        assert_eq!(parse_utc_offset("+0100"), Some(60));
        assert_eq!(parse_utc_offset("-0500"), Some(-300));
        assert_eq!(parse_utc_offset("+053000"), Some(330));
        assert_eq!(parse_utc_offset("0100"), None);
        assert_eq!(parse_utc_offset("+01"), None);
    }

    #[test]
    fn windows_tzid_without_vtimezone_floating_and_monthly() {
        let text = "BEGIN:VCALENDAR
|BEGIN:VEVENT
|UID:pacific
|SUMMARY:West coast sync
|DTSTART;TZID=Pacific Standard Time:20261013T090000
|DTEND;TZID=Pacific Standard Time:20261013T093000
|END:VEVENT
|BEGIN:VEVENT
|UID:floating
|SUMMARY:Floating
|DTSTART:20261013T100000
|DURATION:PT45M
|END:VEVENT
|BEGIN:VEVENT
|UID:monthly
|SUMMARY:Retro
|DTSTART;TZID=Europe/Warsaw:20260130T150000
|DTEND;TZID=Europe/Warsaw:20260130T160000
|RRULE:FREQ=MONTHLY;BYDAY=-1FR
|EXDATE;TZID=Europe/Warsaw:20261030T150000,20261127T150000
|END:VEVENT
|BEGIN:VEVENT
|UID:unknown-zone
|SUMMARY:Somewhere
|DTSTART;TZID=Mars/Olympus_Mons:20261013T120000
|DTEND;TZID=Mars/Olympus_Mons:20261013T130000
|END:VEVENT
|END:VCALENDAR";
        let local = TimeZone::fixed(120);
        let start = utc(2026, 9, 1, 0, 0);
        let end = utc(2027, 1, 1, 0, 0);
        let events = parse_events(&crlf(text), start, end, &local);
        let find = |id: &str| -> Vec<(i64, i64)> {
            events
                .iter()
                .filter(|e| e.id == id)
                .map(|e| (e.start_ms, e.end_ms))
                .collect()
        };
        // 09:00 PDT = 16:00 UTC.
        assert_eq!(
            find("pacific"),
            vec![(utc(2026, 10, 13, 16, 0), utc(2026, 10, 13, 16, 30))]
        );
        // Floating 10:00 in a UTC+2 local zone.
        assert_eq!(
            find("floating"),
            vec![(utc(2026, 10, 13, 8, 0), utc(2026, 10, 13, 8, 45))]
        );
        // Last Fridays: 25 Sep (CEST), [30 Oct and 27 Nov excluded], 25 Dec (CET).
        assert_eq!(
            find("monthly"),
            vec![
                (utc(2026, 9, 25, 13, 0), utc(2026, 9, 25, 14, 0)),
                (utc(2026, 12, 25, 14, 0), utc(2026, 12, 25, 15, 0)),
            ]
        );
        // Unknown zone: local time.
        assert_eq!(
            find("unknown-zone"),
            vec![(utc(2026, 10, 13, 10, 0), utc(2026, 10, 13, 11, 0))]
        );
    }

    #[test]
    fn range_edges_and_limits() {
        let text = "BEGIN:VCALENDAR
|BEGIN:VEVENT
|UID:daily
|SUMMARY:Every day
|DTSTART:20000101T080000Z
|DTEND:20000101T081500Z
|RRULE:FREQ=DAILY
|END:VEVENT
|BEGIN:VEVENT
|UID:long
|SUMMARY:Workshop
|DTSTART:20261012T080000Z
|DTEND:20261014T160000Z
|END:VEVENT
|BEGIN:VEVENT
|UID:unsupported
|SUMMARY:Odd rule
|DTSTART:20261013T120000Z
|DTEND:20261013T130000Z
|RRULE:FREQ=HOURLY;INTERVAL=2
|END:VEVENT
|BEGIN:VEVENT
|SUMMARY:No uid
|DTSTART:20261013T140000Z
|DTEND:20261013T150000Z
|END:VEVENT
|END:VCALENDAR";
        let data = crlf(text);
        let local = TimeZone::utc();
        // A 10-hour horizon: the daily event of a rule started in 2000, the workshop that started before.
        let start = utc(2026, 10, 13, 6, 0);
        let end = utc(2026, 10, 13, 16, 0);
        let events = parse_events(&data, start, end, &local);
        let ids: Vec<&str> = events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids.len(), 4, "{ids:?}");
        assert_eq!(ids[0], "long");
        assert_eq!(ids[1], "daily");
        assert_eq!(
            ids[2], "unsupported",
            "unsupported rule: first occurrence only"
        );
        assert!(ids[3].starts_with("no-uid-"));
        assert_eq!(
            parse_events(&data, start, end, &local)[3].id,
            ids[3],
            "synthetic ids are stable"
        );
        // Touching the range edge does not count.
        assert!(
            parse_events(
                &data,
                utc(2026, 10, 13, 8, 15),
                utc(2026, 10, 13, 8, 16),
                &local
            )
            .iter()
            .all(|e| e.id != "daily")
        );
        // Ten years of a daily event: capped.
        let all = parse_events(&data, utc(2016, 1, 1, 0, 0), utc(2026, 1, 1, 0, 0), &local);
        assert_eq!(all.len(), MAX_EVENTS);
        assert!(parse_events(&data, end, start, &local).is_empty());
        assert!(parse_events(b"garbage", start, end, &local).is_empty());
    }

    #[test]
    fn icalendar_detection() {
        assert!(looks_like_icalendar(b"BEGIN:VCALENDAR\r\nEND:VCALENDAR"));
        assert!(looks_like_icalendar(
            b"\xEF\xBB\xBF \r\nbegin:vcalendar\r\n"
        ));
        assert!(!looks_like_icalendar(b"<!doctype html><html>"));
        assert!(!looks_like_icalendar(b""));
    }

    #[test]
    fn links_are_normalized() {
        assert_eq!(
            normalize_url(" webcal://p01-calendars.icloud.com/published/2/abc ").expect("webcal"),
            "https://p01-calendars.icloud.com/published/2/abc"
        );
        assert_eq!(
            normalize_url("WEBCALS://example.com/cal.ics").expect("webcals"),
            "https://example.com/cal.ics"
        );
        assert_eq!(
            normalize_url("https://outlook.office365.com/owa/calendar/x/y/calendar.ics")
                .expect("https"),
            "https://outlook.office365.com/owa/calendar/x/y/calendar.ics"
        );
        for bad in [
            "",
            "http://example.com/cal.ics",
            "ftp://example.com/cal.ics",
            "calendar.ics",
            "https://",
        ] {
            let err = normalize_url(bad).expect_err(bad);
            assert!(matches!(err, PlatformError::Invalid(_)), "{bad}");
        }
        let secret = "http://example.com/private-secret-token/basic.ics";
        let err = normalize_url(secret).expect_err("http");
        assert!(!err.to_string().contains("private-secret-token"));
        assert!(
            normalize_url(&format!("https://example.com/{}", "a".repeat(MAX_URL_LEN))).is_err()
        );
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "focusmode-test-{}-{nanos}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn link_storage() {
        let path = unique_temp_path("ics-links.bin");
        assert_eq!(load_urls(&path).expect("missing"), Vec::<String>::new());
        let err = save_urls(
            &path,
            &["https://ok.example/a.ics".into(), "http://bad".into()],
        )
        .expect_err("invalid link");
        assert!(matches!(err, PlatformError::Invalid(ref m) if m.starts_with("link 2:")));
        let too_many: Vec<String> = (0..=MAX_URLS)
            .map(|i| format!("https://x.example/{i}.ics"))
            .collect();
        assert!(matches!(
            save_urls(&path, &too_many),
            Err(PlatformError::Invalid(_))
        ));
        assert!(!path.exists());

        let urls = vec![
            "webcal://cal.example/a.ics".to_owned(),
            "  ".to_owned(),
            "https://cal.example/a.ics".to_owned(),
            "https://cal.example/b.ics".to_owned(),
        ];
        match save_urls(&path, &urls) {
            Ok(stored) => {
                assert_eq!(
                    stored,
                    vec!["https://cal.example/a.ics", "https://cal.example/b.ics"]
                );
                assert_eq!(load_urls(&path).expect("load"), stored);
                let raw = std::fs::read(&path).expect("raw");
                assert!(
                    !String::from_utf8_lossy(&raw).contains("cal.example"),
                    "encrypted"
                );
                assert_eq!(save_urls(&path, &[]).expect("clear"), Vec::<String>::new());
                assert!(!path.exists());
            }
            Err(err) => assert!(err.is_unsupported(), "{err}"),
        }
        delete_urls(&path).expect("delete");
    }

    #[test]
    fn fetch_rejects_bad_links_without_network() {
        let client = IcsClient::new();
        assert!(matches!(
            client.fetch("http://example.com/cal.ics"),
            Err(PlatformError::Invalid(_))
        ));
    }

    /// Hostile or broken files stay cheap: each case below took seconds to minutes per 5 MB before the limits
    /// (shared recurrence budget, range-filtered RDATE / EXDATE, nesting cap, VTIMEZONE complexity cap).
    #[test]
    fn hostile_files_stay_cheap() {
        let local = TimeZone::utc();
        let (start, end) = (utc(2026, 10, 13, 6, 0), utc(2026, 10, 13, 16, 0));
        let meeting = "BEGIN:VEVENT\r\nUID:real\r\nSUMMARY:Real\r\nDTSTART:20261013T100000Z\r\n\
                       DTEND:20261013T110000Z\r\nEND:VEVENT\r\n";
        let real = vec![(
            "Real".to_owned(),
            utc(2026, 10, 13, 10, 0),
            utc(2026, 10, 13, 11, 0),
        )];

        // COUNT rules from 1970 cannot be fast-forwarded: together they would walk millions of days.
        let mut text = String::from("BEGIN:VCALENDAR\r\n");
        for i in 0..400 {
            text.push_str(&format!(
                "BEGIN:VEVENT\r\nUID:c{i}\r\nSUMMARY:Bomb\r\nDTSTART:19700101T000000Z\r\n\
                 DTEND:19700101T000100Z\r\nRRULE:FREQ=DAILY;COUNT=99999999\r\nEND:VEVENT\r\n"
            ));
        }
        text.push_str(meeting);
        text.push_str("END:VCALENDAR\r\n");
        let events = parse_events(text.as_bytes(), start, end, &local);
        assert_eq!(
            summary(&events),
            real,
            "budget used up, DTSTART still counts"
        );

        // A huge RDATE / EXDATE list: only the dates near the range are looked at, duplicates collapse.
        let mut text = String::from(
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:r\r\nSUMMARY:Extra\r\nDTSTART:20200101T120000Z\r\n\
             DTEND:20200101T130000Z\r\n",
        );
        for minute in 0..30_000i64 {
            let at = utc(2020, 1, 2, 0, 0) + minute * MS_PER_MINUTE;
            let stamp = super::super::format_utc(at).replace(['-', ':'], "");
            text.push_str(&format!("RDATE:{stamp}\r\nEXDATE:{stamp}\r\n"));
        }
        text.push_str("RDATE:20261013T120000Z,20261013T120000Z\r\nRDATE:20261013T140000Z\r\n");
        text.push_str("EXDATE:20261013T140000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n");
        let events = parse_events(text.as_bytes(), start, end, &local);
        assert_eq!(
            summary(&events),
            vec![(
                "Extra".to_owned(),
                utc(2026, 10, 13, 12, 0),
                utc(2026, 10, 13, 13, 0)
            )]
        );

        // Unbalanced nesting: BEGINs past the cap are ignored and the END search stays short.
        let mut text = format!("BEGIN:VCALENDAR\r\n{meeting}");
        text.push_str(&"BEGIN:X\r\n".repeat(20_000));
        text.push_str(&"END:Y\r\n".repeat(20_000));
        let events = parse_events(text.as_bytes(), start, end, &local);
        assert_eq!(summary(&events), real);

        // A VTIMEZONE with thousands of onsets is not used; its TZID resolves by name (Warsaw: CEST, UTC+2).
        let mut text = String::from(
            "BEGIN:VCALENDAR\r\nBEGIN:VTIMEZONE\r\nTZID:Europe/Warsaw\r\nBEGIN:STANDARD\r\n\
             DTSTART:19700101T000000\r\nTZOFFSETFROM:+0500\r\nTZOFFSETTO:+0500\r\n",
        );
        for hour in 0..(MAX_ZONE_COST as i64) {
            let stamp = super::super::format_utc(utc(1980, 1, 1, 0, 0) + hour * MS_PER_HOUR);
            text.push_str(&format!("RDATE:{}\r\n", stamp.replace(['-', ':', 'Z'], "")));
        }
        text.push_str(
            "END:STANDARD\r\nEND:VTIMEZONE\r\nBEGIN:VEVENT\r\nUID:z\r\nSUMMARY:Zoned\r\n\
             DTSTART;TZID=Europe/Warsaw:20261013T120000\r\nDTEND;TZID=Europe/Warsaw:20261013T130000\r\n\
             END:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let events = parse_events(text.as_bytes(), start, end, &local);
        assert_eq!(
            summary(&events),
            vec![(
                "Zoned".to_owned(),
                utc(2026, 10, 13, 10, 0),
                utc(2026, 10, 13, 11, 0)
            )]
        );
    }
}
