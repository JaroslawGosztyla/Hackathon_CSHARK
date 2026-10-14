//! UI language selection: mapping OS locale names to the supported [`Language`]s and resolving
//! `Language::System`. The OS language itself is read by the shell (`focus-platform`).

use crate::api::Language;

impl Language {
    /// Every concrete language (everything except `System`), in the settings order.
    pub const SUPPORTED: [Language; 10] = [
        Language::En,
        Language::Pl,
        Language::De,
        Language::Fr,
        Language::It,
        Language::Uk,
        Language::Ru,
        Language::Es,
        Language::Zh,
        Language::Ja,
    ];

    /// The supported language of a BCP-47 tag or locale name (`pl-PL`, `en_US.UTF-8`, `zh-Hans-CN`,
    /// `es-419`), decided by its primary language subtag (also the ISO 639-2 code, `pol`).
    /// Case-insensitive; `-`, `_`, `.` and `@` separate subtags. Never returns `System`;
    /// `None` for unsupported or malformed tags (`ko-KR`, `C`, `system`, empty).
    pub fn from_tag(tag: &str) -> Option<Language> {
        let primary = tag.trim().split(['-', '_', '.', '@']).next()?;
        let language = match primary.to_ascii_lowercase().as_str() {
            "en" | "eng" => Language::En,
            "pl" | "pol" => Language::Pl,
            "de" | "deu" | "ger" => Language::De,
            "fr" | "fra" | "fre" => Language::Fr,
            "it" | "ita" => Language::It,
            "uk" | "ukr" => Language::Uk,
            "ru" | "rus" => Language::Ru,
            "es" | "spa" => Language::Es,
            "zh" | "zho" | "chi" => Language::Zh,
            "ja" | "jpn" => Language::Ja,
            _ => return None,
        };
        Some(language)
    }

    /// The language to show: `System` becomes `system` (the OS language from [`Language::from_tag`]),
    /// and when that is `System` too, English.
    pub fn resolve(self, system: Language) -> Language {
        match (self, system) {
            (Language::System, Language::System) => Language::En,
            (Language::System, system) => system,
            (chosen, _) => chosen,
        }
    }

    /// Lowercase code, the same as in JSON: `en`, `pl`, `de`, `fr`, `it`, `uk`, `ru`, `es`, `zh`,
    /// `ja`, and `system` for `System`.
    pub fn code(self) -> &'static str {
        match self {
            Language::System => "system",
            Language::En => "en",
            Language::Pl => "pl",
            Language::De => "de",
            Language::Fr => "fr",
            Language::It => "it",
            Language::Uk => "uk",
            Language::Ru => "ru",
            Language::Es => "es",
            Language::Zh => "zh",
            Language::Ja => "ja",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_locale_names() {
        for (tag, expected) in [
            ("pl-PL", Some(Language::Pl)),
            ("pl", Some(Language::Pl)),
            ("PL_pl", Some(Language::Pl)),
            ("pol", Some(Language::Pl)),
            ("en-US", Some(Language::En)),
            ("en-GB", Some(Language::En)),
            ("en_US.UTF-8", Some(Language::En)),
            ("de", Some(Language::De)),
            ("de-AT", Some(Language::De)),
            ("fr-CA", Some(Language::Fr)),
            ("it", Some(Language::It)),
            ("it-CH", Some(Language::It)),
            ("uk-UA", Some(Language::Uk)),
            ("ru", Some(Language::Ru)),
            ("ru-RU", Some(Language::Ru)),
            ("es-419", Some(Language::Es)),
            ("es-ES", Some(Language::Es)),
            ("zh-Hans-CN", Some(Language::Zh)),
            ("zh-TW", Some(Language::Zh)),
            ("zh-CHS", Some(Language::Zh)),
            ("ja-JP", Some(Language::Ja)),
            ("  JA-jp  ", Some(Language::Ja)),
            ("sr-Latn-RS@latin", None),
            ("ko-KR", None),
            ("pt-BR", None),
            ("nl", None),
            ("C", None),
            ("POSIX", None),
            ("system", None),
            ("", None),
            ("-", None),
            ("   ", None),
        ] {
            assert_eq!(Language::from_tag(tag), expected, "{tag:?}");
        }
    }

    #[test]
    fn from_tag_never_returns_system() {
        for language in Language::SUPPORTED {
            assert_eq!(Language::from_tag(language.code()), Some(language));
        }
        assert_eq!(Language::from_tag(Language::System.code()), None);
    }

    #[test]
    fn resolves_system_language() {
        assert_eq!(Language::System.resolve(Language::Pl), Language::Pl);
        assert_eq!(Language::System.resolve(Language::Ja), Language::Ja);
        assert_eq!(Language::System.resolve(Language::System), Language::En);
        assert_eq!(Language::De.resolve(Language::Pl), Language::De);
        assert_eq!(Language::Uk.resolve(Language::System), Language::Uk);
        for language in Language::SUPPORTED {
            assert_eq!(language.resolve(Language::Fr), language);
            assert_ne!(Language::System.resolve(language), Language::System);
        }
    }

    #[test]
    fn codes_match_json_and_are_unique() {
        let mut all = vec![Language::System];
        all.extend(Language::SUPPORTED);
        for (index, language) in all.iter().enumerate() {
            let json = serde_json::to_value(language).expect("serialize");
            assert_eq!(json, language.code(), "{language:?}");
            let back: Language = serde_json::from_value(json).expect("deserialize");
            assert_eq!(back, *language);
            assert!(!all.iter().skip(index + 1).any(|other| other == language));
        }
        assert_eq!(all.len(), 11);
    }
}
