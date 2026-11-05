//! Taking a trigger live: the settings window asks for one, the next key (with the modifiers held then) or mouse
//! button becomes it, written as the settings write triggers. Goes through the system hook, so it sees what the
//! window could not: a combination a mouse's own software sends, the Windows key, a side button.

use crate::controller::{Event, Modifier, Press};
use crate::trigger::{Input, Modifiers, Trigger};

#[derive(Debug, Default)]
pub struct Capture {
    // Kept up to date also while not capturing, so a modifier held before the start counts.
    modifiers: Modifiers,
    active: bool,
}

impl Capture {
    pub fn start(&mut self) {
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Every press of the hook goes through here first. `Some(trigger)` when it completes a capture: the press
    /// is then swallowed and the capture is over. A key or button no trigger can use is passed over.
    pub fn observe(&mut self, event: &Event) -> Option<String> {
        let input = match event {
            Event::Modifier(modifier, press) => {
                let flag = match modifier {
                    Modifier::Ctrl => &mut self.modifiers.ctrl,
                    Modifier::Alt => &mut self.modifiers.alt,
                    Modifier::Shift => &mut self.modifiers.shift,
                    Modifier::Meta => &mut self.modifiers.meta,
                };
                *flag = *press == Press::Down;
                return None;
            }
            Event::Key(name, Press::Down) if self.active => {
                Trigger { modifiers: self.modifiers, input: Input::Key(name.clone()) }
            }
            Event::Mouse(name, Press::Down) if self.active => {
                Trigger { modifiers: Modifiers::default(), input: Input::Mouse(name.clone()) }
            }
            _ => return None,
        };
        let text = input.to_string();
        // Only what the settings can read back.
        Trigger::parse(&text).ok()?;
        self.active = false;
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(modifier: Modifier) -> Event {
        Event::Modifier(modifier, Press::Down)
    }

    #[test]
    fn a_combination_with_modifiers_held_before_the_start() {
        let mut capture = Capture::default();
        for modifier in [Modifier::Ctrl, Modifier::Shift, Modifier::Alt] {
            capture.observe(&down(modifier));
        }
        capture.start();
        assert_eq!(capture.observe(&Event::Key("D".into(), Press::Down)).as_deref(), Some("Ctrl+Shift+Alt+D"));
        assert_eq!(capture.observe(&Event::Key("E".into(), Press::Down)), None, "the capture is over");
    }

    #[test]
    fn a_side_button() {
        let mut capture = Capture::default();
        capture.start();
        assert_eq!(capture.observe(&Event::Mouse("X1".into(), Press::Down)).as_deref(), Some("Mouse:X1"));
    }

    #[test]
    fn nothing_while_not_asked() {
        let mut capture = Capture::default();
        assert_eq!(capture.observe(&Event::Key("F9".into(), Press::Down)), None);
    }

    #[test]
    fn a_key_no_trigger_can_name_is_passed_over() {
        let mut capture = Capture::default();
        capture.start();
        assert_eq!(capture.observe(&Event::Key("VK187".into(), Press::Down)), None);
        assert_eq!(capture.observe(&Event::Key("F9".into(), Press::Down)).as_deref(), Some("F9"));
    }

    #[test]
    fn released_modifiers_do_not_count() {
        let mut capture = Capture::default();
        capture.observe(&down(Modifier::Ctrl));
        capture.observe(&Event::Modifier(Modifier::Ctrl, Press::Up));
        capture.start();
        assert_eq!(capture.observe(&Event::Key("D".into(), Press::Down)).as_deref(), Some("D"));
    }
}
