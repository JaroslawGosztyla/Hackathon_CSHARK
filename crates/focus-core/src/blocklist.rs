//! Distracting-site blocklist: the `websiteblacklist.txt` format (domains and regexes), domain
//! normalization, browser detection, URL matching and the window-title fallback matcher used
//! when the browser extension is not connected.
//!
//! The file itself is read and written by the shell (`persistence.rs`); this module only parses
//! its text ([`Blocklist::parse`]).

use std::collections::HashSet;

use regex::{Regex, RegexBuilder};

use crate::api::BlocklistIssue;

/// File name of the user-editable blocklist in the app config directory.
pub const BLACKLIST_FILE_NAME: &str = "websiteblacklist.txt";
/// Bundled default `websiteblacklist.txt`, written to the config directory on first run.
pub const DEFAULT_BLACKLIST: &str = include_str!("../assets/websiteblacklist.txt");
/// Maximum number of (unique, valid) domains; later ones are skipped with one issue.
pub const MAX_DOMAINS: usize = 2000;
/// Maximum number of (unique, valid) regexes; later ones are skipped with one issue.
pub const MAX_PATTERNS: usize = 500;
/// Longer lines (in characters, after trimming) are skipped.
pub const MAX_LINE_CHARS: usize = 500;
/// At most this many issues are reported; further problem lines are still skipped.
pub const MAX_ISSUES: usize = 100;

/// `BlocklistIssue.text` is shortened to this many characters.
const MAX_ISSUE_TEXT_CHARS: usize = 200;
/// Compiled program and lazy DFA cache limit of one regex.
const REGEX_SIZE_LIMIT: usize = 256 * 1024;
/// Regex error reasons are shortened to this many characters.
const MAX_REASON_CHARS: usize = 120;
/// A line containing any of these characters is a regex (unless it is wrapped in `/…/`).
const REGEX_METACHARS: &[char] = &[
    '\\', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|',
];
/// Regex characters that never occur in a pasted address (`?`, `+`, `*`, `$` can).
const STRUCTURAL_REGEX_CHARS: &[char] = &['\\', '^', '(', ')', '[', ']', '{', '}', '|'];
/// Two unrelated addresses: a regex matching both of them matches practically every address.
const MATCH_ALL_PROBES: [&str; 2] = ["https://example.com/", "http://10.0.0.1:8080/a?b=c#d"];

const MSG_INVALID_DOMAIN: &str = "Nieprawidłowa domena";
const MSG_INVALID_REGEX: &str = "Nieprawidłowe wyrażenie regularne";
const MSG_TOO_COMPLEX: &str = "Wyrażenie regularne jest zbyt złożone";
const MSG_MATCHES_ALL: &str = "Wyrażenie pasuje do każdego adresu";

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

/// Main labels that are common words in Polish or English window titles (`temu` = "ago",
/// "third-party", "people", "Max"). They match a title only via the full domain (`temu.com`),
/// otherwise the fallback nudge would fire on unrelated pages (`viva` is also Microsoft Viva, a work
/// tool; `ppe` a common acronym). `onet` comes from the `przegladsportowy.onet.pl` entry, which must
/// not flag every Onet page.
const AMBIGUOUS_TITLE_LABELS: &[&str] = &[
    "temu",
    "people",
    "player",
    "party",
    "max",
    "kick",
    "threads",
    "sport",
    "polygon",
    "onet",
    "gram",
    "viva",
    "ppe",
    "chess",
    "mistrzowie",
];

/// Main labels shorter than this (`x` for `x.com`) only match a window title via the full domain.
const MIN_TITLE_LABEL_LEN: usize = 3;
/// Maximum length of a DNS name.
const MAX_DOMAIN_LEN: usize = 253;
/// Maximum length of one DNS label.
const MAX_LABEL_LEN: usize = 63;

/// Parsed `websiteblacklist.txt`: normalized domains (each also blocks its subdomains) and
/// case-insensitive regexes matched against the full URL (or the window title in the fallback).
#[derive(Debug, Clone, Default)]
pub struct Blocklist {
    domains: Vec<String>,
    /// `(source as written, compiled case-insensitive regex)`.
    patterns: Vec<(String, Regex)>,
}

impl PartialEq for Blocklist {
    fn eq(&self, other: &Self) -> bool {
        self.domains == other.domains
            && self.patterns.len() == other.patterns.len()
            && self
                .patterns
                .iter()
                .zip(&other.patterns)
                .all(|((a, _), (b, _))| a == b)
    }
}

impl Eq for Blocklist {}

/// One meaningful line of the file.
enum Entry<'a> {
    Domain(&'a str),
    Pattern(&'a str),
}

impl Blocklist {
    /// Parses the text of `websiteblacklist.txt`.
    ///
    /// One entry per line; blank lines and lines starting with `#` are skipped, as is a trailing
    /// ` # comment` (whitespace before `#`). A UTF-8 BOM and `\r\n` line endings are fine.
    /// A line wrapped in `/…/` (at least 3 characters) is a regex (the inner text); a line
    /// containing any of `\ ^ $ * + ? ( ) [ ] { } |` is a regex as written; anything else is a
    /// domain or URL reduced by [`normalize_domain`]. A pasted address with `?` that would be a regex
    /// not even matching itself is reported instead. Entries keep file order without duplicates
    /// (domains compared after normalization, regexes by source). Invalid lines are skipped and
    /// reported as issues with a 1-based line number and a Polish message (at most
    /// [`MAX_ISSUES`]).
    pub fn parse(text: &str) -> (Blocklist, Vec<BlocklistIssue>) {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut list = Blocklist::default();
        let mut issues = Vec::new();
        let mut seen_domains = HashSet::new();
        let mut seen_patterns = HashSet::new();
        let mut domain_limit_reported = false;
        let mut pattern_limit_reported = false;

        for (index, raw) in text.lines().enumerate() {
            let line_no = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
            let written = raw.trim();
            if written.is_empty() || written.starts_with('#') {
                continue;
            }
            let mut report = |message: String| push_issue(&mut issues, line_no, written, message);

            if written.chars().count() > MAX_LINE_CHARS {
                report(format!(
                    "Linia jest za długa (limit {MAX_LINE_CHARS} znaków)"
                ));
                continue;
            }

            let entry = strip_inline_comment(written);
            if let Some(domain) = pasted_url_domain(entry) {
                report(format!(
                    "Adres ze znakiem ? to wyrażenie regularne, które nie pasuje do tego adresu — żeby zablokować całą stronę, wpisz {domain}; żeby tylko ten adres, zamień ? na \\?"
                ));
                continue;
            }

            match classify(entry) {
                Entry::Domain(input) => {
                    let Some(domain) = normalize_domain(input) else {
                        report(MSG_INVALID_DOMAIN.to_owned());
                        continue;
                    };
                    if seen_domains.contains(&domain) {
                        continue;
                    }
                    if list.domains.len() >= MAX_DOMAINS {
                        if !domain_limit_reported {
                            domain_limit_reported = true;
                            report(format!("Za dużo domen (limit {MAX_DOMAINS})"));
                        }
                        continue;
                    }
                    seen_domains.insert(domain.clone());
                    list.domains.push(domain);
                }
                Entry::Pattern(source) => {
                    if seen_patterns.contains(source) {
                        continue;
                    }
                    if list.patterns.len() >= MAX_PATTERNS {
                        if !pattern_limit_reported {
                            pattern_limit_reported = true;
                            report(format!(
                                "Za dużo wyrażeń regularnych (limit {MAX_PATTERNS})"
                            ));
                        }
                        continue;
                    }
                    match compile_pattern(source) {
                        Ok(regex) => {
                            seen_patterns.insert(source.to_owned());
                            list.patterns.push((source.to_owned(), regex));
                        }
                        Err(message) => report(message),
                    }
                }
            }
        }
        (list, issues)
    }

    /// Normalized domains in file order.
    pub fn domains(&self) -> &[String] {
        &self.domains
    }

    /// Regex sources (as written, without the `/…/` wrapper) in file order.
    pub fn pattern_sources(&self) -> Vec<String> {
        self.patterns
            .iter()
            .map(|(source, _)| source.clone())
            .collect()
    }

    /// No domains and no regexes.
    pub fn is_empty(&self) -> bool {
        self.domains.is_empty() && self.patterns.is_empty()
    }

    /// The entry that blocks `url`: the first domain (file order) whose host is the URL's host or
    /// one of its parents, else the source of the first regex matching the full URL
    /// (case-insensitive). Domains win over regexes regardless of file order.
    pub fn match_url(&self, url: &str) -> Option<String> {
        if let Some(host) = url_host(url)
            && let Some(domain) = self.domains.iter().find(|d| host_matches(&host, d))
        {
            return Some(domain.clone());
        }
        self.patterns
            .iter()
            .find(|(_, regex)| regex.is_match(url))
            .map(|(source, _)| source.clone())
    }

    /// Fallback matcher for browser window titles (no URL available).
    ///
    /// For every domain (in order) the title matches when it contains the full domain
    /// (`kwejk.pl`, not inside another domain such as `netflix.com` for `x.com`) or the domain's
    /// main label as a whole word (`reddit` for `reddit.com`, `jbzd` for `jbzd.com.pl`). Labels
    /// shorter than 3 characters and common words (`temu`, `party`, ...) only match via the full
    /// domain. Case-insensitive. Returns the matched domain; when no domain matches, the source of
    /// the first regex matching the title.
    pub fn match_title(&self, title: &str) -> Option<String> {
        if title.trim().is_empty() {
            return None;
        }
        let lower = title.to_lowercase();
        self.domains
            .iter()
            .find(|domain| title_matches_domain(&lower, domain))
            .cloned()
            .or_else(|| {
                self.patterns
                    .iter()
                    .find(|(_, regex)| regex.is_match(title))
                    .map(|(source, _)| source.clone())
            })
    }
}

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

/// `/…/` (at least 3 characters) or a line with regex characters is a regex; anything else a domain.
fn classify(line: &str) -> Entry<'_> {
    let wrapped = line.len() >= 3 && line.starts_with('/') && line.ends_with('/');
    if wrapped && let Some(inner) = line.get(1..line.len().saturating_sub(1)) {
        return Entry::Pattern(inner);
    }
    if line.contains(REGEX_METACHARS) {
        Entry::Pattern(line)
    } else {
        Entry::Domain(line)
    }
}

/// A pasted address with a query string (`https://www.youtube.com/watch?v=x`): the `?` makes the line a
/// regex, and as a regex it does not even match that address (`h?` is an optional `h`). Returns the
/// address's domain for the hint. Lines in `/…/`, with other regex syntax, or whose regex does match
/// their own text (`reddit.com/r/polska/?`) are left alone.
fn pasted_url_domain(line: &str) -> Option<String> {
    if !line.contains('?') || line.starts_with('/') || line.contains(STRUCTURAL_REGEX_CHARS) {
        return None;
    }
    let domain = normalize_domain(line)?;
    let regex = RegexBuilder::new(line)
        .case_insensitive(true)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_SIZE_LIMIT)
        .build()
        .ok()?;
    (!regex.is_match(line)).then_some(domain)
}

/// Cuts a trailing comment (`kwejk.pl   # memy`): a `#` preceded by whitespace.
fn strip_inline_comment(line: &str) -> &str {
    let mut previous_is_space = false;
    for (index, c) in line.char_indices() {
        if c == '#' && previous_is_space {
            return line.get(..index).map_or(line, str::trim_end);
        }
        previous_is_space = c.is_whitespace();
    }
    line
}

/// Compiles one regex source or returns the Polish issue message.
fn compile_pattern(source: &str) -> Result<Regex, String> {
    let regex = RegexBuilder::new(source)
        .case_insensitive(true)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map_err(|err| match err {
            regex::Error::CompiledTooBig(_) => MSG_TOO_COMPLEX.to_owned(),
            regex::Error::Syntax(detail) => {
                let mut message = format!("{MSG_INVALID_REGEX}: {}", syntax_reason(&detail));
                if let Some(domain) = source
                    .strip_prefix('*')
                    .and_then(|rest| normalize_domain(rest.trim_end_matches('*')))
                {
                    message.push_str(&format!(
                        " — żeby zablokować domenę z subdomenami, wpisz po prostu {domain}"
                    ));
                }
                message
            }
            other => format!(
                "{MSG_INVALID_REGEX}: {}",
                shorten(&other.to_string(), MAX_REASON_CHARS)
            ),
        })?;
    let matches_all =
        regex.is_match("") || MATCH_ALL_PROBES.iter().all(|probe| regex.is_match(probe));
    if matches_all {
        return Err(MSG_MATCHES_ALL.to_owned());
    }
    Ok(regex)
}

/// Short reason from a `regex-syntax` error (the `error: …` line), in Polish when known.
fn syntax_reason(detail: &str) -> String {
    let reason = detail
        .lines()
        .find_map(|line| line.trim().strip_prefix("error:"))
        .or_else(|| detail.lines().rev().find(|line| !line.trim().is_empty()))
        .unwrap_or(detail)
        .trim();
    let known: &[(&str, &str)] = &[
        ("unclosed group", "brak zamykającego nawiasu )"),
        ("unopened group", "nadmiarowy nawias )"),
        (
            "repetition operator missing expression",
            "*, + albo ? bez wyrażenia przed nim",
        ),
        ("unclosed character class", "brak zamykającego nawiasu ]"),
        ("incomplete escape sequence", "niedokończona sekwencja \\"),
        ("unrecognized escape sequence", "nieznana sekwencja \\"),
        ("look-around", "lookahead i lookbehind nie są obsługiwane"),
        ("backreferences", "odwołania wsteczne nie są obsługiwane"),
        (
            "invalid character class range",
            "nieprawidłowy zakres w [ ]",
        ),
        ("unclosed counted repetition", "brak zamykającego nawiasu }"),
    ];
    known
        .iter()
        .find(|(english, _)| reason.contains(english))
        .map_or_else(
            || shorten(reason, MAX_REASON_CHARS),
            |(_, polish)| (*polish).to_owned(),
        )
}

fn push_issue(issues: &mut Vec<BlocklistIssue>, line: u32, text: &str, message: String) {
    if issues.len() < MAX_ISSUES {
        issues.push(BlocklistIssue {
            line,
            text: shorten(text, MAX_ISSUE_TEXT_CHARS),
            message,
        });
    }
}

/// At most `max` characters; a shortened text ends with `…`.
fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(max.saturating_sub(1)).collect();
    short.push('…');
    short
}

/// Host of a URL (lowercase, without credentials, port, trailing dot and a leading `www.`).
fn url_host(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let after_scheme = match trimmed.split_once("://") {
        Some((scheme, rest)) if is_scheme(scheme) => rest,
        _ => trimmed.strip_prefix("//").unwrap_or(trimmed),
    };
    let authority_end = after_scheme
        .find(['/', '?', '#', '\\'])
        .unwrap_or(after_scheme.len());
    let authority = after_scheme.get(..authority_end)?;
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_credentials, host)| host);
    if host_port.starts_with('[') {
        return None;
    }
    let host = host_port
        .split_once(':')
        .map_or(host_port, |(host, _)| host);
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let host = match host.strip_prefix("www.") {
        Some(rest) if rest.contains('.') => rest.to_owned(),
        _ => host,
    };
    (!host.is_empty()).then_some(host)
}

/// Title fallback for one domain; `title` is already lowercase.
fn title_matches_domain(title: &str, domain: &str) -> bool {
    if domain.is_empty() {
        return false;
    }
    if contains_bounded(title, domain, domain_boundary) {
        return true;
    }
    main_label(domain).is_some_and(|label| {
        label.chars().count() >= MIN_TITLE_LABEL_LEN
            && !AMBIGUOUS_TITLE_LABELS.contains(&label)
            && contains_bounded(title, label, word_boundary)
    })
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

    const PORTAL_REGEX: &str = r"^https?://(www\.)?(onet|wp|interia|o2|gazeta)\.pl/?(\?.*)?$";

    fn defaults() -> Blocklist {
        let (list, issues) = Blocklist::parse(DEFAULT_BLACKLIST);
        assert!(issues.is_empty(), "{issues:?}");
        list
    }

    fn parse_ok(text: &str) -> Blocklist {
        let (list, issues) = Blocklist::parse(text);
        assert!(issues.is_empty(), "{issues:?}");
        list
    }

    fn messages(text: &str) -> Vec<(u32, String)> {
        Blocklist::parse(text)
            .1
            .into_iter()
            .map(|issue| (issue.line, issue.message))
            .collect()
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
    fn bundled_default_parses_without_issues() {
        let list = defaults();
        for domain in [
            "kwejk.pl",
            "jbzd.com.pl",
            "reddit.com",
            "facebook.com",
            "9gag.com",
            "x.com",
            "sportowefakty.wp.pl",
        ] {
            assert!(list.domains().iter().any(|d| d == domain), "{domain}");
        }
        let patterns = list.pattern_sources();
        for pattern in [
            r"linkedin\.com/feed",
            PORTAL_REGEX,
            r"bbc\.co\.uk/sport",
            r"youtube\.com/shorts",
        ] {
            assert!(patterns.iter().any(|p| p == pattern), "{pattern}");
        }
        // Commented-out entries stay off.
        for off in ["youtube.com", "onet.pl", "amazon.com", "discord.com"] {
            assert!(!list.domains().iter().any(|d| d == off), "{off}");
        }
        assert!(list.domains().len() <= MAX_DOMAINS);
        assert!(!list.is_empty());
    }

    #[test]
    fn bundled_default_domains_are_normalized_and_unique() {
        let list = defaults();
        let mut seen = HashSet::new();
        for domain in list.domains() {
            assert_eq!(normalize_domain(domain).as_deref(), Some(domain.as_str()));
            assert!(seen.insert(domain), "duplicate {domain}");
        }
        assert_eq!(BLACKLIST_FILE_NAME, "websiteblacklist.txt");
    }

    #[test]
    fn detects_regex_lines() {
        let text = "\
/reddit\\.com/r/polska/
youtube\\.com/shorts
^https?://(www\\.)?onet\\.pl/?$
foo\\.com/(a|b)
/x.com/
kwejk.pl
https://www.Reddit.com/r/polska
youtube.com/shorts
";
        let parsed = parse_ok(text);
        assert_eq!(
            parsed.pattern_sources(),
            [
                r"reddit\.com/r/polska",
                r"youtube\.com/shorts",
                r"^https?://(www\.)?onet\.pl/?$",
                r"foo\.com/(a|b)",
                "x.com",
            ]
        );
        // Plain lines are domains, also full URLs and paths (only the host counts).
        assert_eq!(parsed.domains(), ["kwejk.pl", "reddit.com", "youtube.com"]);
    }

    #[test]
    fn short_slash_lines_are_not_regexes() {
        let issues = messages("//\n/\n");
        assert_eq!(
            issues,
            [
                (1, MSG_INVALID_DOMAIN.to_owned()),
                (2, MSG_INVALID_DOMAIN.to_owned()),
            ]
        );
    }

    #[test]
    fn skips_comments_bom_and_crlf() {
        let text = "\u{feff}kwejk.pl\r\n# comment\r\n\r\n   # indented comment\r\n  jbzd.com.pl  \r\nreddit.com   # memes\r\n\t\r\n/youtube\\.com/shorts/ # shorts\r\n";
        let parsed = parse_ok(text);
        assert_eq!(parsed.domains(), ["kwejk.pl", "jbzd.com.pl", "reddit.com"]);
        assert_eq!(parsed.pattern_sources(), [r"youtube\.com/shorts"]);

        let (empty, issues) = Blocklist::parse("\u{feff}\r\n# only comments\n\n");
        assert!(empty.is_empty());
        assert!(issues.is_empty());
        // A `#` without whitespace before it is part of the entry.
        assert_eq!(
            parse_ok(r"example\.com/#/feed").pattern_sources(),
            [r"example\.com/#/feed"]
        );
    }

    #[test]
    fn deduplicates_in_file_order() {
        let text = "\
reddit.com
kwejk.pl
https://www.reddit.com/r/x
REDDIT.COM
/youtube\\.com/shorts/
youtube\\.com/shorts
linkedin\\.com/feed
";
        let parsed = parse_ok(text);
        assert_eq!(parsed.domains(), ["reddit.com", "kwejk.pl"]);
        assert_eq!(
            parsed.pattern_sources(),
            [r"youtube\.com/shorts", r"linkedin\.com/feed"]
        );
    }

    #[test]
    fn reports_invalid_domains_with_line_numbers() {
        let issues = Blocklist::parse("kwejk.pl\nreddit\n\n  bad domain.com  \nżółw.pl\n").1;
        let summary: Vec<_> = issues
            .iter()
            .map(|i| (i.line, i.text.as_str(), i.message.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (2, "reddit", MSG_INVALID_DOMAIN),
                (4, "bad domain.com", MSG_INVALID_DOMAIN),
                (5, "żółw.pl", MSG_INVALID_DOMAIN),
            ]
        );
    }

    #[test]
    fn rejects_invalid_and_too_complex_regexes() {
        let (parsed, issues) =
            Blocklist::parse("(reddit\n[abc\nfoo(?=bar)\n*.facebook.com\nok\\.com/x\n");
        assert_eq!(parsed.pattern_sources(), [r"ok\.com/x"]);
        let lines: Vec<u32> = issues.iter().map(|i| i.line).collect();
        assert_eq!(lines, [1, 2, 3, 4]);
        for issue in &issues {
            assert!(
                issue
                    .message
                    .starts_with("Nieprawidłowe wyrażenie regularne: "),
                "{issue:?}"
            );
        }
        assert!(issues[0].message.contains("nawiasu )"), "{:?}", issues[0]);
        assert!(issues[2].message.contains("lookahead"), "{:?}", issues[2]);
        assert!(
            issues[3].message.contains("facebook.com"),
            "{:?}",
            issues[3]
        );

        let complex = messages(r"\w{200}\w{200}\w{200}");
        assert_eq!(complex, [(1, MSG_TOO_COMPLEX.to_owned())]);
    }

    #[test]
    fn rejects_regexes_matching_every_address() {
        for source in [
            ".*",
            "a*",
            "(foo)?",
            "^",
            "$",
            "/.*/",
            "reddit|",
            "/./",
            r"\w",
            "https?://",
            "/:/",
        ] {
            assert_eq!(
                messages(source),
                [(1, MSG_MATCHES_ALL.to_owned())],
                "{source}"
            );
        }
        // Broad but not universal regexes are allowed.
        for source in [r"\.ru/", "^http://", r"reddit\.com/r/"] {
            assert!(messages(source).is_empty(), "{source}");
        }
    }

    #[test]
    fn enforces_limits() {
        let domains: String = (0..MAX_DOMAINS + 5)
            .map(|i| format!("site{i}.com\n"))
            .chain(std::iter::once("site0.com\n".to_owned()))
            .collect();
        let (parsed, issues) = Blocklist::parse(&domains);
        assert_eq!(parsed.domains().len(), MAX_DOMAINS);
        let expected_line = u32::try_from(MAX_DOMAINS + 1).expect("small");
        assert_eq!(
            messages(&domains),
            [(expected_line, "Za dużo domen (limit 2000)".to_owned())]
        );
        assert_eq!(issues.len(), 1);

        let patterns: String = (0..MAX_PATTERNS + 3)
            .map(|i| format!("site{i}\\.com/feed\n"))
            .collect();
        let (parsed, issues) = Blocklist::parse(&patterns);
        assert_eq!(parsed.pattern_sources().len(), MAX_PATTERNS);
        let expected_line = u32::try_from(MAX_PATTERNS + 1).expect("small");
        assert_eq!(
            issues
                .iter()
                .map(|i| (i.line, i.message.as_str()))
                .collect::<Vec<_>>(),
            [(expected_line, "Za dużo wyrażeń regularnych (limit 500)")]
        );
    }

    #[test]
    fn rejects_overlong_lines_and_caps_issues() {
        let exact = format!(r"x\.com/{}", "a".repeat(MAX_LINE_CHARS - 7));
        assert_eq!(exact.chars().count(), MAX_LINE_CHARS);
        assert_eq!(parse_ok(&exact).pattern_sources().len(), 1);

        let long = format!("{exact}a");
        let issues = Blocklist::parse(&long).1;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].message, "Linia jest za długa (limit 500 znaków)");
        assert_eq!(issues[0].text.chars().count(), MAX_ISSUE_TEXT_CHARS);
        assert!(issues[0].text.ends_with('…'));

        let garbage = "bad\n".repeat(MAX_ISSUES + 50);
        assert_eq!(Blocklist::parse(&garbage).1.len(), MAX_ISSUES);
    }

    #[test]
    fn matches_urls_by_domain() {
        let list = defaults();
        for (url, expected) in [
            ("https://reddit.com/", Some("reddit.com")),
            ("https://old.reddit.com/r/rust", Some("reddit.com")),
            ("https://www.kwejk.pl/", Some("kwejk.pl")),
            ("http://kwejk.pl:8080/obrazek", Some("kwejk.pl")),
            ("https://user:pw@m.facebook.com/x", Some("facebook.com")),
            ("HTTPS://WWW.JBZD.COM.PL/", Some("jbzd.com.pl")),
            ("https://reddit.com./", Some("reddit.com")),
            (
                "https://sportowefakty.wp.pl/pilka",
                Some("sportowefakty.wp.pl"),
            ),
            ("reddit.com/r/polska", Some("reddit.com")),
            ("https://notreddit.com/", None),
            ("https://reddit.com.evil.org/", None),
            ("https://evil.org/?u=https://reddit.com/", None),
            ("https://github.com/", None),
            ("", None),
        ] {
            assert_eq!(list.match_url(url).as_deref(), expected, "{url}");
        }
        let x_only = parse_ok("x.com");
        assert_eq!(x_only.match_url("https://netflix.com/").as_deref(), None);
        assert_eq!(
            x_only.match_url("https://x.com/home").as_deref(),
            Some("x.com")
        );
    }

    #[test]
    fn matches_urls_by_regex() {
        let list = defaults();
        let shorts = Some(r"youtube\.com/shorts");
        assert_eq!(
            list.match_url("https://www.youtube.com/shorts/x")
                .as_deref(),
            shorts
        );
        assert_eq!(
            list.match_url("HTTPS://WWW.YOUTUBE.COM/SHORTS/X")
                .as_deref(),
            shorts
        );
        assert_eq!(list.match_url("https://www.youtube.com/watch?v=x"), None);

        for url in [
            "https://www.onet.pl/",
            "https://wp.pl",
            "https://www.interia.pl/?utm=1",
            "http://gazeta.pl/",
        ] {
            assert_eq!(list.match_url(url).as_deref(), Some(PORTAL_REGEX), "{url}");
        }
        for url in [
            "https://poczta.wp.pl/",
            "https://www.onet.pl/informacje/x",
            "https://poczta.interia.pl/",
        ] {
            assert_eq!(list.match_url(url), None, "{url}");
        }
        assert_eq!(
            list.match_url("https://www.linkedin.com/feed/").as_deref(),
            Some(r"linkedin\.com/feed")
        );
        assert_eq!(list.match_url("https://www.linkedin.com/in/someone"), None);
    }

    #[test]
    fn domains_win_over_earlier_regexes() {
        let list = parse_ok("/reddit/\nreddit.com\n");
        assert_eq!(
            list.match_url("https://reddit.com/r/x").as_deref(),
            Some("reddit.com")
        );
        assert_eq!(
            list.match_url("https://www.google.com/search?q=reddit")
                .as_deref(),
            Some("reddit")
        );
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
        for (title, expected) in [
            ("r/polska - Reddit - Google Chrome", Some("reddit.com")),
            ("r/rust - Reddit - Google Chrome", Some("reddit.com")),
            ("9GAG: Go Fun The World - Opera", Some("9gag.com")),
            ("JBZD - najlepsze memy - Brave", Some("jbzd.com.pl")),
            ("(3) Facebook – Microsoft Edge", Some("facebook.com")),
            ("Redditor stories - Google Chrome", None),
        ] {
            assert_eq!(list.match_title(title).as_deref(), expected, "{title}");
        }
    }

    #[test]
    fn title_matches_full_domain() {
        let list = defaults();
        for (title, expected) in [
            ("Kwejk.pl - Opera", "kwejk.pl"),
            ("Kwejk.pl – Opera", "kwejk.pl"),
            ("Post on x.com - Google Chrome", "x.com"),
            ("mobile.x.com - Firefox", "x.com"),
            ("Promocje temu.com - Google Chrome", "temu.com"),
        ] {
            assert_eq!(
                list.match_title(title).as_deref(),
                Some(expected),
                "{title}"
            );
        }
    }

    #[test]
    fn short_labels_need_the_full_domain() {
        let list = defaults();
        assert_eq!(list.match_title("Xcode"), None);
        assert_eq!(list.match_title("Home / X"), None);
        assert_eq!(list.match_title("x.company.org"), None);
        let x_only = parse_ok("x.com");
        assert_eq!(x_only.match_title("netflix.com - Chrome"), None);
    }

    #[test]
    fn common_word_labels_need_the_full_domain() {
        let list = defaults();
        for title in [
            "Third-party crates - Google Chrome",
            "Wysłano 2 godziny temu - Gmail - Google Chrome",
            "People also ask - Google Chrome",
            "Max width in CSS - Stack Overflow",
            "Rust threads and channels - Opera",
            "Onet – Jesteś na bieżąco - Google Chrome",
            "Mesh polygon count - Unity Discussions",
            "Viva Engage - Microsoft Edge",
            "PPE requirements - Google Chrome",
            "Chess engine in Rust - GitHub - Google Chrome",
            "Mistrzowie Excela - kurs - Opera",
        ] {
            assert_eq!(list.match_title(title), None, "{title}");
        }
        assert_eq!(
            list.match_title("Viva.pl - Google Chrome").as_deref(),
            Some("viva.pl")
        );
    }

    #[test]
    fn flags_pasted_urls_with_a_query() {
        let (parsed, issues) = Blocklist::parse(
            "https://www.youtube.com/watch?v=abc\nyoutube.com/results?search_query=lofi\nreddit.com/r/polska/?\n/https://x.com/a?b/\ncolou?r\nhttps://x.com/search?q=a+b\n",
        );
        let summary: Vec<_> = issues.iter().map(|i| i.line).collect();
        assert_eq!(summary, [1, 2, 6]);
        assert!(
            issues[0].message.contains("wpisz youtube.com"),
            "{issues:?}"
        );
        assert!(issues[2].message.contains("wpisz x.com"), "{issues:?}");
        // Regexes that match their own text, explicit /…/ regexes and non-addresses stay regexes.
        assert_eq!(
            parsed.pattern_sources(),
            ["reddit.com/r/polska/?", "https://x.com/a?b", "colou?r"]
        );
        assert!(parsed.domains().is_empty());
    }

    #[test]
    fn title_matching_handles_edge_cases() {
        let list = defaults();
        assert_eq!(list.match_title(""), None);
        assert_eq!(list.match_title("   "), None);
        assert_eq!(list.match_title("Visual Studio Code"), None);
        assert_eq!(Blocklist::default().match_title("reddit"), None);
        let custom = parse_ok("\n  News.YCombinator.com \n");
        assert_eq!(
            custom.match_title("Hacker News | ycombinator").as_deref(),
            Some("news.ycombinator.com")
        );
        // Multi-byte characters around a match never break slicing.
        assert_eq!(
            list.match_title("Zażółć reddit gęślą jaźń").as_deref(),
            Some("reddit.com")
        );
        assert_eq!(list.match_title("Zażółćreddit"), None);
    }

    #[test]
    fn title_falls_back_to_regexes() {
        let list = parse_ok("reddit.com\n/^\\(\\d+\\) youtube/\n");
        assert_eq!(
            list.match_title("(3) YouTube - Google Chrome").as_deref(),
            Some(r"^\(\d+\) youtube")
        );
        assert_eq!(list.match_title("YouTube - Google Chrome"), None);
        assert_eq!(
            list.match_title("(3) Reddit - Google Chrome").as_deref(),
            Some("reddit.com")
        );
    }

    #[test]
    fn equality_compares_domains_and_sources() {
        let a = parse_ok("A.com\n/b\\.com/\n");
        let b = parse_ok("a.com\nb\\.com\n");
        assert_eq!(a, b);
        assert_ne!(a, parse_ok("a.com\n"));
        assert_ne!(a, parse_ok("a.com\nB\\.com\n"));
        assert_eq!(Blocklist::default(), parse_ok("# nothing\n"));
        assert!(Blocklist::default().is_empty());
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
