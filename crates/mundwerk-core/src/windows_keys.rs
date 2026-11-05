//! Windows virtual-key codes as the controller's events: the names of [`crate::trigger::Trigger::parse`] and the
//! modifiers. A pure table, so it is tested on any machine; the Windows hook only looks the code up.

use crate::controller::{Event, Modifier, Press};

/// The event for virtual-key `code` going `press`.
#[must_use]
pub fn key_event(code: u32, press: Press) -> Event {
    if let Some(modifier) = modifier(code) {
        return Event::Modifier(modifier, press);
    }
    Event::Key(key_name(code), press)
}

fn modifier(code: u32) -> Option<Modifier> {
    match code {
        // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
        0x10 | 0xA0 | 0xA1 => Some(Modifier::Shift),
        // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
        0x11 | 0xA2 | 0xA3 => Some(Modifier::Ctrl),
        // VK_MENU, VK_LMENU, VK_RMENU (Alt; AltGr arrives as Ctrl + Alt, as Windows sends it)
        0x12 | 0xA4 | 0xA5 => Some(Modifier::Alt),
        // VK_LWIN, VK_RWIN
        0x5B | 0x5C => Some(Modifier::Meta),
        _ => None,
    }
}

/// The trigger name of a key, or `"VK<code>"` for one no trigger can name (it still counts as a key).
fn key_name(code: u32) -> String {
    let named = match code {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x13 => "Pause",
        0x14 => "CapsLock",
        0x1B => "Escape",
        0x20 => "Space",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2C => "PrintScreen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x91 => "ScrollLock",
        // Digits and letters: the code is the ASCII character.
        0x30..=0x39 | 0x41..=0x5A => return char::from_u32(code).map_or_else(String::new, String::from),
        // VK_F1 .. VK_F24
        0x70..=0x87 => return format!("F{}", code - 0x70 + 1),
        _ => return format!("VK{code}"),
    };
    named.to_owned()
}

/// The mouse button of a `WM_XBUTTON*` message (the high word of `mouseData`: 1 or 2).
#[must_use]
pub fn x_button(high_word: u16) -> String {
    format!("X{high_word}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::{Input, Trigger};

    fn key(code: u32) -> String {
        match key_event(code, Press::Down) {
            Event::Key(name, _) => name,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn letters_digits_and_function_keys() {
        assert_eq!(key(0x41), "A");
        assert_eq!(key(0x5A), "Z");
        assert_eq!(key(0x39), "9");
        assert_eq!(key(0x70), "F1");
        assert_eq!(key(0x78), "F9");
        assert_eq!(key(0x87), "F24");
    }

    #[test]
    fn every_name_is_one_a_trigger_can_use() {
        for code in (0x08..=0x91).filter(|&code| modifier(code).is_none()) {
            let name = key(code);
            if !name.starts_with("VK") {
                let trigger = Trigger::parse(&name).unwrap_or_else(|error| panic!("{code:#x} {name}: {error}"));
                assert_eq!(trigger.input, Input::Key(name));
            }
        }
    }

    #[test]
    fn modifiers_left_right_and_generic() {
        assert_eq!(key_event(0xA2, Press::Down), Event::Modifier(Modifier::Ctrl, Press::Down));
        assert_eq!(key_event(0x11, Press::Up), Event::Modifier(Modifier::Ctrl, Press::Up));
        assert_eq!(key_event(0xA5, Press::Down), Event::Modifier(Modifier::Alt, Press::Down));
        assert_eq!(key_event(0x5C, Press::Down), Event::Modifier(Modifier::Meta, Press::Down));
    }

    #[test]
    fn an_unnamed_key() {
        // VK_OEM_PLUS
        assert_eq!(key(0xBB), "VK187");
    }

    #[test]
    fn side_buttons() {
        assert_eq!(x_button(1), "X1");
        assert_eq!(x_button(2), "X2");
        assert!(Trigger::parse(&format!("Mouse:{}", x_button(2))).is_ok());
    }
}
