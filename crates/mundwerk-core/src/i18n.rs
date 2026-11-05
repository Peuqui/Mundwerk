//! The words of Mundwerk in German and English, from one file, `ui/i18n.json`: the pages load it themselves, the app
//! has it built in. Which language: the setting `system.language`, `auto` following the system's.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Error;

/// The texts, as the pages load them.
pub const TEXTS_JSON: &str = include_str!("../../../ui/i18n.json");

/// A language the texts are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    De,
    En,
}

/// The setting: a language, or the system's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSetting {
    Auto,
    De,
    En,
}

impl LanguageSetting {
    /// The language to show. `locale`: the system's (`"de-DE"`, `"en_US"`); German for a German one, English for
    /// any other, and for none.
    #[must_use]
    pub fn resolve(self, locale: Option<&str>) -> Language {
        let german_system = locale.is_some_and(|locale| locale.to_ascii_lowercase().starts_with("de"));
        match self {
            Self::De => Language::De,
            Self::Auto if german_system => Language::De,
            Self::En | Self::Auto => Language::En,
        }
    }
}

pub struct Texts {
    by_language: HashMap<Language, HashMap<String, String>>,
}

impl Texts {
    /// The built-in texts.
    ///
    /// # Errors
    /// `ui/i18n.json` is no valid texts file (the tests rule it out).
    pub fn load() -> Result<Self, Error> {
        let by_language =
            serde_json::from_str(TEXTS_JSON).map_err(|error| Error::Config(format!("i18n.json: {error}")))?;
        Ok(Self { by_language })
    }

    /// The text for `key`; `None` for a key the file does not have (the tests make sure every key the code uses
    /// is there, in both languages).
    #[must_use]
    pub fn get(&self, language: Language, key: &str) -> Option<&str> {
        self.by_language.get(&language)?.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use regex::Regex;

    use super::*;

    fn keys(language: Language) -> BTreeSet<String> {
        Texts::load().unwrap().by_language[&language].keys().cloned().collect()
    }

    #[test]
    fn both_languages_have_the_same_keys() {
        let german = keys(Language::De);
        let english = keys(Language::En);
        assert_eq!(german.difference(&english).collect::<Vec<_>>(), Vec::<&String>::new(), "only in German");
        assert_eq!(english.difference(&german).collect::<Vec<_>>(), Vec::<&String>::new(), "only in English");
    }

    #[test]
    fn no_text_is_empty() {
        let texts = Texts::load().unwrap();
        for (language, words) in &texts.by_language {
            for (key, text) in words {
                assert!(!text.trim().is_empty(), "{language:?} {key}");
            }
        }
    }

    /// Every key the pages (`t('…')`) and the app (`tr("…")`) use is in the file.
    #[test]
    fn every_key_the_code_uses_exists() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let known = keys(Language::De);
        let uses = [("ui", r"\bt\('([^']+)'\)", "js"), ("src-tauri/src", r#"\btr\("([^"]+)"\)"#, "rs")];
        let mut found = 0;
        for (folder, pattern, extension) in uses {
            let pattern = Regex::new(pattern).unwrap();
            for entry in std::fs::read_dir(root.join(folder)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some(extension) {
                    continue;
                }
                let source = std::fs::read_to_string(&path).unwrap();
                for capture in pattern.captures_iter(&source) {
                    found += 1;
                    assert!(known.contains(&capture[1]), "{}: unknown key {:?}", path.display(), &capture[1]);
                }
            }
        }
        assert!(found > 50, "the pattern should find the uses, found {found}");
    }

    #[test]
    fn auto_follows_the_system() {
        assert_eq!(LanguageSetting::Auto.resolve(Some("de-DE")), Language::De);
        assert_eq!(LanguageSetting::Auto.resolve(Some("de_AT")), Language::De);
        assert_eq!(LanguageSetting::Auto.resolve(Some("en-US")), Language::En);
        assert_eq!(LanguageSetting::Auto.resolve(Some("fr-FR")), Language::En);
        assert_eq!(LanguageSetting::Auto.resolve(None), Language::En);
        assert_eq!(LanguageSetting::De.resolve(Some("en-US")), Language::De);
    }
}
