//! The words the app shows outside its web pages (tray menu, tooltip, error messages), from the same file the pages
//! load (`ui/i18n.json`, see `mundwerk_core::i18n`), in the language of the settings.

use std::sync::{Mutex, OnceLock};

use mundwerk_core::i18n::{Language, LanguageSetting, Texts};

use crate::engine::lock;
use crate::status::Status;

/// How long a history entry may be in the menu before it is cut.
pub const HISTORY_LABEL_CHARS: usize = 60;

static TEXTS: OnceLock<Texts> = OnceLock::new();
static LANGUAGE: Mutex<Language> = Mutex::new(Language::En);

/// Show the app in the language of `setting` from now on (`auto`: the system's).
pub fn set_language(setting: LanguageSetting) {
    let language = setting.resolve(sys_locale::get_locale().as_deref());
    log::info!("language: {language:?}");
    *lock(&LANGUAGE) = language;
}

#[must_use]
pub fn language() -> Language {
    *lock(&LANGUAGE)
}

/// The text for `key` in the current language. A key the file does not have is a bug the core's tests catch
/// (`every_key_the_code_uses_exists`).
#[must_use]
pub fn tr(key: &str) -> String {
    let texts = TEXTS.get_or_init(|| Texts::load().expect("ui/i18n.json is checked by the tests"));
    texts.get(language(), key).unwrap_or_else(|| panic!("no text {key:?} in ui/i18n.json")).to_owned()
}

#[must_use]
pub fn status(status: &Status) -> String {
    let app = tr("app");
    match status {
        Status::Idle => format!("{app}: {}", tr("status.idle")),
        Status::Listening => format!("{app}: {}", tr("status.listening")),
        Status::Transcribing => format!("{app}: {}", tr("status.transcribing")),
        Status::Error(message) => format!("{app}: {message}"),
    }
}
