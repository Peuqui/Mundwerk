//! Writing the text into the field that has the focus: typed (Unicode, the clipboard untouched) or pasted (Ctrl+V,
//! then the clipboard gets back what it held), as the settings say for the program in front.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use mundwerk_core::config::{Insert, InsertMethod};
use mundwerk_core::spacing::Spacing;

pub struct Inserter {
    enigo: Enigo,
    spacing: Spacing,
}

/// The window in front: an id for the space between dictations, the process name for the method.
struct Target {
    window: u64,
    process: String,
}

impl Inserter {
    pub fn new() -> Result<Self, String> {
        let enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;
        Ok(Self { enigo, spacing: Spacing::default() })
    }

    pub fn insert(&mut self, settings: &Insert, text: &str) -> Result<(), String> {
        let target = target()?;
        let prefix = if settings.space_between_dictations { self.spacing.prefix(target.window, text) } else { "" };
        let full = format!("{prefix}{text}");
        match settings.method_for(&target.process) {
            InsertMethod::Type => self.enigo.text(&full).map_err(|error| error.to_string())?,
            InsertMethod::Clipboard => self.paste(&full, Duration::from_millis(settings.clipboard_restore_delay_ms))?,
        }
        if settings.press_enter {
            self.enigo.key(Key::Return, Direction::Click).map_err(|error| error.to_string())?;
            // After Enter the field starts anew: the next dictation needs no space before it.
            self.spacing.forget();
        }
        Ok(())
    }

    /// Put `text` into the clipboard, press Ctrl+V, and after `restore_delay` give the clipboard back the text it
    /// held (only text: an image there is gone, which the settings window says).
    fn paste(&mut self, text: &str, restore_delay: Duration) -> Result<(), String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        let held = clipboard.get_text().ok();
        clipboard.set_text(text).map_err(|error| error.to_string())?;
        self.enigo.key(Key::Control, Direction::Press).map_err(|error| error.to_string())?;
        let pressed = self.enigo.key(Key::Unicode('v'), Direction::Click);
        self.enigo.key(Key::Control, Direction::Release).map_err(|error| error.to_string())?;
        pressed.map_err(|error| error.to_string())?;
        std::thread::sleep(restore_delay);
        if let Some(held) = held {
            clipboard.set_text(held).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn target() -> Result<Target, String> {
    let window = active_win_pos_rs::get_active_window().map_err(|()| "no window has the focus".to_owned())?;
    let mut hasher = DefaultHasher::new();
    (window.process_id, &window.window_id).hash(&mut hasher);
    let process = window.process_path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Target { window: hasher.finish(), process })
}

/// Copy `text` to the clipboard (the history's "copy").
pub fn copy(text: &str) -> Result<(), String> {
    arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)).map_err(|error| error.to_string())
}
