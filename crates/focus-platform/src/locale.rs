//! The user's system UI language, as a language tag (e.g. `pl-PL`, `en-US`, `zh-Hans-CN`).
//!
//! The shell maps the tag to a supported UI language with `focus_core::api::Language::from_tag`
//! (falling back to English for unsupported languages).
//!
//! * Windows: the first entry of `GetUserPreferredUILanguages(MUI_LANGUAGE_NAME)` (the Windows display
//!   language), falling back to `GetUserDefaultLocaleName` (regional format).
//! * Elsewhere: the first non-empty of the `LC_ALL`, `LC_MESSAGES` and `LANG` environment variables
//!   (POSIX precedence), e.g. `pl_PL.UTF-8`.

/// System UI language tag, or `None` when it cannot be determined.
pub fn system_language_tag() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::preferred_ui_language().or_else(windows_impl::default_locale_name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        first_env_tag(
            ["LC_ALL", "LC_MESSAGES", "LANG"]
                .into_iter()
                .map(|key| std::env::var(key).ok()),
        )
    }
}

/// First entry of a NUL-separated (double-NUL-terminated) UTF-16 string list, trimmed; `None` when empty.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn first_multi_sz(buffer: &[u16]) -> Option<String> {
    let first = buffer.split(|&unit| unit == 0).next()?;
    let tag = String::from_utf16_lossy(first);
    let tag = tag.trim();
    (!tag.is_empty()).then(|| tag.to_owned())
}

/// First non-blank value, trimmed (environment variables in precedence order; an empty variable counts as unset).
#[cfg_attr(target_os = "windows", allow(dead_code))]
fn first_env_tag(values: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_owned())
        .find(|value| !value.is_empty())
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows::Win32::Globalization::{
        GetUserDefaultLocaleName, GetUserPreferredUILanguages, MUI_LANGUAGE_NAME,
    };
    use windows::core::PWSTR;

    use super::first_multi_sz;

    /// `LOCALE_NAME_MAX_LENGTH` (characters, including the terminating NUL).
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    /// Upper bound for the preferred UI language list (UTF-16 units); real lists are a few dozen.
    const MAX_LANGUAGE_LIST_CHARS: u32 = 64 * 1024;

    /// First Windows display language, e.g. `pl-PL`.
    pub(super) fn preferred_ui_language() -> Option<String> {
        let mut count = 0u32;
        let mut len = 0u32;
        // SAFETY: both out pointers are valid; with no buffer the call only reports the needed size.
        unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut len) }
            .ok()?;
        if len == 0 || len > MAX_LANGUAGE_LIST_CHARS {
            return None;
        }
        let mut buffer = vec![0u16; usize::try_from(len).ok()?];
        // SAFETY: `buffer` holds `len` UTF-16 units and outlives the call; the out pointers are valid. If the list
        // grew since the first call, the call fails (insufficient buffer) instead of overflowing.
        unsafe {
            GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                Some(PWSTR(buffer.as_mut_ptr())),
                &mut len,
            )
        }
        .ok()?;
        let filled = usize::try_from(len).unwrap_or(0).min(buffer.len());
        first_multi_sz(&buffer[..filled])
    }

    /// The user's default locale (regional format), e.g. `pl-PL`.
    pub(super) fn default_locale_name() -> Option<String> {
        let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH];
        // SAFETY: the buffer is valid for its whole length; the API writes at most that many units (with NUL).
        let written = unsafe { GetUserDefaultLocaleName(&mut buffer) };
        let written = usize::try_from(written).ok()?.min(buffer.len());
        first_multi_sz(&buffer[..written])
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn windows_reports_a_language() {
            let ui = super::preferred_ui_language();
            assert!(ui.as_deref().is_some_and(|tag| !tag.is_empty()), "{ui:?}");
            let locale = super::default_locale_name();
            assert!(
                locale.as_deref().is_some_and(|tag| !tag.is_empty()),
                "{locale:?}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn first_entry_of_a_multi_string() {
        assert_eq!(
            first_multi_sz(&utf16("pl-PL\0en-US\0\0")).as_deref(),
            Some("pl-PL")
        );
        assert_eq!(
            first_multi_sz(&utf16("zh-Hans-CN\0\0")).as_deref(),
            Some("zh-Hans-CN")
        );
        assert_eq!(
            first_multi_sz(&utf16("en-US")).as_deref(),
            Some("en-US"),
            "no NUL"
        );
        assert_eq!(first_multi_sz(&utf16("\0en-US\0\0")), None, "empty first");
        assert_eq!(first_multi_sz(&[]), None);
        assert_eq!(first_multi_sz(&utf16("  \0")), None);
    }

    #[test]
    fn env_precedence_skips_empty_values() {
        let tag = |values: [Option<&str>; 3]| first_env_tag(values.map(|v| v.map(str::to_owned)));
        assert_eq!(
            tag([Some("de_DE.UTF-8"), Some("fr_FR"), Some("pl_PL.UTF-8")]).as_deref(),
            Some("de_DE.UTF-8")
        );
        assert_eq!(
            tag([Some(""), None, Some("pl_PL.UTF-8")]).as_deref(),
            Some("pl_PL.UTF-8")
        );
        assert_eq!(tag([None, Some(" ja_JP "), None]).as_deref(), Some("ja_JP"));
        assert_eq!(tag([None, None, None]), None);
        assert_eq!(tag([Some(" "), Some(""), None]), None);
    }

    #[test]
    fn platform_tag_is_never_blank() {
        if let Some(tag) = system_language_tag() {
            assert!(!tag.trim().is_empty());
            assert_eq!(tag, tag.trim());
        }
        #[cfg(target_os = "windows")]
        assert!(system_language_tag().is_some());
    }
}
