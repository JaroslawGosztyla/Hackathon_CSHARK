//! Config defaults and validation. The shell persists only configs that passed
//! [`Config::normalized`]; the UI shows the returned `AppError.field` next to the input.

use std::ops::RangeInclusive;

use crate::api::{
    AppError, CalendarConfig, ChimeKind, ChimeSound, Config, CustomChimes, Language, MusicConfig,
    MusicSourceKind, NoiseColor, ShieldsConfig, Theme, TintConfig,
};
use crate::blocklist::normalize_domain;
use crate::launchers;
use crate::timing::{DEFAULT_BREAK_MINUTES, DEFAULT_FOCUS_MINUTES};
use crate::tint::{MAX_STRENGTH, MAX_TRANSITION_MINUTES, MINUTES_PER_DAY};

/// Lofi Girl's current YouTube live stream (default music source). The channel `/live` page always
/// points at the stream that is on air; individual stream ids end and change over time.
pub const LOFI_GIRL_URL: &str = "https://www.youtube.com/@LofiGirl/live";

/// `AppError.code` for a config that failed validation.
pub const CONFIG_INVALID: &str = "config.invalid";

/// Allowed focus length in minutes.
pub const FOCUS_MINUTES_RANGE: RangeInclusive<u32> = 1..=240;
/// Allowed break length in minutes.
pub const BREAK_MINUTES_RANGE: RangeInclusive<u32> = 1..=120;
/// Maximum number of music links.
pub const MAX_LINKS: usize = 200;
/// Default chime volume.
pub const DEFAULT_CHIME_VOLUME: f32 = 0.7;
/// Default music volume.
pub const DEFAULT_MUSIC_VOLUME: f32 = 0.6;
/// Default Microsoft identity platform tenant for Teams (work and school accounts).
pub const DEFAULT_TEAMS_TENANT: &str = "organizations";
/// Most per-break reminders (`breakReminders`).
pub const MAX_BREAK_REMINDERS: usize = 24;
/// Longest reminder in characters (after trimming).
pub const MAX_REMINDER_CHARS: usize = 160;
/// Longest custom chime file name in characters.
pub const MAX_CHIME_FILE_NAME_CHARS: usize = 100;
/// Largest audio file accepted as a custom chime (`import_chime`), 10 MiB.
pub const MAX_CUSTOM_CHIME_BYTES: u64 = 10 * 1024 * 1024;
/// Audio file extensions accepted as custom chimes (lowercase, without the dot).
pub const CUSTOM_CHIME_EXTENSIONS: [&str; 5] = ["mp3", "wav", "ogg", "flac", "m4a"];
/// Default calendar look-ahead (`calendar.horizonHours`).
pub const DEFAULT_CALENDAR_HORIZON_HOURS: u32 = 10;
/// Allowed calendar look-ahead in hours.
pub const CALENDAR_HORIZON_HOURS_RANGE: RangeInclusive<u32> = 1..=24;
/// Every Google OAuth client id ends with this (`<number>-<hash>.apps.googleusercontent.com`).
pub const GOOGLE_CLIENT_ID_SUFFIX: &str = ".apps.googleusercontent.com";
/// Longest Google OAuth client id accepted.
pub const MAX_GOOGLE_CLIENT_ID_LEN: usize = 255;
/// Longest Google OAuth client secret accepted (they look like `GOCSPX-` + 28 characters).
pub const MAX_GOOGLE_CLIENT_SECRET_LEN: usize = 100;

const MAX_TENANT_LEN: usize = 253;
const MS_PER_HOUR: i64 = 60 * 60 * 1000;
/// Characters Windows does not allow in file names (besides separators and control characters).
const FORBIDDEN_FILE_NAME_CHARS: [char; 7] = ['<', '>', ':', '"', '|', '?', '*'];
/// Windows device names that cannot be used as a file name stem (`nul.mp3`).
const RESERVED_FILE_STEMS: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];
const GUID_LEN: usize = 36;
const GUID_DASH_POSITIONS: [usize; 4] = [8, 13, 18, 23];

impl Default for Config {
    fn default() -> Self {
        Self {
            focus_minutes: DEFAULT_FOCUS_MINUTES,
            break_minutes: DEFAULT_BREAK_MINUTES,
            music: MusicConfig::default(),
            shields: ShieldsConfig::default(),
            chime_volume: DEFAULT_CHIME_VOLUME,
            track_apps: true,
            launch_on_login: false,
            language: Language::System,
            close_to_tray: None,
            phase_popups: false,
            tint: TintConfig::default(),
            theme: Theme::Midnight,
            chime_sound: ChimeSound::Midnight,
            custom_chimes: CustomChimes::default(),
            break_reminders: Vec::new(),
            repeat_reminders: false,
            track_titles: true,
            keep_history: true,
            calendar: CalendarConfig::default(),
        }
    }
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            microsoft: false,
            google: false,
            google_client_id: String::new(),
            google_client_secret: String::new(),
            ics: false,
            horizon_hours: DEFAULT_CALENDAR_HORIZON_HOURS,
        }
    }
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            // The bundled Open Lo-Fi tracks: offline, no consent page, no account.
            source: MusicSourceKind::Library,
            folder: None,
            links: Vec::new(),
            stream_url: LOFI_GIRL_URL.to_owned(),
            noise: NoiseColor::Brown,
            shuffle: true,
            volume: DEFAULT_MUSIC_VOLUME,
            pause_during_break: false,
        }
    }
}

impl Default for ShieldsConfig {
    fn default() -> Self {
        Self {
            mute_notifications: false,
            teams_dnd: false,
            teams_client_id: String::new(),
            teams_tenant: DEFAULT_TEAMS_TENANT.to_owned(),
            block_sites: true,
            keep_awake: true,
            close_launchers: false,
            launchers: launchers::default_launchers(),
        }
    }
}

impl Config {
    /// Validates and normalizes a config received from the UI or read from disk.
    ///
    /// Trims strings, trims links and drops empty ones (a bare `youtube.com/...` gets `https://`).
    /// The site blocklist is not part of the config: it lives in `websiteblacklist.txt`
    /// ([`crate::blocklist::Blocklist::parse`]).
    /// Game launchers lose duplicates and follow the canonical order ([`launchers::ALL`]).
    /// Break reminders are trimmed (control characters become spaces); empty ones stay as
    /// "no reminder for that break" placeholders. Custom chime names are trimmed (empty → `None`)
    /// and must be plain audio file names ([`is_chime_file_name`]).
    /// The Google client id and secret are trimmed ([`CalendarConfig::normalized`]).
    /// Out-of-range values are never clamped silently: they return `Err` with code
    /// `config.invalid` and `field` set to the camelCase path (`focusMinutes`, `music.volume`,
    /// `tint.startMinutes`, `breakReminders`, `customChimes.breakFile`, `calendar.horizonHours`).
    pub fn normalized(self) -> Result<Config, AppError> {
        let Config {
            focus_minutes,
            break_minutes,
            music,
            shields,
            chime_volume,
            track_apps,
            launch_on_login,
            language,
            close_to_tray,
            phase_popups,
            tint,
            theme,
            chime_sound,
            custom_chimes,
            break_reminders,
            repeat_reminders,
            track_titles,
            keep_history,
            calendar,
        } = self;

        check_range("focusMinutes", focus_minutes, FOCUS_MINUTES_RANGE)?;
        check_range("breakMinutes", break_minutes, BREAK_MINUTES_RANGE)?;
        check_unit_interval("chimeVolume", chime_volume)?;

        Ok(Config {
            focus_minutes,
            break_minutes,
            music: music.normalized()?,
            shields: shields.normalized()?,
            chime_volume,
            track_apps,
            launch_on_login,
            language,
            close_to_tray,
            phase_popups,
            tint: tint.normalized()?,
            theme,
            chime_sound,
            custom_chimes: custom_chimes.normalized()?,
            break_reminders: normalize_reminders(break_reminders)?,
            repeat_reminders,
            track_titles,
            keep_history,
            calendar: calendar.normalized()?,
        })
    }
}

impl CalendarConfig {
    /// Calendar part of [`Config::normalized`] (checked also while every source is off); errors use
    /// `calendar.*` field paths. `horizonHours` is `1..=24`; the Google client id is trimmed and must be
    /// empty or a Google OAuth client id ([`is_google_client_id`]); the client secret is trimmed and must be
    /// empty or pass [`is_google_client_secret`]. `google: true` with an empty id is accepted (the shell
    /// reports the source as not configured); an empty secret too (it is then not sent).
    pub fn normalized(self) -> Result<CalendarConfig, AppError> {
        let CalendarConfig {
            microsoft,
            google,
            google_client_id,
            google_client_secret,
            ics,
            horizon_hours,
        } = self;

        check_range(
            "calendar.horizonHours",
            horizon_hours,
            CALENDAR_HORIZON_HOURS_RANGE,
        )?;
        let google_client_id = google_client_id.trim();
        if !google_client_id.is_empty() && !is_google_client_id(google_client_id) {
            return Err(invalid(
                "calendar.googleClientId",
                format!(
                    "Google client id must look like 123-abc{GOOGLE_CLIENT_ID_SUFFIX} \
                     (letters, digits, '.', '-', '_'; at most {MAX_GOOGLE_CLIENT_ID_LEN} characters)"
                ),
            ));
        }
        let google_client_secret = google_client_secret.trim();
        if !google_client_secret.is_empty() && !is_google_client_secret(google_client_secret) {
            // The secret itself is never echoed back (it ends up in the log with the error).
            return Err(invalid(
                "calendar.googleClientSecret",
                format!(
                    "Google client secret may contain only letters, digits, '-' and '_' \
                     (at most {MAX_GOOGLE_CLIENT_SECRET_LEN} characters)"
                ),
            ));
        }

        Ok(CalendarConfig {
            microsoft,
            google,
            google_client_id: google_client_id.to_owned(),
            google_client_secret: google_client_secret.to_owned(),
            ics,
            horizon_hours,
        })
    }

    /// The look-ahead in milliseconds (for [`crate::calendar::plan_breaks`] and the calendar fetch).
    pub fn horizon_ms(&self) -> i64 {
        i64::from(self.horizon_hours).saturating_mul(MS_PER_HOUR)
    }
}

/// `true` for a Google OAuth client id: at most [`MAX_GOOGLE_CLIENT_ID_LEN`] characters of
/// `[A-Za-z0-9._-]`, ending in [`GOOGLE_CLIENT_ID_SUFFIX`] after a non-empty prefix that does not
/// start with `.` or `-`. It ends up in an OAuth URL query, so nothing else is allowed.
pub fn is_google_client_id(id: &str) -> bool {
    let Some(prefix) = id.strip_suffix(GOOGLE_CLIENT_ID_SUFFIX) else {
        return false;
    };
    id.len() <= MAX_GOOGLE_CLIENT_ID_LEN
        && !prefix.is_empty()
        && !prefix.starts_with(['.', '-'])
        && !prefix.ends_with('.')
        && !prefix.contains("..")
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// `true` for a plausible Google OAuth client secret: 1..=[`MAX_GOOGLE_CLIENT_SECRET_LEN`] characters of
/// `[A-Za-z0-9_-]` (e.g. `GOCSPX-…`). It ends up in a token request form, so nothing else is allowed.
pub fn is_google_client_secret(secret: &str) -> bool {
    !secret.is_empty()
        && secret.len() <= MAX_GOOGLE_CLIENT_SECRET_LEN
        && secret
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// The user's reminder for the `break_number`-th break of a session (1-based: the break after
/// focus cycle 1 is break 1, so the shell passes the snapshot's `cycle` when a break starts).
///
/// `None` for break 0 and for an entry that is empty after trimming. Past the end of
/// `break_reminders` it is `None` too, unless `repeat_reminders` is on: then the list starts over
/// (`[a, b]` → a, b, a, b, …).
pub fn reminder_for_break(cfg: &Config, break_number: u32) -> Option<String> {
    let reminders = &cfg.break_reminders;
    let index = usize::try_from(break_number.checked_sub(1)?).ok()?;
    let entry = match reminders.get(index) {
        Some(entry) => entry,
        None if cfg.repeat_reminders && !reminders.is_empty() => {
            reminders.get(index % reminders.len())?
        }
        None => return None,
    };
    let entry = entry.trim();
    (!entry.is_empty()).then(|| entry.to_owned())
}

/// `true` for a name that can be used as a custom chime file inside the app data `sounds` folder:
/// a plain file name (no `/`, `\`, `:`, `..`, control or other characters Windows forbids, no
/// leading dot, no trailing dot or space, not a device name like `nul.mp3`) of at most
/// [`MAX_CHIME_FILE_NAME_CHARS`] characters, with a non-empty stem and one of the
/// [`CUSTOM_CHIME_EXTENSIONS`] (any case).
pub fn is_chime_file_name(name: &str) -> bool {
    if name.is_empty()
        || name.chars().count() > MAX_CHIME_FILE_NAME_CHARS
        || name.contains("..")
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || name.chars().any(|c| {
            c == '/' || c == '\\' || c.is_control() || FORBIDDEN_FILE_NAME_CHARS.contains(&c)
        })
    {
        return false;
    }
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    let device = stem.split('.').next().unwrap_or(stem).trim_end();
    !stem.trim().is_empty()
        && chime_extension(extension).is_some()
        && !RESERVED_FILE_STEMS
            .iter()
            .any(|reserved| device.eq_ignore_ascii_case(reserved))
}

/// File name the shell stores an imported chime under: `break.<ext>` or `focus.<ext>`, with the
/// lowercase extension of `source` (a path or a file name). `None` when the extension is not one of
/// the [`CUSTOM_CHIME_EXTENSIONS`].
pub fn custom_chime_file_name(chime: ChimeKind, source: &str) -> Option<String> {
    let file_name = source.trim().rsplit(['/', '\\']).next()?;
    let (stem, extension) = file_name.rsplit_once('.')?;
    if stem.trim().is_empty() {
        return None;
    }
    let extension = chime_extension(extension)?;
    let prefix = match chime {
        ChimeKind::BreakStart => "break",
        ChimeKind::FocusStart => "focus",
    };
    Some(format!("{prefix}.{extension}"))
}

/// The allowed extension matching `extension` case-insensitively.
fn chime_extension(extension: &str) -> Option<&'static str> {
    CUSTOM_CHIME_EXTENSIONS
        .iter()
        .copied()
        .find(|allowed| extension.eq_ignore_ascii_case(allowed))
}

impl CustomChimes {
    /// Custom part of [`Config::normalized`]: trims both names (empty → `None`) and rejects
    /// anything but a plain audio file name ([`is_chime_file_name`]); errors use
    /// `customChimes.breakFile` / `customChimes.focusFile`.
    pub fn normalized(self) -> Result<CustomChimes, AppError> {
        Ok(CustomChimes {
            break_file: normalize_chime_file("customChimes.breakFile", self.break_file)?,
            focus_file: normalize_chime_file("customChimes.focusFile", self.focus_file)?,
        })
    }

    /// File to play for `chime` with `chimeSound: custom`: its own file, else the other one
    /// (`focus_file: None` = the break file for both); `None` when no custom file is set
    /// (the shell then plays the default sound).
    pub fn file_for(&self, chime: ChimeKind) -> Option<&str> {
        let (own, other) = match chime {
            ChimeKind::BreakStart => (&self.break_file, &self.focus_file),
            ChimeKind::FocusStart => (&self.focus_file, &self.break_file),
        };
        own.as_deref().or(other.as_deref())
    }
}

fn normalize_chime_file(field: &str, name: Option<String>) -> Result<Option<String>, AppError> {
    let Some(name) = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    if is_chime_file_name(&name) {
        return Ok(Some(name));
    }
    let allowed = CUSTOM_CHIME_EXTENSIONS.join(", ");
    Err(invalid(
        field,
        format!(
            "{field} must be a plain file name of at most {MAX_CHIME_FILE_NAME_CHARS} characters \
             ending in one of: {allowed}; got {name:?}"
        ),
    ))
}

/// Trims every reminder (control characters such as line breaks become spaces) and checks the
/// count and length limits. Empty entries are kept: they mean "no reminder for that break".
fn normalize_reminders(reminders: Vec<String>) -> Result<Vec<String>, AppError> {
    const FIELD: &str = "breakReminders";
    if reminders.len() > MAX_BREAK_REMINDERS {
        return Err(invalid(
            FIELD,
            format!(
                "at most {MAX_BREAK_REMINDERS} break reminders are allowed, got {}",
                reminders.len()
            ),
        ));
    }
    reminders
        .iter()
        .enumerate()
        .map(|(index, reminder)| {
            let text: String = reminder
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            let text = text.trim();
            let chars = text.chars().count();
            if chars > MAX_REMINDER_CHARS {
                return Err(invalid(
                    FIELD,
                    format!(
                        "break reminder {} is longer than {MAX_REMINDER_CHARS} characters ({chars})",
                        index + 1
                    ),
                ));
            }
            Ok(text.to_owned())
        })
        .collect()
}

impl MusicConfig {
    /// Music part of [`Config::normalized`]; errors use `music.*` field paths.
    pub fn normalized(self) -> Result<MusicConfig, AppError> {
        let MusicConfig {
            source,
            folder,
            links,
            stream_url,
            noise,
            shuffle,
            volume,
            pause_during_break,
        } = self;

        check_unit_interval("music.volume", volume)?;

        let folder = folder
            .map(|f| f.trim().to_owned())
            .filter(|f| !f.is_empty());

        let links = links
            .iter()
            .map(|link| link.trim())
            .filter(|link| !link.is_empty())
            .map(|link| normalize_link("music.links", link))
            .collect::<Result<Vec<_>, _>>()?;
        if links.len() > MAX_LINKS {
            return Err(invalid(
                "music.links",
                format!("at most {MAX_LINKS} links are allowed, got {}", links.len()),
            ));
        }

        let stream_url = stream_url.trim();
        let stream_url = if stream_url.is_empty() {
            String::new()
        } else {
            normalize_link("music.streamUrl", stream_url)?
        };

        Ok(MusicConfig {
            source,
            folder,
            links,
            stream_url,
            noise,
            shuffle,
            volume,
            pause_during_break,
        })
    }
}

impl ShieldsConfig {
    /// Shields part of [`Config::normalized`]; errors use `shields.*` field paths.
    pub fn normalized(self) -> Result<ShieldsConfig, AppError> {
        let ShieldsConfig {
            mute_notifications,
            teams_dnd,
            teams_client_id,
            teams_tenant,
            block_sites,
            keep_awake,
            close_launchers,
            launchers,
        } = self;

        let teams_client_id = normalize_client_id(&teams_client_id)?;
        let teams_tenant = normalize_tenant(&teams_tenant)?;

        Ok(ShieldsConfig {
            mute_notifications,
            teams_dnd,
            teams_client_id,
            teams_tenant,
            block_sites,
            keep_awake,
            close_launchers,
            launchers: launchers::normalized(&launchers),
        })
    }
}

impl TintConfig {
    /// Tint part of [`Config::normalized`] (checked also while disabled); errors use `tint.*`
    /// field paths. Times are `0..1440` and must differ, strength is `0..=100`, fades `0..=180`
    /// minutes.
    pub fn normalized(self) -> Result<TintConfig, AppError> {
        let TintConfig {
            enabled,
            start_minutes,
            end_minutes,
            strength,
            transition_minutes,
            mode,
        } = self;

        let last_minute = MINUTES_PER_DAY.saturating_sub(1);
        check_range("tint.startMinutes", start_minutes, 0..=last_minute)?;
        check_range("tint.endMinutes", end_minutes, 0..=last_minute)?;
        if start_minutes == end_minutes {
            return Err(invalid(
                "tint.endMinutes",
                format!("tint.endMinutes must differ from tint.startMinutes, got {end_minutes}"),
            ));
        }
        check_range("tint.strength", strength, 0..=MAX_STRENGTH)?;
        check_range(
            "tint.transitionMinutes",
            transition_minutes,
            0..=MAX_TRANSITION_MINUTES,
        )?;

        Ok(TintConfig {
            enabled,
            start_minutes,
            end_minutes,
            strength,
            transition_minutes,
            mode,
        })
    }
}

fn invalid(field: &str, message: impl Into<String>) -> AppError {
    AppError {
        code: CONFIG_INVALID.to_owned(),
        message: message.into(),
        field: Some(field.to_owned()),
    }
}

fn check_range(field: &str, value: u32, range: RangeInclusive<u32>) -> Result<(), AppError> {
    if range.contains(&value) {
        return Ok(());
    }
    let (min, max) = (*range.start(), *range.end());
    Err(invalid(
        field,
        format!("{field} must be between {min} and {max}, got {value}"),
    ))
}

fn check_unit_interval(field: &str, value: f32) -> Result<(), AppError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        return Ok(());
    }
    Err(invalid(
        field,
        format!("{field} must be between 0 and 1, got {value}"),
    ))
}

/// Accepts `http(s)://` links as they are; a bare domain link (`youtube.com/watch?v=...`)
/// gets `https://`. Anything else (other schemes, whitespace inside) is rejected.
fn normalize_link(field: &str, link: &str) -> Result<String, AppError> {
    if link.chars().any(char::is_whitespace) {
        return Err(invalid(
            field,
            format!("link must not contain spaces: {link}"),
        ));
    }
    for scheme in ["https://", "http://"] {
        let rest = link
            .get(..scheme.len())
            .filter(|prefix| prefix.eq_ignore_ascii_case(scheme))
            .and_then(|_| link.get(scheme.len()..));
        if let Some(rest) = rest {
            if rest.is_empty() || rest.starts_with(['/', '?', '#']) {
                return Err(invalid(field, format!("link has no host: {link}")));
            }
            return Ok(link.to_owned());
        }
    }
    if !link.contains("://") && normalize_domain(link).is_some() {
        return Ok(format!("https://{link}"));
    }
    Err(invalid(
        field,
        format!("only http(s) links are supported: {link}"),
    ))
}

/// Azure application (client) id: empty, or a GUID (optionally wrapped in braces).
fn normalize_client_id(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let id = trimmed
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .unwrap_or(trimmed)
        .trim();
    if id.is_empty() || is_guid(id) {
        return Ok(id.to_owned());
    }
    Err(invalid(
        "shields.teamsClientId",
        "Teams client id must be a GUID like 00000000-0000-0000-0000-000000000000",
    ))
}

/// Tenant id (GUID), domain (`contoso.onmicrosoft.com`) or `organizations` / `common`.
/// It ends up in a login URL path, so only `[a-z0-9.-]` is allowed. Empty means the default.
fn normalize_tenant(raw: &str) -> Result<String, AppError> {
    let tenant = raw.trim().to_ascii_lowercase();
    if tenant.is_empty() {
        return Ok(DEFAULT_TEAMS_TENANT.to_owned());
    }
    let valid = tenant.len() <= MAX_TENANT_LEN
        && tenant
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && !tenant.starts_with(['.', '-'])
        && !tenant.ends_with(['.', '-'])
        && !tenant.contains("..");
    if valid {
        return Ok(tenant);
    }
    Err(invalid(
        "shields.teamsTenant",
        format!("invalid tenant: {tenant}"),
    ))
}

fn is_guid(value: &str) -> bool {
    value.len() == GUID_LEN
        && value.bytes().enumerate().all(|(i, b)| {
            if GUID_DASH_POSITIONS.contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{GameLauncher, TintMode};

    fn field_of(result: Result<Config, AppError>) -> Option<String> {
        let err = result.expect_err("config should be rejected");
        assert_eq!(err.code, CONFIG_INVALID);
        err.field
    }

    fn with_music(music: MusicConfig) -> Config {
        Config {
            music,
            ..Config::default()
        }
    }

    fn with_shields(shields: ShieldsConfig) -> Config {
        Config {
            shields,
            ..Config::default()
        }
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn defaults_match_spec() {
        let cfg = Config::default();
        assert_eq!(cfg.focus_minutes, 45);
        assert_eq!(cfg.break_minutes, 15);
        assert!(approx(cfg.chime_volume, 0.7));
        assert!(cfg.track_apps);
        assert!(!cfg.launch_on_login);

        let music = &cfg.music;
        assert_eq!(music.source, MusicSourceKind::Library);
        assert_eq!(music.stream_url, LOFI_GIRL_URL);
        assert_eq!(music.noise, NoiseColor::Brown);
        assert!(music.shuffle);
        assert!(approx(music.volume, 0.6));
        assert!(!music.pause_during_break);
        assert!(music.links.is_empty());
        assert_eq!(music.folder, None);

        let shields = &cfg.shields;
        assert!(!shields.mute_notifications);
        assert!(!shields.teams_dnd);
        assert_eq!(shields.teams_client_id, "");
        assert_eq!(shields.teams_tenant, "organizations");
        assert!(shields.block_sites);
        assert!(shields.keep_awake);
        assert!(!shields.close_launchers);
        assert_eq!(
            shields.launchers,
            [
                GameLauncher::Steam,
                GameLauncher::Epic,
                GameLauncher::Gog,
                GameLauncher::Ubisoft,
                GameLauncher::Ea,
                GameLauncher::BattleNet,
                GameLauncher::Riot,
            ]
        );

        assert_eq!(cfg.language, Language::System);
        assert_eq!(cfg.close_to_tray, None);
        assert!(!cfg.phase_popups);
        assert_eq!(
            cfg.tint,
            TintConfig {
                enabled: false,
                start_minutes: 18 * 60,
                end_minutes: 9 * 60,
                strength: 45,
                transition_minutes: 30,
                mode: TintMode::NightLight,
            }
        );

        assert_eq!(cfg.theme, Theme::Midnight);
        assert_eq!(cfg.chime_sound, ChimeSound::Midnight);
        assert_eq!(cfg.custom_chimes, CustomChimes::default());
        assert_eq!(cfg.custom_chimes.break_file, None);
        assert_eq!(cfg.custom_chimes.focus_file, None);
        assert!(cfg.break_reminders.is_empty());
        assert!(!cfg.repeat_reminders);
        assert!(cfg.track_titles);
        assert!(cfg.keep_history);
        assert_eq!(
            cfg.calendar,
            CalendarConfig {
                microsoft: false,
                google: false,
                google_client_id: String::new(),
                google_client_secret: String::new(),
                ics: false,
                horizon_hours: 10,
            }
        );
        assert_eq!(cfg.calendar.horizon_ms(), 10 * 3_600_000);
    }

    #[test]
    fn config_written_before_calendars_loads_with_calendars_off() {
        let old: Config = serde_json::from_str(
            r#"{"focusMinutes": 50, "theme": "lunar", "keepHistory": false, "trackTitles": false}"#,
        )
        .expect("old config");
        assert_eq!(old.calendar, CalendarConfig::default());
        assert_eq!(old.clone().normalized(), Ok(old));

        let partial: Config =
            serde_json::from_str(r#"{"calendar":{"ics":true}}"#).expect("partial calendar");
        assert!(partial.calendar.ics);
        assert!(!partial.calendar.microsoft);
        assert_eq!(partial.calendar.horizon_hours, 10);

        // Written before the Google client secret: loads with an empty one.
        let before_secret: Config = serde_json::from_str(
            r#"{"calendar":{"microsoft":false,"google":true,
                "googleClientId":"1-x.apps.googleusercontent.com","ics":false,"horizonHours":8}}"#,
        )
        .expect("calendar without the secret");
        assert_eq!(before_secret.calendar.google_client_secret, "");
        assert_eq!(
            before_secret.calendar.google_client_id,
            "1-x.apps.googleusercontent.com"
        );
        assert_eq!(
            before_secret.clone().normalized(),
            Ok(before_secret.clone())
        );
        let json = serde_json::to_string(&before_secret).expect("serialize");
        assert!(json.contains(r#""googleClientSecret":"""#), "{json}");
    }

    #[test]
    fn validates_google_client_secret() {
        let with_secret = |secret: &str| Config {
            calendar: CalendarConfig {
                google: true,
                google_client_secret: secret.to_owned(),
                ..CalendarConfig::default()
            },
            ..Config::default()
        };
        let secret = "GOCSPX-abcDEF123_ghi-JKL456mno789pq";
        assert_eq!(
            with_secret(&format!(" {secret}\n"))
                .normalized()
                .expect("valid")
                .calendar
                .google_client_secret,
            secret
        );
        assert_eq!(
            with_secret("  ")
                .normalized()
                .expect("empty is allowed")
                .calendar
                .google_client_secret,
            ""
        );
        let longest = "a".repeat(MAX_GOOGLE_CLIENT_SECRET_LEN);
        assert!(is_google_client_secret(&longest));
        let too_long = "a".repeat(MAX_GOOGLE_CLIENT_SECRET_LEN + 1);
        for bad in [
            too_long.as_str(),
            "GOCSPX abc",
            "GOCSPX.abc",
            "GOCSPX/abc",
            "abc&client_id=x",
            "abc=",
            "sekret-żółw",
            "abc\tdef",
        ] {
            assert!(!is_google_client_secret(bad), "{bad}");
            let err = with_secret(bad)
                .normalized()
                .expect_err("invalid secret is rejected");
            assert_eq!(err.code, CONFIG_INVALID);
            assert_eq!(
                err.field.as_deref(),
                Some("calendar.googleClientSecret"),
                "{bad}"
            );
            assert!(!err.message.contains(bad), "the secret is not echoed");
        }
        assert!(!is_google_client_secret(""));
    }

    #[test]
    fn validates_calendar() {
        let with_calendar = |calendar: CalendarConfig| Config {
            calendar,
            ..Config::default()
        };
        for hours in [0, 25, u32::MAX] {
            let cfg = with_calendar(CalendarConfig {
                horizon_hours: hours,
                ..CalendarConfig::default()
            });
            assert_eq!(
                field_of(cfg.normalized()).as_deref(),
                Some("calendar.horizonHours"),
                "{hours}"
            );
        }
        for hours in [1, 10, 24] {
            let cfg = with_calendar(CalendarConfig {
                horizon_hours: hours,
                ..CalendarConfig::default()
            });
            let calendar = cfg.normalized().expect("valid").calendar;
            assert_eq!(calendar.horizon_hours, hours);
            assert_eq!(calendar.horizon_ms(), i64::from(hours) * 3_600_000);
        }

        let id = "123456789012-abc_DEF.ghi-jkl.apps.googleusercontent.com";
        let ok = with_calendar(CalendarConfig {
            google: true,
            google_client_id: format!("  {id}\t"),
            ..CalendarConfig::default()
        });
        assert_eq!(
            ok.normalized().expect("valid").calendar.google_client_id,
            id
        );

        // Google on without a client id is stored as it is (the source is just not configured).
        let unconfigured = with_calendar(CalendarConfig {
            google: true,
            google_client_id: "   ".to_owned(),
            ..CalendarConfig::default()
        });
        let calendar = unconfigured.normalized().expect("valid").calendar;
        assert!(calendar.google);
        assert_eq!(calendar.google_client_id, "");

        let too_long = format!(
            "{}{GOOGLE_CLIENT_ID_SUFFIX}",
            "a".repeat(MAX_GOOGLE_CLIENT_ID_LEN + 1 - GOOGLE_CLIENT_ID_SUFFIX.len())
        );
        let longest = format!(
            "{}{GOOGLE_CLIENT_ID_SUFFIX}",
            "a".repeat(MAX_GOOGLE_CLIENT_ID_LEN - GOOGLE_CLIENT_ID_SUFFIX.len())
        );
        assert!(is_google_client_id(&longest));
        for bad in [
            "abc",
            "123-abc",
            ".apps.googleusercontent.com",
            "apps.googleusercontent.com",
            "123-abc.apps.googleusercontent.com.evil.example",
            "123-abc.APPS.GOOGLEUSERCONTENT.COM",
            "123 abc.apps.googleusercontent.com",
            "123/abc.apps.googleusercontent.com",
            "123?x=1&abc.apps.googleusercontent.com",
            "123#abc.apps.googleusercontent.com",
            "żółw.apps.googleusercontent.com",
            "-123.apps.googleusercontent.com",
            ".123.apps.googleusercontent.com",
            "123..abc.apps.googleusercontent.com",
            "123..apps.googleusercontent.com",
            too_long.as_str(),
        ] {
            assert!(!is_google_client_id(bad), "{bad}");
            let cfg = with_calendar(CalendarConfig {
                google_client_id: bad.to_owned(),
                ..CalendarConfig::default()
            });
            assert_eq!(
                field_of(cfg.normalized()).as_deref(),
                Some("calendar.googleClientId"),
                "{bad}"
            );
        }
    }

    #[test]
    fn config_written_before_themes_reminders_and_history_options_loads_with_defaults() {
        // config.json as saved by the version with language, tint and launchers (no newer fields).
        let old: Config = serde_json::from_str(
            r#"{
                "focusMinutes": 45, "breakMinutes": 15,
                "music": {
                    "source": "stream", "folder": null, "links": [],
                    "streamUrl": "https://www.youtube.com/@LofiGirl/live",
                    "noise": "brown", "shuffle": true, "volume": 0.6, "pauseDuringBreak": false
                },
                "shields": {
                    "muteNotifications": false, "teamsDnd": false, "teamsClientId": "",
                    "teamsTenant": "organizations", "blockSites": true, "keepAwake": true,
                    "closeLaunchers": true, "launchers": ["steam"]
                },
                "chimeVolume": 0.7, "trackApps": true, "launchOnLogin": false,
                "language": "pl", "closeToTray": true, "phasePopups": true,
                "tint": {
                    "enabled": true, "startMinutes": 1200, "endMinutes": 420,
                    "strength": 60, "transitionMinutes": 15
                }
            }"#,
        )
        .expect("old config");
        // An explicit choice of the old default source is kept.
        assert_eq!(old.music.source, MusicSourceKind::Stream);
        assert!(old.tint.enabled);
        assert_eq!(old.tint.strength, 60);
        assert_eq!(old.tint.mode, TintMode::NightLight);
        assert_eq!(old.theme, Theme::Midnight);
        assert_eq!(old.chime_sound, ChimeSound::Midnight);
        assert_eq!(old.custom_chimes, CustomChimes::default());
        assert!(old.break_reminders.is_empty());
        assert!(!old.repeat_reminders);
        assert!(old.track_titles);
        assert!(old.keep_history);
        assert_eq!(old.clone().normalized(), Ok(old));
    }

    #[test]
    fn reminders_follow_break_numbers() {
        let cfg = Config {
            break_reminders: vec![
                "Stretch".to_owned(),
                "Water + dog".to_owned(),
                "Lunch".to_owned(),
            ],
            ..Config::default()
        };
        assert_eq!(reminder_for_break(&cfg, 0), None);
        assert_eq!(reminder_for_break(&cfg, 1).as_deref(), Some("Stretch"));
        assert_eq!(reminder_for_break(&cfg, 2).as_deref(), Some("Water + dog"));
        assert_eq!(reminder_for_break(&cfg, 3).as_deref(), Some("Lunch"));
        assert_eq!(reminder_for_break(&cfg, 4), None);
        assert_eq!(reminder_for_break(&cfg, u32::MAX), None);

        let repeating = Config {
            repeat_reminders: true,
            ..cfg
        };
        let cycle: Vec<_> = (1..=7).map(|n| reminder_for_break(&repeating, n)).collect();
        assert_eq!(
            cycle,
            [
                Some("Stretch"),
                Some("Water + dog"),
                Some("Lunch"),
                Some("Stretch"),
                Some("Water + dog"),
                Some("Lunch"),
                Some("Stretch"),
            ]
            .map(|r| r.map(str::to_owned))
        );
        assert_eq!(reminder_for_break(&repeating, 0), None);
        assert!(reminder_for_break(&repeating, u32::MAX).is_some());
    }

    #[test]
    fn empty_reminders_mean_no_reminder() {
        let cfg = Config {
            break_reminders: vec![String::new(), "  Walk  ".to_owned(), "   ".to_owned()],
            repeat_reminders: true,
            ..Config::default()
        };
        let got: Vec<_> = (1..=6).map(|n| reminder_for_break(&cfg, n)).collect();
        let walk = Some("Walk".to_owned());
        assert_eq!(got, [None, walk.clone(), None, None, walk, None]);

        let none = Config::default();
        assert_eq!(reminder_for_break(&none, 1), None);
        let repeat_none = Config {
            repeat_reminders: true,
            ..Config::default()
        };
        assert_eq!(reminder_for_break(&repeat_none, 5), None);
    }

    #[test]
    fn normalizes_and_limits_break_reminders() {
        let cfg = Config {
            break_reminders: vec![
                "  Rozciąganie  ".to_owned(),
                String::new(),
                "Woda\r\ni pies\t".to_owned(),
                "   ".to_owned(),
            ],
            ..Config::default()
        };
        let reminders = cfg.normalized().expect("valid").break_reminders;
        assert_eq!(reminders, ["Rozciąganie", "", "Woda  i pies", ""]);

        // 160 characters (not bytes) after trimming are fine; 161 are not.
        let ok = Config {
            break_reminders: vec![format!("  {}  ", "ż".repeat(MAX_REMINDER_CHARS))],
            ..Config::default()
        };
        assert!(ok.normalized().is_ok());
        let long = Config {
            break_reminders: vec!["a".repeat(MAX_REMINDER_CHARS + 1)],
            ..Config::default()
        };
        assert_eq!(
            field_of(long.normalized()).as_deref(),
            Some("breakReminders")
        );

        let full = Config {
            break_reminders: vec![String::new(); MAX_BREAK_REMINDERS],
            ..Config::default()
        };
        assert!(full.normalized().is_ok());
        let too_many = Config {
            break_reminders: vec!["x".to_owned(); MAX_BREAK_REMINDERS + 1],
            ..Config::default()
        };
        assert_eq!(
            field_of(too_many.normalized()).as_deref(),
            Some("breakReminders")
        );
    }

    #[test]
    fn validates_custom_chime_file_names() {
        for good in [
            "break.mp3",
            "focus.WAV",
            "my chime (1).ogg",
            "dzwonek-żaba.flac",
            "a.m4a",
            "con-test.mp3",
            "nullify.wav",
        ] {
            assert!(is_chime_file_name(good), "{good}");
        }
        let long_ok = format!("{}.mp3", "a".repeat(MAX_CHIME_FILE_NAME_CHARS - 4));
        assert!(is_chime_file_name(&long_ok));
        let too_long = format!("{}.mp3", "a".repeat(MAX_CHIME_FILE_NAME_CHARS - 3));
        assert!(!is_chime_file_name(&too_long));
        for bad in [
            "",
            "mp3",
            ".mp3",
            "chime",
            "chime.txt",
            "chime.mp3.exe",
            "chime.",
            "../break.mp3",
            "..\\break.mp3",
            "a..b.mp3",
            "sounds/break.mp3",
            "sounds\\break.mp3",
            "C:break.mp3",
            "C:\\break.mp3",
            "/etc/break.mp3",
            "break.mp3:stream",
            "br<e>ak.mp3",
            "what?.mp3",
            "star*.mp3",
            "pipe|.mp3",
            "quote\".mp3",
            "line\nbreak.mp3",
            "nul.mp3",
            "NUL.tar.mp3",
            "Com1.wav",
            "lpt9.ogg",
            "trailing .mp3 ",
        ] {
            assert!(!is_chime_file_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn normalizes_custom_chimes_in_config() {
        let cfg = Config {
            chime_sound: ChimeSound::Custom,
            custom_chimes: CustomChimes {
                break_file: Some("  break.mp3 ".to_owned()),
                focus_file: Some("   ".to_owned()),
            },
            ..Config::default()
        };
        let normalized = cfg.normalized().expect("valid");
        assert_eq!(normalized.chime_sound, ChimeSound::Custom);
        assert_eq!(
            normalized.custom_chimes,
            CustomChimes {
                break_file: Some("break.mp3".to_owned()),
                focus_file: None,
            }
        );

        for (chimes, field) in [
            (
                CustomChimes {
                    break_file: Some("..\\..\\evil.mp3".to_owned()),
                    focus_file: None,
                },
                "customChimes.breakFile",
            ),
            (
                CustomChimes {
                    break_file: Some("break.mp3".to_owned()),
                    focus_file: Some("focus.exe".to_owned()),
                },
                "customChimes.focusFile",
            ),
        ] {
            let cfg = Config {
                custom_chimes: chimes,
                ..Config::default()
            };
            assert_eq!(field_of(cfg.normalized()).as_deref(), Some(field));
        }
    }

    #[test]
    fn custom_chime_file_falls_back_to_the_other_one() {
        let both = CustomChimes {
            break_file: Some("break.mp3".to_owned()),
            focus_file: Some("focus.wav".to_owned()),
        };
        assert_eq!(both.file_for(ChimeKind::BreakStart), Some("break.mp3"));
        assert_eq!(both.file_for(ChimeKind::FocusStart), Some("focus.wav"));

        let break_only = CustomChimes {
            break_file: Some("break.mp3".to_owned()),
            focus_file: None,
        };
        assert_eq!(
            break_only.file_for(ChimeKind::FocusStart),
            Some("break.mp3")
        );

        let focus_only = CustomChimes {
            break_file: None,
            focus_file: Some("focus.wav".to_owned()),
        };
        assert_eq!(
            focus_only.file_for(ChimeKind::BreakStart),
            Some("focus.wav")
        );

        let none = CustomChimes::default();
        assert_eq!(none.file_for(ChimeKind::BreakStart), None);
        assert_eq!(none.file_for(ChimeKind::FocusStart), None);
    }

    #[test]
    fn names_imported_chime_files() {
        for (chime, source, expected) in [
            (
                ChimeKind::BreakStart,
                "C:\\Users\\me\\Music\\Bell.MP3",
                Some("break.mp3"),
            ),
            (
                ChimeKind::FocusStart,
                "/home/me/gong.flac",
                Some("focus.flac"),
            ),
            (ChimeKind::FocusStart, "ding.m4a", Some("focus.m4a")),
            (
                ChimeKind::BreakStart,
                "C:\\a.b\\song.ogg",
                Some("break.ogg"),
            ),
            (ChimeKind::BreakStart, "C:\\Music\\notes.txt", None),
            (ChimeKind::BreakStart, "C:\\Music\\.mp3", None),
            (ChimeKind::BreakStart, "C:\\Music.mp3\\file", None),
            (ChimeKind::BreakStart, "", None),
        ] {
            let name = custom_chime_file_name(chime, source);
            assert_eq!(name.as_deref(), expected, "{source}");
            if let Some(name) = name {
                assert!(is_chime_file_name(&name));
            }
        }
    }

    #[test]
    fn config_written_before_language_tint_and_launchers_loads_with_defaults() {
        // config.json as saved by the previous version (every field it knew, none of the new ones).
        let old: Config = serde_json::from_str(
            r#"{
                "focusMinutes": 50,
                "breakMinutes": 10,
                "music": {
                    "source": "noise", "folder": null, "links": [], "streamUrl": "",
                    "noise": "pink", "shuffle": false, "volume": 0.4, "pauseDuringBreak": true
                },
                "shields": {
                    "muteNotifications": true, "teamsDnd": false, "teamsClientId": "",
                    "teamsTenant": "organizations", "blockSites": true, "keepAwake": false
                },
                "chimeVolume": 0.5,
                "trackApps": false,
                "launchOnLogin": true
            }"#,
        )
        .expect("old config");
        assert_eq!(old.focus_minutes, 50);
        assert_eq!(old.music.noise, NoiseColor::Pink);
        assert!(old.shields.mute_notifications);
        assert!(!old.shields.keep_awake);
        assert!(old.launch_on_login);

        let defaults = Config::default();
        assert_eq!(old.language, defaults.language);
        assert_eq!(old.close_to_tray, None);
        assert!(!old.phase_popups);
        assert_eq!(old.tint, defaults.tint);
        assert!(!old.shields.close_launchers);
        assert_eq!(old.shields.launchers, defaults.shields.launchers);
        assert_eq!(old.clone().normalized(), Ok(old));
    }

    #[test]
    fn partial_tint_falls_back_to_defaults() {
        let cfg: Config =
            serde_json::from_str(r#"{"tint":{"enabled":true,"strength":70}}"#).expect("partial");
        assert!(cfg.tint.enabled);
        assert_eq!(cfg.tint.strength, 70);
        assert_eq!(cfg.tint.start_minutes, 1080);
        assert_eq!(cfg.tint.end_minutes, 540);
        assert_eq!(cfg.tint.transition_minutes, 30);
        assert_eq!(cfg.tint.mode, TintMode::NightLight);

        let gamma: Config = serde_json::from_str(r#"{"tint":{"mode":"gamma"}}"#).expect("mode");
        assert_eq!(gamma.tint.mode, TintMode::Gamma);
        assert!(!gamma.tint.enabled);
    }

    #[test]
    fn new_fields_round_trip() {
        for close_to_tray in [None, Some(true), Some(false)] {
            let cfg = Config {
                language: Language::Pl,
                close_to_tray,
                phase_popups: true,
                tint: TintConfig {
                    enabled: true,
                    start_minutes: 1200,
                    end_minutes: 420,
                    strength: 80,
                    transition_minutes: 0,
                    mode: TintMode::Gamma,
                },
                shields: ShieldsConfig {
                    close_launchers: true,
                    launchers: vec![GameLauncher::Xbox, GameLauncher::Itch],
                    ..ShieldsConfig::default()
                },
                theme: Theme::Pixel,
                chime_sound: ChimeSound::Custom,
                custom_chimes: CustomChimes {
                    break_file: Some("break.mp3".to_owned()),
                    focus_file: Some("focus.ogg".to_owned()),
                },
                break_reminders: vec!["Stretch".to_owned(), String::new(), "Lunch".to_owned()],
                repeat_reminders: true,
                track_titles: false,
                keep_history: false,
                calendar: CalendarConfig {
                    microsoft: true,
                    google: true,
                    google_client_id: "42-x.apps.googleusercontent.com".to_owned(),
                    google_client_secret: "GOCSPX-test_Secret-42".to_owned(),
                    ics: true,
                    horizon_hours: 24,
                },
                ..Config::default()
            };
            let json = serde_json::to_string(&cfg).expect("serialize");
            let back: Config = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, cfg);
            assert_eq!(back.clone().normalized(), Ok(back));
        }
    }

    #[test]
    fn default_config_is_valid_and_unchanged_by_normalization() {
        let cfg = Config::default();
        assert_eq!(cfg.clone().normalized(), Ok(cfg));
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let empty: Config = serde_json::from_str("{}").expect("empty object");
        assert_eq!(empty, Config::default());

        let partial: Config = serde_json::from_str(
            r#"{"focusMinutes":25,"music":{"volume":0.3},"shields":{"blockSites":false}}"#,
        )
        .expect("partial config");
        assert_eq!(partial.focus_minutes, 25);
        assert_eq!(partial.break_minutes, 15);
        assert!(approx(partial.music.volume, 0.3));
        assert_eq!(partial.music.stream_url, LOFI_GIRL_URL);
        assert!(!partial.shields.block_sites);
        assert!(partial.shields.keep_awake);
    }

    #[test]
    fn ignores_the_removed_blocklist_field() {
        // A config.json written before websiteblacklist.txt still loads; the old list is dropped.
        let old: Config = serde_json::from_str(
            r#"{"shields":{"blockSites":true,"blocklist":["reddit.com","kwejk.pl"]}}"#,
        )
        .expect("old config");
        assert_eq!(old, Config::default());
        let json = serde_json::to_value(&old).expect("serialize");
        assert!(json["shields"].get("blocklist").is_none());
    }

    #[test]
    fn serializes_with_camel_case_keys() {
        let json = serde_json::to_value(Config::default()).expect("serialize");
        assert_eq!(json["focusMinutes"], 45);
        assert_eq!(json["breakMinutes"], 15);
        assert_eq!(json["trackApps"], true);
        assert_eq!(json["launchOnLogin"], false);
        assert_eq!(json["music"]["streamUrl"], LOFI_GIRL_URL);
        assert_eq!(json["music"]["source"], "library");
        assert_eq!(json["music"]["noise"], "brown");
        assert_eq!(json["music"]["pauseDuringBreak"], false);
        assert_eq!(json["shields"]["teamsClientId"], "");
        assert_eq!(json["shields"]["teamsTenant"], "organizations");
        assert_eq!(json["shields"]["blockSites"], true);
        assert_eq!(json["shields"]["keepAwake"], true);
        assert_eq!(json["shields"]["muteNotifications"], false);
        assert_eq!(json["shields"]["closeLaunchers"], false);
        assert_eq!(
            json["shields"]["launchers"],
            serde_json::json!(["steam", "epic", "gog", "ubisoft", "ea", "battleNet", "riot"])
        );
        assert_eq!(json["language"], "system");
        assert!(json["closeToTray"].is_null());
        assert_eq!(json["phasePopups"], false);
        assert_eq!(json["tint"]["enabled"], false);
        assert_eq!(json["tint"]["startMinutes"], 1080);
        assert_eq!(json["tint"]["endMinutes"], 540);
        assert_eq!(json["tint"]["strength"], 45);
        assert_eq!(json["tint"]["transitionMinutes"], 30);
        assert_eq!(json["tint"]["mode"], "nightLight");
        assert_eq!(json["theme"], "midnight");
        assert_eq!(json["chimeSound"], "midnight");
        assert!(json["customChimes"]["breakFile"].is_null());
        assert!(json["customChimes"]["focusFile"].is_null());
        assert_eq!(json["breakReminders"], serde_json::json!([]));
        assert_eq!(json["repeatReminders"], false);
        assert_eq!(json["trackTitles"], true);
        assert_eq!(json["keepHistory"], true);
        assert_eq!(
            json["calendar"],
            serde_json::json!({
                "microsoft": false,
                "google": false,
                "googleClientId": "",
                "googleClientSecret": "",
                "ics": false,
                "horizonHours": 10,
            })
        );

        let gamma = serde_json::to_value(TintMode::Gamma).expect("serialize");
        assert_eq!(gamma, "gamma");
        let themes = [
            (Theme::Midnight, "midnight"),
            (Theme::Vaporwave, "vaporwave"),
            (Theme::Lunar, "lunar"),
            (Theme::Autumn, "autumn"),
            (Theme::Dark, "dark"),
            (Theme::Light, "light"),
            (Theme::Pixel, "pixel"),
        ];
        for (theme, name) in themes {
            assert_eq!(serde_json::to_value(theme).expect("serialize"), name);
        }
        let sounds = [
            (ChimeSound::Midnight, "midnight"),
            (ChimeSound::Vaporwave, "vaporwave"),
            (ChimeSound::Lunar, "lunar"),
            (ChimeSound::Autumn, "autumn"),
            (ChimeSound::Classic, "classic"),
            (ChimeSound::Soft, "soft"),
            (ChimeSound::Chiptune, "chiptune"),
            (ChimeSound::Custom, "custom"),
        ];
        for (sound, name) in sounds {
            assert_eq!(serde_json::to_value(sound).expect("serialize"), name);
        }
    }

    #[test]
    fn validates_tint() {
        let with_tint = |tint: TintConfig| Config {
            tint,
            ..Config::default()
        };
        let base = TintConfig {
            enabled: true,
            ..TintConfig::default()
        };
        for (bad, field) in [
            (
                TintConfig {
                    start_minutes: 1440,
                    ..base.clone()
                },
                "tint.startMinutes",
            ),
            (
                TintConfig {
                    end_minutes: 1440,
                    ..base.clone()
                },
                "tint.endMinutes",
            ),
            (
                TintConfig {
                    start_minutes: 600,
                    end_minutes: 600,
                    ..base.clone()
                },
                "tint.endMinutes",
            ),
            (
                TintConfig {
                    strength: 101,
                    ..base.clone()
                },
                "tint.strength",
            ),
            (
                TintConfig {
                    transition_minutes: 181,
                    ..base.clone()
                },
                "tint.transitionMinutes",
            ),
            (
                // Checked also while the tint is off.
                TintConfig {
                    enabled: false,
                    strength: 500,
                    ..base.clone()
                },
                "tint.strength",
            ),
        ] {
            assert_eq!(
                field_of(with_tint(bad.clone()).normalized()).as_deref(),
                Some(field),
                "{bad:?}"
            );
        }
        for good in [
            base.clone(),
            TintConfig {
                start_minutes: 0,
                end_minutes: 1439,
                strength: 0,
                transition_minutes: 0,
                ..base.clone()
            },
            TintConfig {
                start_minutes: 1439,
                end_minutes: 0,
                strength: 100,
                transition_minutes: 180,
                ..base.clone()
            },
            TintConfig {
                start_minutes: 13 * 60,
                end_minutes: 15 * 60,
                ..base.clone()
            },
        ] {
            let cfg = with_tint(good.clone());
            assert_eq!(cfg.normalized().map(|c| c.tint), Ok(good));
        }
    }

    #[test]
    fn dedupes_launchers_in_canonical_order() {
        let cfg = with_shields(ShieldsConfig {
            close_launchers: true,
            launchers: vec![
                GameLauncher::Riot,
                GameLauncher::Steam,
                GameLauncher::Itch,
                GameLauncher::Riot,
                GameLauncher::Steam,
            ],
            ..ShieldsConfig::default()
        });
        let shields = cfg.normalized().expect("valid").shields;
        assert!(shields.close_launchers);
        assert_eq!(
            shields.launchers,
            [GameLauncher::Steam, GameLauncher::Riot, GameLauncher::Itch]
        );

        let none = with_shields(ShieldsConfig {
            launchers: Vec::new(),
            ..ShieldsConfig::default()
        });
        assert!(
            none.normalized()
                .expect("valid")
                .shields
                .launchers
                .is_empty()
        );
    }

    #[test]
    fn rejects_out_of_range_minutes() {
        for (focus, brk, field) in [
            (0, 15, "focusMinutes"),
            (241, 15, "focusMinutes"),
            (45, 0, "breakMinutes"),
            (45, 121, "breakMinutes"),
        ] {
            let cfg = Config {
                focus_minutes: focus,
                break_minutes: brk,
                ..Config::default()
            };
            assert_eq!(field_of(cfg.normalized()).as_deref(), Some(field));
        }
        for (focus, brk) in [(1, 1), (240, 120), (25, 5), (90, 20)] {
            let cfg = Config {
                focus_minutes: focus,
                break_minutes: brk,
                ..Config::default()
            };
            assert!(cfg.normalized().is_ok(), "{focus}/{brk}");
        }
    }

    #[test]
    fn rejects_invalid_volumes() {
        for bad in [-0.1, 1.01, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let chime = Config {
                chime_volume: bad,
                ..Config::default()
            };
            assert_eq!(field_of(chime.normalized()).as_deref(), Some("chimeVolume"));

            let music = with_music(MusicConfig {
                volume: bad,
                ..MusicConfig::default()
            });
            assert_eq!(
                field_of(music.normalized()).as_deref(),
                Some("music.volume")
            );
        }
        for good in [0.0, 0.5, 1.0] {
            let cfg = Config {
                chime_volume: good,
                music: MusicConfig {
                    volume: good,
                    ..MusicConfig::default()
                },
                ..Config::default()
            };
            assert!(cfg.normalized().is_ok());
        }
    }

    #[test]
    fn trims_links_and_drops_empty_ones() {
        let cfg = with_music(MusicConfig {
            links: vec![
                "  https://www.youtube.com/watch?v=abc  ".to_owned(),
                String::new(),
                "   ".to_owned(),
                "music.youtube.com/watch?v=xyz".to_owned(),
                "HTTP://radio.example.org:8000/live.mp3".to_owned(),
            ],
            folder: Some("   ".to_owned()),
            ..MusicConfig::default()
        });
        let music = cfg.normalized().expect("valid").music;
        assert_eq!(
            music.links,
            [
                "https://www.youtube.com/watch?v=abc",
                "https://music.youtube.com/watch?v=xyz",
                "HTTP://radio.example.org:8000/live.mp3",
            ]
        );
        assert_eq!(music.folder, None);
    }

    #[test]
    fn rejects_unsupported_links() {
        for bad in [
            "spotify:track:123",
            "ftp://example.com/a.mp3",
            "https://",
            "not a link",
        ] {
            let cfg = with_music(MusicConfig {
                links: vec![bad.to_owned()],
                ..MusicConfig::default()
            });
            assert_eq!(
                field_of(cfg.normalized()).as_deref(),
                Some("music.links"),
                "{bad}"
            );
        }
    }

    #[test]
    fn rejects_too_many_links() {
        let links = (0..=MAX_LINKS)
            .map(|i| format!("https://example.com/{i}.mp3"))
            .collect();
        let cfg = with_music(MusicConfig {
            links,
            ..MusicConfig::default()
        });
        assert_eq!(field_of(cfg.normalized()).as_deref(), Some("music.links"));
    }

    #[test]
    fn normalizes_stream_url_and_folder() {
        let cfg = with_music(MusicConfig {
            stream_url: "  youtube.com/live/jfKfPfyJRdk ".to_owned(),
            folder: Some("  C:\\Music\\Lofi  ".to_owned()),
            ..MusicConfig::default()
        });
        let music = cfg.normalized().expect("valid").music;
        assert_eq!(music.stream_url, "https://youtube.com/live/jfKfPfyJRdk");
        assert_eq!(music.folder.as_deref(), Some("C:\\Music\\Lofi"));

        let empty = with_music(MusicConfig {
            stream_url: "   ".to_owned(),
            ..MusicConfig::default()
        });
        assert_eq!(empty.normalized().expect("valid").music.stream_url, "");

        let bad = with_music(MusicConfig {
            stream_url: "javascript:alert(1)".to_owned(),
            ..MusicConfig::default()
        });
        assert_eq!(
            field_of(bad.normalized()).as_deref(),
            Some("music.streamUrl")
        );
    }

    #[test]
    fn validates_teams_client_id() {
        let ok = with_shields(ShieldsConfig {
            teams_client_id: " {0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0} ".to_owned(),
            ..ShieldsConfig::default()
        });
        assert_eq!(
            ok.normalized().expect("valid").shields.teams_client_id,
            "0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0"
        );

        let empty = with_shields(ShieldsConfig {
            teams_client_id: "   ".to_owned(),
            ..ShieldsConfig::default()
        });
        assert_eq!(
            empty.normalized().expect("valid").shields.teams_client_id,
            ""
        );

        for bad in [
            "abc",
            "0f1e2d3c4b5a69788796a5b4c3d2e1f0",
            "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1fz",
        ] {
            let cfg = with_shields(ShieldsConfig {
                teams_client_id: bad.to_owned(),
                ..ShieldsConfig::default()
            });
            assert_eq!(
                field_of(cfg.normalized()).as_deref(),
                Some("shields.teamsClientId"),
                "{bad}"
            );
        }
    }

    #[test]
    fn validates_teams_tenant() {
        for (input, expected) in [
            ("", "organizations"),
            ("  Common ", "common"),
            ("Contoso.onmicrosoft.com", "contoso.onmicrosoft.com"),
            (
                "72f988bf-86f1-41af-91ab-2d7cd011db47",
                "72f988bf-86f1-41af-91ab-2d7cd011db47",
            ),
        ] {
            let cfg = with_shields(ShieldsConfig {
                teams_tenant: input.to_owned(),
                ..ShieldsConfig::default()
            });
            assert_eq!(
                cfg.normalized().expect("valid").shields.teams_tenant,
                expected
            );
        }
        for bad in [
            "../evil", "a/b", "a..b", ".common", "common-", "a b", "ten?ant",
        ] {
            let cfg = with_shields(ShieldsConfig {
                teams_tenant: bad.to_owned(),
                ..ShieldsConfig::default()
            });
            assert_eq!(
                field_of(cfg.normalized()).as_deref(),
                Some("shields.teamsTenant"),
                "{bad}"
            );
        }
    }

    #[test]
    fn normalization_is_idempotent() {
        let messy = Config {
            focus_minutes: 50,
            break_minutes: 10,
            music: MusicConfig {
                links: vec![
                    " youtu.be/abc ".to_owned(),
                    "https://example.com/a.mp3".to_owned(),
                ],
                stream_url: " www.youtube.com/watch?v=jfKfPfyJRdk ".to_owned(),
                folder: Some(" /home/me/music ".to_owned()),
                ..MusicConfig::default()
            },
            shields: ShieldsConfig {
                teams_tenant: " Organizations ".to_owned(),
                ..ShieldsConfig::default()
            },
            ..Config::default()
        };
        let once = messy.normalized().expect("valid");
        let twice = once.clone().normalized().expect("still valid");
        assert_eq!(once, twice);
        assert_eq!(once.shields.teams_tenant, "organizations");
        assert_eq!(once.music.links[0], "https://youtu.be/abc");
    }
}
