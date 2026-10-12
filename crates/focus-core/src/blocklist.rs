//! Distracting-site blocklist: domain normalization, browser detection and the window-title
//! fallback matcher used when the browser extension is not connected.

/// Domains blocked during focus by default (meme / social / entertainment sites).
pub const DEFAULT_BLOCKLIST: &[&str] = &[
    "9gag.com",
    "reddit.com",
    "facebook.com",
    "instagram.com",
    "tiktok.com",
    "x.com",
    "twitter.com",
    "kwejk.pl",
    "jbzd.com.pl",
    "demotywatory.pl",
    "joemonster.org",
    "besty.pl",
    "memedroid.com",
    "imgur.com",
    "twitch.tv",
    "pudelek.pl",
];

/// Browser executable names without the `.exe` suffix (lowercase).
const BROWSER_EXECUTABLES: &[&str] = &[
    "chrome",
    "msedge",
    "brave",
    "opera",
    "vivaldi",
    "firefox",
    "arc",
    "yandex",
    "chromium",
    "librewolf",
    "waterfox",
    "thorium",
];

/// Second-level labels that sit under a two-letter country code (`jbzd.com.pl`, `bbc.co.uk`).
const SECOND_LEVEL_LABELS: &[&str] = &[
    "com", "co", "org", "net", "gov", "edu", "ac", "or", "ne", "go", "info", "biz",
];

/// Main labels shorter than this (`x` for `x.com`) only match a window title via the full domain.
const MIN_TITLE_LABEL_LEN: usize = 3;
/// Maximum length of a DNS name.
const MAX_DOMAIN_LEN: usize = 253;
/// Maximum length of one DNS label.
const MAX_LABEL_LEN: usize = 63;

/// Normalizes user input (a domain or a full URL) to a bare lowercase domain.
///
/// `"https://www.Reddit.com/r/x"` becomes `Some("reddit.com")`; `"reddit"` is `None` (needs a dot).
/// Strips the scheme, credentials, port, path, query, fragment, a leading `www.` / `*.` and a
/// trailing dot. Rejects whitespace, IPv6 literals and non-ASCII hosts (the browser extension
/// needs plain ASCII / punycode domains).
pub fn normalize_domain(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let lower = trimmed.to_lowercase();

    let after_scheme = match lower.split_once("://") {
        Some((scheme, rest)) if is_scheme(scheme) => rest,
        _ => lower.strip_prefix("//").unwrap_or(&lower),
    };
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = after_scheme.get(..authority_end)?;
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_credentials, host)| host);
    if host_port.starts_with('[') {
        return None;
    }
    let host = match host_port.split_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => host,
        Some(_) => return None,
        None => host_port,
    };

    let host = host.trim_end_matches('.');
    let host = host.strip_prefix("*.").unwrap_or(host);
    let host = host.trim_start_matches('.');
    let host = match host.strip_prefix("www.") {
        Some(rest) if rest.contains('.') => rest,
        _ => host,
    };

    is_valid_host(host).then(|| host.to_owned())
}

/// `true` for known browser executables (`chrome.exe`, `msedge.exe`, `firefox`, ...),
/// case-insensitive, with or without `.exe`, also when given a full path.
pub fn is_browser_process(process: &str) -> bool {
    let trimmed = process.trim();
    let file_name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    let lower = file_name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    BROWSER_EXECUTABLES.contains(&stem)
}

/// Fallback matcher for browser window titles.
///
/// For every blocklist domain (in order) the title matches when it contains the full domain
/// (`kwejk.pl`, not inside another domain such as `netflix.com` for `x.com`) or the domain's main
/// label as a whole word (`reddit` for `reddit.com`, `jbzd` for `jbzd.com.pl`). Labels shorter
/// than 3 characters only match via the full domain. Case-insensitive. Returns the matched
/// domain (lowercase).
pub fn title_matches(title: &str, blocklist: &[String]) -> Option<String> {
    let title = title.to_lowercase();
    if title.trim().is_empty() {
        return None;
    }
    blocklist.iter().find_map(|entry| {
        let domain = entry.trim().to_lowercase();
        if domain.is_empty() {
            return None;
        }
        if contains_bounded(&title, &domain, domain_boundary) {
            return Some(domain);
        }
        let label = main_label(&domain)?;
        let long_enough = label.chars().count() >= MIN_TITLE_LABEL_LEN;
        let matched = long_enough && contains_bounded(&title, label, word_boundary);
        matched.then_some(domain)
    })
}

/// `true` when `host` is `domain` or one of its subdomains (`old.reddit.com` for `reddit.com`).
/// Both sides are compared case-insensitively; a trailing dot on `host` is ignored.
pub fn host_matches(host: &str, domain: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let domain = domain.trim().to_ascii_lowercase();
    if domain.is_empty() {
        return false;
    }
    host == domain
        || host
            .strip_suffix(domain.as_str())
            .is_some_and(|prefix| prefix.ends_with('.'))
}

fn is_scheme(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn is_valid_host(host: &str) -> bool {
    host.len() <= MAX_DOMAIN_LEN && host.contains('.') && host.split('.').all(is_valid_label)
}

fn is_valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= MAX_LABEL_LEN
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The registrable name of a domain: `reddit` for `old.reddit.com`, `jbzd` for `jbzd.com.pl`.
fn main_label(domain: &str) -> Option<&str> {
    let mut labels = domain.rsplit('.');
    let top = labels.next()?;
    let second = labels.next()?;
    match labels.next() {
        Some(third) if top.len() == 2 && SECOND_LEVEL_LABELS.contains(&second) => Some(third),
        _ => Some(second),
    }
}

/// `true` when `needle` occurs in `haystack` with acceptable surroundings.
fn contains_bounded(haystack: &str, needle: &str, boundary: fn(&str, &str) -> bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    haystack.match_indices(needle).any(|(start, matched)| {
        let end = start.saturating_add(matched.len());
        match (haystack.get(..start), haystack.get(end..)) {
            (Some(before), Some(after)) => boundary(before, after),
            _ => false,
        }
    })
}

/// A full domain must not be glued to other host characters (`netflix.com` is not `x.com`);
/// a preceding dot is fine (subdomain).
fn domain_boundary(before: &str, after: &str) -> bool {
    let prev_ok = before.chars().next_back().is_none_or(|c| !is_host_char(c));
    let mut rest = after.chars();
    let next_ok = match rest.next() {
        None => true,
        Some('.') => rest.next().is_none_or(|c| !c.is_alphanumeric()),
        Some(c) => !is_host_char(c),
    };
    prev_ok && next_ok
}

/// A label must be a whole word (`reddit`, not `redditor`).
fn word_boundary(before: &str, after: &str) -> bool {
    before
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric())
        && after.chars().next().is_none_or(|c| !c.is_alphanumeric())
}

fn is_host_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Vec<String> {
        DEFAULT_BLOCKLIST.iter().map(|d| (*d).to_owned()).collect()
    }

    #[test]
    fn normalizes_urls_and_domains() {
        let cases = [
            ("https://www.Reddit.com/r/x", Some("reddit.com")),
            ("reddit.com", Some("reddit.com")),
            ("  REDDIT.COM  ", Some("reddit.com")),
            (
                "http://user:pass@www.example.com:8080/path?q=1#top",
                Some("example.com"),
            ),
            ("old.reddit.com/r/rust", Some("old.reddit.com")),
            ("9gag.com.", Some("9gag.com")),
            ("*.facebook.com", Some("facebook.com")),
            (".instagram.com", Some("instagram.com")),
            ("//imgur.com/gallery", Some("imgur.com")),
            ("jbzd.com.pl", Some("jbzd.com.pl")),
            ("www.com", Some("www.com")),
            ("example.com?next=http://other.org", Some("example.com")),
            ("chrome-extension://abc.def/page", Some("abc.def")),
            ("reddit", None),
            ("", None),
            ("   ", None),
            ("red dit.com", None),
            ("reddit.com:abc", None),
            ("[::1]:8080", None),
            ("http://[::1]/", None),
            ("-bad.com", None),
            ("bad-.com", None),
            ("a..b.com", None),
            ("żółw.pl", None),
            ("under_score.com", None),
            ("https://", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                normalize_domain(input).as_deref(),
                expected,
                "input {input:?}"
            );
        }
    }

    #[test]
    fn rejects_overlong_labels_and_domains() {
        let long_label = format!("{}.com", "a".repeat(64));
        assert_eq!(normalize_domain(&long_label), None);
        let ok_label = format!("{}.com", "a".repeat(63));
        assert_eq!(
            normalize_domain(&ok_label).as_deref(),
            Some(ok_label.as_str())
        );
        let long_domain = format!("{}com", "abcdefghi.".repeat(26));
        assert!(long_domain.len() > 253);
        assert_eq!(normalize_domain(&long_domain), None);
    }

    #[test]
    fn default_blocklist_is_already_normalized_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for domain in DEFAULT_BLOCKLIST {
            assert_eq!(normalize_domain(domain).as_deref(), Some(*domain));
            assert!(seen.insert(*domain), "duplicate {domain}");
        }
    }

    #[test]
    fn detects_browser_processes() {
        for name in [
            "chrome.exe",
            "Chrome.EXE",
            "msedge.exe",
            "brave.exe",
            "opera.exe",
            "vivaldi.exe",
            "firefox.exe",
            "firefox",
            "arc.exe",
            "yandex.exe",
            "chromium.exe",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            "/usr/bin/firefox",
        ] {
            assert!(is_browser_process(name), "{name}");
        }
        for name in [
            "Code.exe",
            "explorer.exe",
            "chrome_helper.exe",
            "",
            "exe",
            "slack.exe",
        ] {
            assert!(!is_browser_process(name), "{name}");
        }
    }

    #[test]
    fn title_matches_main_label_as_whole_word() {
        let list = defaults();
        assert_eq!(
            title_matches("r/rust - Reddit - Google Chrome", &list).as_deref(),
            Some("reddit.com")
        );
        assert_eq!(
            title_matches("9GAG: Go Fun The World - Opera", &list).as_deref(),
            Some("9gag.com")
        );
        assert_eq!(
            title_matches("JBZD - najlepsze memy - Brave", &list).as_deref(),
            Some("jbzd.com.pl")
        );
        assert_eq!(
            title_matches("(3) Facebook – Microsoft Edge", &list).as_deref(),
            Some("facebook.com")
        );
        assert_eq!(
            title_matches("Redditor stories - Google Chrome", &list),
            None
        );
    }

    #[test]
    fn title_matches_full_domain() {
        let list = defaults();
        assert_eq!(
            title_matches("Kwejk.pl – Opera", &list).as_deref(),
            Some("kwejk.pl")
        );
        assert_eq!(
            title_matches("Post on x.com - Google Chrome", &list).as_deref(),
            Some("x.com")
        );
        assert_eq!(
            title_matches("mobile.x.com - Firefox", &list).as_deref(),
            Some("x.com")
        );
    }

    #[test]
    fn short_labels_need_the_full_domain() {
        let list = defaults();
        assert_eq!(title_matches("Xcode", &list), None);
        assert_eq!(title_matches("Home / X", &list), None);
        assert_eq!(title_matches("netflix.com - Chrome", &list), None);
        assert_eq!(title_matches("x.company.org", &list), None);
    }

    #[test]
    fn title_matching_handles_edge_cases() {
        let list = defaults();
        assert_eq!(title_matches("", &list), None);
        assert_eq!(title_matches("   ", &list), None);
        assert_eq!(title_matches("Visual Studio Code", &list), None);
        assert_eq!(title_matches("reddit", &[]), None);
        let custom = ["".to_owned(), "  News.YCombinator.com ".to_owned()];
        assert_eq!(
            title_matches("Hacker News | ycombinator", &custom).as_deref(),
            Some("news.ycombinator.com")
        );
        // Multi-byte characters around a match never break slicing.
        assert_eq!(
            title_matches("Zażółć reddit gęślą jaźń", &list).as_deref(),
            Some("reddit.com")
        );
        assert_eq!(title_matches("Zażółćreddit", &list), None);
    }

    #[test]
    fn main_label_skips_country_second_levels() {
        assert_eq!(main_label("reddit.com"), Some("reddit"));
        assert_eq!(main_label("old.reddit.com"), Some("reddit"));
        assert_eq!(main_label("jbzd.com.pl"), Some("jbzd"));
        assert_eq!(main_label("bbc.co.uk"), Some("bbc"));
        assert_eq!(main_label("twitch.tv"), Some("twitch"));
        assert_eq!(main_label("com"), None);
    }

    #[test]
    fn host_matches_domain_and_subdomains() {
        assert!(host_matches("reddit.com", "reddit.com"));
        assert!(host_matches("old.reddit.com", "reddit.com"));
        assert!(host_matches("WWW.Reddit.com.", "reddit.com"));
        assert!(!host_matches("notreddit.com", "reddit.com"));
        assert!(!host_matches("reddit.com.evil.org", "reddit.com"));
        assert!(!host_matches("reddit.com", ""));
    }
}
