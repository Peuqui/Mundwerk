//! Taking a trigger live: the settings window asks for one, and while it waits it is told what is held, so the
//! user sees the combination grow. The next key (with the modifiers held then) or mouse button becomes the trigger,
//! written as the settings write triggers. Goes through the system hook, so it sees what the window could not: a
//! combination a mouse's own software sends, the Windows key, a side button.

use serde::Serialize;

use crate::controller::{Event, Modifier, Press};
use crate::trigger::{Input, Modifiers, Trigger};

/// What a press means for a running capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "trigger", rename_all = "snake_case")]
pub enum Update {
    /// The modifiers held now (`"Ctrl+Shift"`, or `""` when none): the capture goes on.
    Held(String),
    /// A key no trigger can use (`"Ctrl+VK187"`): the capture goes on.
    Unusable(String),
    /// The trigger: the press is swallowed and the capture is over.
    Done(String),
}

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

    /// Every press of the hook goes through here first; `None` when no capture runs or the press means nothing
    /// to it.
    pub fn observe(&mut self, event: &Event) -> Option<Update> {
        let trigger = match event {
            Event::Modifier(modifier, press) => {
                let flag = match modifier {
                    Modifier::Ctrl => &mut self.modifiers.ctrl,
                    Modifier::Alt => &mut self.modifiers.alt,
                    Modifier::Shift => &mut self.modifiers.shift,
                    Modifier::Meta => &mut self.modifiers.meta,
                };
                *flag = *press == Press::Down;
                return self.active.then(|| Update::Held(self.modifiers.names().join("+")));
            }
            Event::Key(name, Press::Down) if self.active => {
                Trigger { modifiers: self.modifiers, input: Input::Key(name.clone()) }
            }
            Event::Mouse(name, Press::Down) if self.active => {
                Trigger { modifiers: Modifiers::default(), input: Input::Mouse(name.clone()) }
            }
            _ => return None,
        };
        let text = trigger.to_string();
        // Only what the settings can read back.
        if Trigger::parse(&text).is_err() {
            return Some(Update::Unusable(text));
        }
        self.active = false;
        Some(Update::Done(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(modifier: Modifier) -> Event {
        Event::Modifier(modifier, Press::Down)
    }

    fn done(text: &str) -> Update {
        Update::Done(text.into())
    }

    #[test]
    fn the_combination_grows_as_it_is_pressed() {
        let mut capture = Capture::default();
        capture.start();
        assert_eq!(capture.observe(&down(Modifier::Ctrl)), Some(Update::Held("Ctrl".into())));
        assert_eq!(capture.observe(&down(Modifier::Shift)), Some(Update::Held("Ctrl+Shift".into())));
        assert_eq!(capture.observe(&down(Modifier::Alt)), Some(Update::Held("Ctrl+Shift+Alt".into())));
        assert_eq!(capture.observe(&Event::Key("D".into(), Press::Down)), Some(done("Ctrl+Shift+Alt+D")));
        assert_eq!(capture.observe(&Event::Key("E".into(), Press::Down)), None, "the capture is over");
        assert_eq!(capture.observe(&Event::Modifier(Modifier::Alt, Press::Up)), None, "and says nothing more");
    }

    #[test]
    fn modifiers_held_before_the_start_count() {
        let mut capture = Capture::default();
        capture.observe(&down(Modifier::Ctrl));
        capture.start();
        assert_eq!(capture.observe(&Event::Key("D".into(), Press::Down)), Some(done("Ctrl+D")));
    }

    #[test]
    fn a_side_button() {
        let mut capture = Capture::default();
        capture.start();
        assert_eq!(capture.observe(&Event::Mouse("X1".into(), Press::Down)), Some(done("Mouse:X1")));
    }

    #[test]
    fn nothing_while_not_asked() {
        let mut capture = Capture::default();
        assert_eq!(capture.observe(&Event::Key("F9".into(), Press::Down)), None);
        assert_eq!(capture.observe(&down(Modifier::Ctrl)), None);
    }

    #[test]
    fn a_key_no_trigger_can_name_is_said_and_passed_over() {
        let mut capture = Capture::default();
        capture.start();
        assert_eq!(capture.observe(&Event::Key("VK187".into(), Press::Down)), Some(Update::Unusable("VK187".into())));
        assert_eq!(capture.observe(&Event::Key("F9".into(), Press::Down)), Some(done("F9")));
    }

    #[test]
    fn released_modifiers_do_not_count() {
        let mut capture = Capture::default();
        capture.start();
        capture.observe(&down(Modifier::Ctrl));
        assert_eq!(capture.observe(&Event::Modifier(Modifier::Ctrl, Press::Up)), Some(Update::Held(String::new())));
        assert_eq!(capture.observe(&Event::Key("D".into(), Press::Down)), Some(done("D")));
    }
}
