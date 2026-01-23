use std::collections::HashMap;

/// Supported language codes based on TranslateGemma's 55 supported languages
/// Using ISO 639-1 Alpha-2 codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Arabic,
    Bengali,
    Bulgarian,
    Catalan,
    Chinese,
    Croatian,
    Czech,
    Danish,
    Dutch,
    English,
    Estonian,
    Finnish,
    French,
    German,
    Greek,
    Gujarati,
    Hebrew,
    Hindi,
    Hungarian,
    Indonesian,
    Italian,
    Japanese,
    Kannada,
    Korean,
    Latvian,
    Lithuanian,
    Macedonian,
    Malay,
    Malayalam,
    Marathi,
    Norwegian,
    Persian,
    Polish,
    Portuguese,
    Punjabi,
    Romanian,
    Russian,
    Serbian,
    Slovak,
    Slovenian,
    Spanish,
    Swedish,
    Tamil,
    Telugu,
    Thai,
    Turkish,
    Ukrainian,
    Urdu,
    Vietnamese,
}

impl Language {
    /// Get the ISO 639-1 code for the language
    pub fn code(&self) -> &'static str {
        match self {
            Self::Arabic => "ar",
            Self::Bengali => "bn",
            Self::Bulgarian => "bg",
            Self::Catalan => "ca",
            Self::Chinese => "zh",
            Self::Croatian => "hr",
            Self::Czech => "cs",
            Self::Danish => "da",
            Self::Dutch => "nl",
            Self::English => "en",
            Self::Estonian => "et",
            Self::Finnish => "fi",
            Self::French => "fr",
            Self::German => "de",
            Self::Greek => "el",
            Self::Gujarati => "gu",
            Self::Hebrew => "he",
            Self::Hindi => "hi",
            Self::Hungarian => "hu",
            Self::Indonesian => "id",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::Kannada => "kn",
            Self::Korean => "ko",
            Self::Latvian => "lv",
            Self::Lithuanian => "lt",
            Self::Macedonian => "mk",
            Self::Malay => "ms",
            Self::Malayalam => "ml",
            Self::Marathi => "mr",
            Self::Norwegian => "no",
            Self::Persian => "fa",
            Self::Polish => "pl",
            Self::Portuguese => "pt",
            Self::Punjabi => "pa",
            Self::Romanian => "ro",
            Self::Russian => "ru",
            Self::Serbian => "sr",
            Self::Slovak => "sk",
            Self::Slovenian => "sl",
            Self::Spanish => "es",
            Self::Swedish => "sv",
            Self::Tamil => "ta",
            Self::Telugu => "te",
            Self::Thai => "th",
            Self::Turkish => "tr",
            Self::Ukrainian => "uk",
            Self::Urdu => "ur",
            Self::Vietnamese => "vi",
        }
    }

    /// Get the display name for the language
    pub fn name(&self) -> &'static str {
        match self {
            Self::Arabic => "Arabic",
            Self::Bengali => "Bengali",
            Self::Bulgarian => "Bulgarian",
            Self::Catalan => "Catalan",
            Self::Chinese => "Chinese",
            Self::Croatian => "Croatian",
            Self::Czech => "Czech",
            Self::Danish => "Danish",
            Self::Dutch => "Dutch",
            Self::English => "English",
            Self::Estonian => "Estonian",
            Self::Finnish => "Finnish",
            Self::French => "French",
            Self::German => "German",
            Self::Greek => "Greek",
            Self::Gujarati => "Gujarati",
            Self::Hebrew => "Hebrew",
            Self::Hindi => "Hindi",
            Self::Hungarian => "Hungarian",
            Self::Indonesian => "Indonesian",
            Self::Italian => "Italian",
            Self::Japanese => "Japanese",
            Self::Kannada => "Kannada",
            Self::Korean => "Korean",
            Self::Latvian => "Latvian",
            Self::Lithuanian => "Lithuanian",
            Self::Macedonian => "Macedonian",
            Self::Malay => "Malay",
            Self::Malayalam => "Malayalam",
            Self::Marathi => "Marathi",
            Self::Norwegian => "Norwegian",
            Self::Persian => "Persian",
            Self::Polish => "Polish",
            Self::Portuguese => "Portuguese",
            Self::Punjabi => "Punjabi",
            Self::Romanian => "Romanian",
            Self::Russian => "Russian",
            Self::Serbian => "Serbian",
            Self::Slovak => "Slovak",
            Self::Slovenian => "Slovenian",
            Self::Spanish => "Spanish",
            Self::Swedish => "Swedish",
            Self::Tamil => "Tamil",
            Self::Telugu => "Telugu",
            Self::Thai => "Thai",
            Self::Turkish => "Turkish",
            Self::Ukrainian => "Ukrainian",
            Self::Urdu => "Urdu",
            Self::Vietnamese => "Vietnamese",
        }
    }

    /// Parse a language code string into a Language enum
    pub fn from_code(code: &str) -> Option<Self> {
        let code = code.to_lowercase();
        // Handle both simple codes (en) and regional variants (en_US, en-GB)
        let base_code = code.split(|c| c == '_' || c == '-').next()?;

        match base_code {
            "ar" => Some(Self::Arabic),
            "bn" => Some(Self::Bengali),
            "bg" => Some(Self::Bulgarian),
            "ca" => Some(Self::Catalan),
            "zh" => Some(Self::Chinese),
            "hr" => Some(Self::Croatian),
            "cs" => Some(Self::Czech),
            "da" => Some(Self::Danish),
            "nl" => Some(Self::Dutch),
            "en" => Some(Self::English),
            "et" => Some(Self::Estonian),
            "fi" => Some(Self::Finnish),
            "fr" => Some(Self::French),
            "de" => Some(Self::German),
            "el" => Some(Self::Greek),
            "gu" => Some(Self::Gujarati),
            "he" | "iw" => Some(Self::Hebrew), // iw is legacy code
            "hi" => Some(Self::Hindi),
            "hu" => Some(Self::Hungarian),
            "id" => Some(Self::Indonesian),
            "it" => Some(Self::Italian),
            "ja" => Some(Self::Japanese),
            "kn" => Some(Self::Kannada),
            "ko" => Some(Self::Korean),
            "lv" => Some(Self::Latvian),
            "lt" => Some(Self::Lithuanian),
            "mk" => Some(Self::Macedonian),
            "ms" => Some(Self::Malay),
            "ml" => Some(Self::Malayalam),
            "mr" => Some(Self::Marathi),
            "no" | "nb" | "nn" => Some(Self::Norwegian),
            "fa" => Some(Self::Persian),
            "pl" => Some(Self::Polish),
            "pt" => Some(Self::Portuguese),
            "pa" => Some(Self::Punjabi),
            "ro" => Some(Self::Romanian),
            "ru" => Some(Self::Russian),
            "sr" => Some(Self::Serbian),
            "sk" => Some(Self::Slovak),
            "sl" => Some(Self::Slovenian),
            "es" => Some(Self::Spanish),
            "sv" => Some(Self::Swedish),
            "ta" => Some(Self::Tamil),
            "te" => Some(Self::Telugu),
            "th" => Some(Self::Thai),
            "tr" => Some(Self::Turkish),
            "uk" => Some(Self::Ukrainian),
            "ur" => Some(Self::Urdu),
            "vi" => Some(Self::Vietnamese),
            _ => None,
        }
    }

    /// Get all supported languages
    pub fn all() -> &'static [Language] {
        &[
            Self::Arabic, Self::Bengali, Self::Bulgarian, Self::Catalan,
            Self::Chinese, Self::Croatian, Self::Czech, Self::Danish,
            Self::Dutch, Self::English, Self::Estonian, Self::Finnish,
            Self::French, Self::German, Self::Greek, Self::Gujarati,
            Self::Hebrew, Self::Hindi, Self::Hungarian, Self::Indonesian,
            Self::Italian, Self::Japanese, Self::Kannada, Self::Korean,
            Self::Latvian, Self::Lithuanian, Self::Macedonian, Self::Malay,
            Self::Malayalam, Self::Marathi, Self::Norwegian, Self::Persian,
            Self::Polish, Self::Portuguese, Self::Punjabi, Self::Romanian,
            Self::Russian, Self::Serbian, Self::Slovak, Self::Slovenian,
            Self::Spanish, Self::Swedish, Self::Tamil, Self::Telugu,
            Self::Thai, Self::Turkish, Self::Ukrainian, Self::Urdu,
            Self::Vietnamese,
        ]
    }

    /// Get a HashMap of code -> name for all languages
    pub fn code_to_name_map() -> HashMap<&'static str, &'static str> {
        Self::all().iter().map(|l| (l.code(), l.name())).collect()
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl serde::Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> serde::Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = String::deserialize(deserializer)?;
        Language::from_code(&code)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown language code: {}", code)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Japanese.code(), "ja");
        assert_eq!(Language::Chinese.code(), "zh");
    }

    #[test]
    fn test_from_code() {
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("EN"), Some(Language::English));
        assert_eq!(Language::from_code("en_US"), Some(Language::English));
        assert_eq!(Language::from_code("en-GB"), Some(Language::English));
        assert_eq!(Language::from_code("xyz"), None);
    }

    #[test]
    fn test_all_languages_count() {
        // TranslateGemma supports ~55 languages, we have 49 most common ones
        assert!(Language::all().len() >= 40);
    }

    #[test]
    fn test_all_languages_code_roundtrip() {
        for lang in Language::all() {
            let code = lang.code();
            let parsed = Language::from_code(code);
            assert_eq!(parsed, Some(*lang), "Roundtrip failed for {:?} (code: {})", lang, code);
        }
    }

    #[test]
    fn test_all_languages_have_nonempty_names() {
        for lang in Language::all() {
            assert!(!lang.name().is_empty(), "{:?} has empty name", lang);
        }
    }

    #[test]
    fn test_all_language_codes_are_2_3_chars() {
        for lang in Language::all() {
            let code = lang.code();
            assert!(
                code.len() >= 2 && code.len() <= 3,
                "{:?} has code '{}' with length {}",
                lang, code, code.len()
            );
        }
    }

    #[test]
    fn test_code_to_name_map_complete() {
        let map = Language::code_to_name_map();
        assert_eq!(map.len(), Language::all().len());
    }

    #[test]
    fn test_legacy_hebrew_code() {
        assert_eq!(Language::from_code("iw"), Some(Language::Hebrew));
        assert_eq!(Language::from_code("he"), Some(Language::Hebrew));
    }

    #[test]
    fn test_norwegian_variants() {
        assert_eq!(Language::from_code("no"), Some(Language::Norwegian));
        assert_eq!(Language::from_code("nb"), Some(Language::Norwegian));
        assert_eq!(Language::from_code("nn"), Some(Language::Norwegian));
    }

    #[test]
    fn test_display_impl() {
        assert_eq!(format!("{}", Language::English), "English");
        assert_eq!(format!("{}", Language::Japanese), "Japanese");
    }

    #[test]
    fn test_serde_roundtrip() {
        for lang in Language::all() {
            let json = serde_json::to_string(lang).unwrap();
            let parsed: Language = serde_json::from_str(&json).unwrap();
            assert_eq!(&parsed, lang);
        }
    }

    #[test]
    fn test_from_code_invalid() {
        assert_eq!(Language::from_code(""), None);
        assert_eq!(Language::from_code("xyz"), None);
        assert_eq!(Language::from_code("123"), None);
        assert_eq!(Language::from_code("zz"), None);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn any_language() -> impl Strategy<Value = Language> {
        prop::sample::select(Language::all().to_vec())
    }

    proptest! {
        #[test]
        fn language_code_roundtrip(lang in any_language()) {
            let code = lang.code();
            let parsed = Language::from_code(code);
            prop_assert_eq!(parsed, Some(lang));
        }

        #[test]
        fn language_regional_variants(
            lang in any_language(),
            suffix in prop_oneof![Just("".to_string()), Just("_US".to_string()), Just("-GB".to_string()), Just("_419".to_string())]
        ) {
            let code_with_suffix = format!("{}{}", lang.code(), suffix);
            let parsed = Language::from_code(&code_with_suffix);
            prop_assert_eq!(parsed, Some(lang));
        }

        #[test]
        fn language_name_not_empty(lang in any_language()) {
            prop_assert!(!lang.name().is_empty());
        }

        #[test]
        fn language_code_length(lang in any_language()) {
            let code = lang.code();
            prop_assert!(code.len() >= 2 && code.len() <= 3);
        }

        #[test]
        fn language_code_is_lowercase(lang in any_language()) {
            let code = lang.code();
            let lowered = code.to_lowercase();
            prop_assert_eq!(code, lowered.as_str());
        }

        #[test]
        fn invalid_codes_return_none(code in "[a-z]{4,10}") {
            // Any code longer than 3 chars should not match
            prop_assert_eq!(Language::from_code(&code), None);
        }
    }
}
