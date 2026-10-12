//! Config defaults and validation. The shell persists only configs that passed
//! [`Config::normalized`]; the UI shows the returned `AppError.field` next to the input.

use std::collections::HashSet;
use std::ops::RangeInclusive;

use crate::api::{AppError, Config, MusicConfig, MusicSourceKind, NoiseColor, ShieldsConfig};
use crate::blocklist::{DEFAULT_BLOCKLIST, normalize_domain};
use crate::timing::{DEFAULT_BREAK_MINUTES, DEFAULT_FOCUS_MINUTES};

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
/// Maximum number of blocked domains.
pub const MAX_BLOCKLIST: usize = 500;
/// Default chime volume.
pub const DEFAULT_CHIME_VOLUME: f32 = 0.7;
/// Default music volume.
pub const DEFAULT_MUSIC_VOLUME: f32 = 0.6;
/// Default Microsoft identity platform tenant for Teams (work and school accounts).
pub const DEFAULT_TEAMS_TENANT: &str = "organizations";

const MAX_TENANT_LEN: usize = 253;
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
        }
    }
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            source: MusicSourceKind::Stream,
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
            blocklist: DEFAULT_BLOCKLIST.iter().map(|d| (*d).to_owned()).collect(),
            keep_awake: true,
        }
    }
}

impl Config {
    /// Validates and normalizes a config received from the UI or read from disk.
    ///
    /// Trims strings, normalizes the blocklist (`normalize_domain`, dedupe, invalid entries
    /// dropped), trims links and drops empty ones (a bare `youtube.com/...` gets `https://`).
    /// Out-of-range values are never clamped silently: they return `Err` with code
    /// `config.invalid` and `field` set to the camelCase path (`focusMinutes`, `music.volume`).
    pub fn normalized(self) -> Result<Config, AppError> {
        let Config {
            focus_minutes,
            break_minutes,
            music,
            shields,
            chime_volume,
            track_apps,
            launch_on_login,
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
        })
    }
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
            blocklist,
            keep_awake,
        } = self;

        let teams_client_id = normalize_client_id(&teams_client_id)?;
        let teams_tenant = normalize_tenant(&teams_tenant)?;

        let mut seen = HashSet::new();
        let blocklist: Vec<String> = blocklist
            .iter()
            .filter_map(|entry| normalize_domain(entry.as_str()))
            .filter(|domain| seen.insert(domain.clone()))
            .collect();
        if blocklist.len() > MAX_BLOCKLIST {
            return Err(invalid(
                "shields.blocklist",
                format!(
                    "at most {MAX_BLOCKLIST} blocked domains are allowed, got {}",
                    blocklist.len()
                ),
            ));
        }

        Ok(ShieldsConfig {
            mute_notifications,
            teams_dnd,
            teams_client_id,
            teams_tenant,
            block_sites,
            blocklist,
            keep_awake,
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
        assert_eq!(music.source, MusicSourceKind::Stream);
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
        assert_eq!(shields.blocklist.len(), DEFAULT_BLOCKLIST.len());
        assert!(shields.blocklist.iter().any(|d| d == "reddit.com"));
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
        assert_eq!(partial.shields.blocklist.len(), DEFAULT_BLOCKLIST.len());
    }

    #[test]
    fn serializes_with_camel_case_keys() {
        let json = serde_json::to_value(Config::default()).expect("serialize");
        assert_eq!(json["focusMinutes"], 45);
        assert_eq!(json["breakMinutes"], 15);
        assert_eq!(json["trackApps"], true);
        assert_eq!(json["launchOnLogin"], false);
        assert_eq!(json["music"]["streamUrl"], LOFI_GIRL_URL);
        assert_eq!(json["music"]["source"], "stream");
        assert_eq!(json["music"]["noise"], "brown");
        assert_eq!(json["music"]["pauseDuringBreak"], false);
        assert_eq!(json["shields"]["teamsClientId"], "");
        assert_eq!(json["shields"]["teamsTenant"], "organizations");
        assert_eq!(json["shields"]["blockSites"], true);
        assert_eq!(json["shields"]["keepAwake"], true);
        assert_eq!(json["shields"]["muteNotifications"], false);
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
    fn normalizes_and_deduplicates_blocklist() {
        let cfg = with_shields(ShieldsConfig {
            blocklist: vec![
                "https://www.Reddit.com/r/x".to_owned(),
                "reddit.com".to_owned(),
                "   ".to_owned(),
                "reddit".to_owned(),
                "YouTube.com".to_owned(),
                "old.reddit.com".to_owned(),
            ],
            ..ShieldsConfig::default()
        });
        let normalized = cfg.normalized().expect("valid");
        assert_eq!(
            normalized.shields.blocklist,
            ["reddit.com", "youtube.com", "old.reddit.com"]
        );
    }

    #[test]
    fn rejects_too_many_blocked_domains() {
        let many = (0..=MAX_BLOCKLIST)
            .map(|i| format!("site{i}.com"))
            .collect();
        let cfg = with_shields(ShieldsConfig {
            blocklist: many,
            ..ShieldsConfig::default()
        });
        assert_eq!(
            field_of(cfg.normalized()).as_deref(),
            Some("shields.blocklist")
        );

        // Duplicates collapse before the limit is checked.
        let dupes = (0..=MAX_BLOCKLIST)
            .map(|_| "reddit.com".to_owned())
            .collect();
        let cfg = with_shields(ShieldsConfig {
            blocklist: dupes,
            ..ShieldsConfig::default()
        });
        assert_eq!(
            cfg.normalized().expect("valid").shields.blocklist,
            ["reddit.com"]
        );
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
                blocklist: vec!["WWW.9GAG.com/".to_owned(), "9gag.com".to_owned()],
                ..ShieldsConfig::default()
            },
            ..Config::default()
        };
        let once = messy.normalized().expect("valid");
        let twice = once.clone().normalized().expect("still valid");
        assert_eq!(once, twice);
        assert_eq!(once.shields.blocklist, ["9gag.com"]);
        assert_eq!(once.music.links[0], "https://youtu.be/abc");
    }
}
