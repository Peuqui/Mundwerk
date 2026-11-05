//! What starts a dictation, as written in the settings: `"F9"`, `"Ctrl+Alt+Space"`, `"Mouse:X2"`. The app maps
//! the parsed trigger onto the keyboard and mouse hooks of the system.

use crate::Error;

/// The keys a trigger can name besides letters and digits.
const NAMED_KEYS: &[&str] = &[
    "Space",
    "Enter",
    "Tab",
    "Escape",
    "Backspace",
    "Insert",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
    "Pause",
    "ScrollLock",
    "CapsLock",
    "PrintScreen",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
];

/// The mouse buttons a trigger can use. Not the left and right button: they are needed for everything else.
const MOUSE_BUTTONS: &[&str] = &["Middle", "X1", "X2"];

const MOUSE_PREFIX: &str = "Mouse:";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // four independent keys, as they are on the keyboard
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// A key by its name: a letter `A`-`Z`, a digit `0`-`9` or one of [`NAMED_KEYS`].
    Key(String),
    /// `Middle`, `X1` or `X2`.
    Mouse(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    pub modifiers: Modifiers,
    pub input: Input,
}

impl Trigger {
    /// Read a trigger as the settings write it.
    ///
    /// # Errors
    /// An unknown key, button or modifier, or nothing to press.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let refuse = |why: &str| Error::Config(format!("trigger {text:?}: {why}"));
        if let Some(button) = text.strip_prefix(MOUSE_PREFIX) {
            if !MOUSE_BUTTONS.contains(&button) {
                return Err(refuse(&format!("mouse button one of {MOUSE_BUTTONS:?}")));
            }
            return Ok(Self { modifiers: Modifiers::default(), input: Input::Mouse(button.to_owned()) });
        }
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = parts.pop().filter(|key| !key.is_empty()).ok_or_else(|| refuse("no key"))?;
        let mut modifiers = Modifiers::default();
        for part in parts {
            let flag = match part {
                "Ctrl" => &mut modifiers.ctrl,
                "Alt" => &mut modifiers.alt,
                "Shift" => &mut modifiers.shift,
                "Meta" => &mut modifiers.meta,
                _ => return Err(refuse(&format!("{part:?} is no modifier (Ctrl, Alt, Shift, Meta)"))),
            };
            *flag = true;
        }
        if !is_key(key) {
            return Err(refuse(&format!("{key:?} is no key")));
        }
        Ok(Self { modifiers, input: Input::Key(key.to_owned()) })
    }
}

/// As the settings write it, the modifiers in the order Ctrl, Shift, Alt, Meta: `Trigger::parse` reads it back.
impl std::fmt::Display for Trigger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.input {
            Input::Mouse(button) => write!(formatter, "{MOUSE_PREFIX}{button}"),
            Input::Key(key) => {
                for (held, name) in [
                    (self.modifiers.ctrl, "Ctrl"),
                    (self.modifiers.shift, "Shift"),
                    (self.modifiers.alt, "Alt"),
                    (self.modifiers.meta, "Meta"),
                ] {
                    if held {
                        write!(formatter, "{name}+")?;
                    }
                }
                write!(formatter, "{key}")
            }
        }
    }
}

fn is_key(name: &str) -> bool {
    let single_character = name.len() == 1 && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    single_character || NAMED_KEYS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_key() {
        let trigger = Trigger::parse("F9").unwrap();
        assert_eq!(trigger.input, Input::Key("F9".into()));
        assert_eq!(trigger.modifiers, Modifiers::default());
    }

    #[test]
    fn a_combination() {
        let trigger = Trigger::parse("Ctrl+Alt+Space").unwrap();
        assert!(trigger.modifiers.ctrl && trigger.modifiers.alt && !trigger.modifiers.shift);
        assert_eq!(trigger.input, Input::Key("Space".into()));
    }

    #[test]
    fn a_mouse_button() {
        assert_eq!(Trigger::parse("Mouse:X1").unwrap().input, Input::Mouse("X1".into()));
    }

    #[test]
    fn written_as_read() {
        for text in ["F9", "Ctrl+Shift+Alt+D", "Meta+Space", "Mouse:X2"] {
            assert_eq!(Trigger::parse(text).unwrap().to_string(), text);
        }
        assert_eq!(Trigger::parse("Alt+Ctrl+D").unwrap().to_string(), "Ctrl+Alt+D");
    }

    #[test]
    fn the_left_button_is_refused() {
        assert!(Trigger::parse("Mouse:Left").is_err());
    }

    #[test]
    fn unknown_names_are_refused() {
        for text in ["", "Ctrl+", "Hyper+A", "F25", "a", "Ctrl+Mouse:X1"] {
            assert!(Trigger::parse(text).is_err(), "{text:?}");
        }
    }
}
