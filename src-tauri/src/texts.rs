//! The words the app shows outside its web pages (tray menu, tooltip), in one place.

use crate::status::Status;

pub const APP: &str = "Mundwerk";
pub const SETTINGS: &str = "Einstellungen …";
pub const OVERLAY_SHOWN: &str = "Overlay zeigen";
pub const HISTORY: &str = "Verlauf (anklicken = kopieren)";
pub const HISTORY_EMPTY: &str = "noch nichts diktiert";
pub const QUIT: &str = "Beenden";
pub const OPEN_LOG: &str = "Protokoll öffnen";
pub const RESEND: &str = "Letzte Aufnahme erneut senden";
/// After an error of the STT-Suite: where the recording waits.
pub const RESEND_HINT: &str = "im Tray: „Letzte Aufnahme erneut senden“";
pub const NOTHING_TO_REPEAT: &str = "noch kein Diktat zum erneuten Einfügen";
pub const SETTINGS_TITLE: &str = "Mundwerk – Einstellungen";

/// How long a history entry may be in the menu before it is cut.
pub const HISTORY_LABEL_CHARS: usize = 60;

#[must_use]
pub fn status(status: &Status) -> String {
    match status {
        Status::Idle => format!("{APP}: bereit"),
        Status::Listening => format!("{APP}: hört zu …"),
        Status::Transcribing => format!("{APP}: transkribiert …"),
        Status::Error(message) => format!("{APP}: {message}"),
    }
}
