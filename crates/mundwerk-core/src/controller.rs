//! When a dictation starts and ends. The app feeds every key and mouse button of the system hook in as names
//! (`"F9"`, `"X2"`, the modifiers on their own); the controller says what to do and whether the press is swallowed
//! (it then does not reach the program below). Hold: records while held, a slip shorter than `min_press_ms` is
//! thrown away. Toggle: one press starts, the next one of the same trigger stops. The cancel key throws a running
//! recording away. Keyboard auto-repeat (a second "down" without an "up") changes nothing.

use std::time::{Duration, Instant};

use crate::config::{Config, TriggerMode};
use crate::trigger::{Input, Modifiers, Trigger};
use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A key by the names of [`Trigger::parse`] (`"F9"`, `"A"`, `"Space"`); a key without such a name too, by any
    /// name: it matches no trigger but still counts.
    Key(String, Press),
    Modifier(Modifier, Press),
    /// `"Middle"`, `"X1"`, `"X2"`, or any other name for a button no trigger can use.
    Mouse(String, Press),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Nothing,
    Start,
    /// The recording is done: transcribe it.
    Stop,
    /// Held shorter than `min_press_ms`: a slip, the recording is thrown away without a sound.
    Discard,
    /// The cancel key: the recording is thrown away.
    Cancel,
    /// The key for the last dictation once more: write it again where the cursor is.
    InsertAgain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub action: Action,
    /// The press does not reach the program below.
    pub swallow: bool,
}

const PASS: Outcome = Outcome { action: Action::Nothing, swallow: false };

struct Armed {
    trigger: Trigger,
    mode: TriggerMode,
    pass_through: bool,
    /// Its "down" was taken (so auto-repeat and the "up" belong to it as well).
    held: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Recording { trigger: usize, since: Instant },
}

pub struct Controller {
    armed: Vec<Armed>,
    cancel: Trigger,
    /// The key for the last dictation once more, and whether its "down" was taken (its "up" then is too).
    again: Option<(Trigger, bool)>,
    min_press: Duration,
    modifiers: Modifiers,
    state: State,
}

impl Controller {
    /// # Errors
    /// A trigger or the cancel key cannot be read (the settings were not validated).
    pub fn new(config: &Config) -> Result<Self, Error> {
        let armed = config
            .triggers
            .iter()
            .map(|setting| {
                Ok(Armed {
                    trigger: Trigger::parse(&setting.input)?,
                    mode: setting.mode,
                    pass_through: setting.pass_through,
                    held: false,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            armed,
            cancel: Trigger::parse(&config.recording.cancel_key)?,
            again: match config.insert.again_key.as_str() {
                "" => None,
                key => Some((Trigger::parse(key)?, false)),
            },
            min_press: Duration::from_millis(config.recording.min_press_ms),
            modifiers: Modifiers::default(),
            state: State::Idle,
        })
    }

    /// The modifiers held now, as the hook saw them (a mouse's macro included). Typing waits until none is: a
    /// letter typed while Ctrl is held arrives as a shortcut (a console shows `^T`).
    #[must_use]
    pub fn held_modifiers(&self) -> Modifiers {
        self.modifiers
    }

    #[must_use]
    pub fn is_recording(&self) -> bool {
        matches!(self.state, State::Recording { .. })
    }

    pub fn handle(&mut self, event: &Event, now: Instant) -> Outcome {
        match event {
            Event::Modifier(modifier, press) => {
                self.set_modifier(*modifier, *press == Press::Down);
                PASS
            }
            Event::Key(name, press) => self.input(&Input::Key(name.clone()), *press, now),
            Event::Mouse(name, press) => self.input(&Input::Mouse(name.clone()), *press, now),
        }
    }

    /// Stop the recording from outside a press: silence or the maximum length (the app's timers).
    pub fn finish(&mut self) -> Action {
        if self.is_recording() {
            self.state = State::Idle;
            Action::Stop
        } else {
            Action::Nothing
        }
    }

    fn input(&mut self, input: &Input, press: Press, now: Instant) -> Outcome {
        if press == Press::Down && self.is_recording() && self.matches(&self.cancel, input) {
            self.state = State::Idle;
            return Outcome { action: Action::Cancel, swallow: true };
        }
        if let Some(outcome) = self.again(input, press) {
            return outcome;
        }
        let Some(index) = self.armed_for(input, press) else { return PASS };
        let swallow = !self.armed[index].pass_through;
        let action = match press {
            Press::Down if self.armed[index].held => Action::Nothing,
            Press::Down => {
                self.armed[index].held = true;
                self.down(index, now)
            }
            Press::Up => {
                self.armed[index].held = false;
                self.up(index, now)
            }
        };
        Outcome { action, swallow }
    }

    /// The key for the last dictation once more: its "down" (not while recording) and the "up" that belongs to it.
    fn again(&mut self, input: &Input, press: Press) -> Option<Outcome> {
        let recording = self.is_recording();
        let modifiers = self.modifiers;
        let (trigger, held) = self.again.as_mut()?;
        match press {
            Press::Down if !recording && trigger.input == *input && trigger.modifiers == modifiers => {
                let first = !*held;
                *held = true;
                Some(Outcome { action: if first { Action::InsertAgain } else { Action::Nothing }, swallow: true })
            }
            Press::Up if *held && trigger.input == *input => {
                *held = false;
                Some(Outcome { action: Action::Nothing, swallow: true })
            }
            _ => None,
        }
    }

    /// The trigger this press belongs to: a "down" by its key and modifiers, an "up" to the trigger that took
    /// the "down" (its modifiers may already be up).
    fn armed_for(&self, input: &Input, press: Press) -> Option<usize> {
        self.armed.iter().position(|armed| match press {
            Press::Down => armed.held && armed.trigger.input == *input || self.matches(&armed.trigger, input),
            Press::Up => armed.held && armed.trigger.input == *input,
        })
    }

    fn matches(&self, trigger: &Trigger, input: &Input) -> bool {
        // A mouse button has no modifiers (Trigger::parse allows none): it works whatever is held.
        trigger.input == *input && (matches!(input, Input::Mouse(_)) || trigger.modifiers == self.modifiers)
    }

    fn down(&mut self, index: usize, now: Instant) -> Action {
        match self.state {
            State::Idle => {
                self.state = State::Recording { trigger: index, since: now };
                Action::Start
            }
            State::Recording { trigger, .. } if trigger == index && self.armed[index].mode == TriggerMode::Toggle => {
                self.state = State::Idle;
                Action::Stop
            }
            // Another trigger while one records: it is not its recording.
            State::Recording { .. } => Action::Nothing,
        }
    }

    fn up(&mut self, index: usize, now: Instant) -> Action {
        match self.state {
            State::Recording { trigger, since } if trigger == index && self.armed[index].mode == TriggerMode::Hold => {
                self.state = State::Idle;
                if now.duration_since(since) < self.min_press {
                    Action::Discard
                } else {
                    Action::Stop
                }
            }
            _ => Action::Nothing,
        }
    }

    fn set_modifier(&mut self, modifier: Modifier, down: bool) {
        let flag = match modifier {
            Modifier::Ctrl => &mut self.modifiers.ctrl,
            Modifier::Alt => &mut self.modifiers.alt,
            Modifier::Shift => &mut self.modifiers.shift,
            Modifier::Meta => &mut self.modifiers.meta,
        };
        *flag = down;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_CONFIG;

    /// The defaults: Mouse:X2 hold, F9 toggle, both swallowed; Escape cancels; 300 ms minimum.
    fn controller() -> Controller {
        Controller::new(&Config::parse(DEFAULT_CONFIG).unwrap()).unwrap()
    }

    fn key(name: &str, press: Press) -> Event {
        Event::Key(name.into(), press)
    }

    fn mouse(name: &str, press: Press) -> Event {
        Event::Mouse(name.into(), press)
    }

    #[test]
    fn the_modifiers_held_are_known_until_let_go() {
        let mut controller = controller();
        let now = Instant::now();
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Down), now);
        controller.handle(&Event::Modifier(Modifier::Shift, Press::Down), now);
        assert_eq!(controller.held_modifiers().names(), ["Ctrl", "Shift"]);
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Up), now);
        controller.handle(&Event::Modifier(Modifier::Shift, Press::Up), now);
        assert_eq!(controller.held_modifiers(), Modifiers::default());
    }

    const LONG: Duration = Duration::from_millis(800);
    const SHORT: Duration = Duration::from_millis(100);

    #[test]
    fn hold_records_while_held() {
        let mut controller = controller();
        let start = Instant::now();
        assert_eq!(
            controller.handle(&mouse("X2", Press::Down), start),
            Outcome { action: Action::Start, swallow: true }
        );
        assert!(controller.is_recording());
        assert_eq!(controller.handle(&mouse("X2", Press::Up), start + LONG).action, Action::Stop);
        assert!(!controller.is_recording());
    }

    #[test]
    fn a_short_hold_is_a_slip() {
        let mut controller = controller();
        let start = Instant::now();
        controller.handle(&mouse("X2", Press::Down), start);
        assert_eq!(controller.handle(&mouse("X2", Press::Up), start + SHORT).action, Action::Discard);
    }

    #[test]
    fn toggle_starts_and_stops_with_presses() {
        let mut controller = controller();
        let start = Instant::now();
        assert_eq!(controller.handle(&key("F9", Press::Down), start).action, Action::Start);
        assert_eq!(controller.handle(&key("F9", Press::Up), start + SHORT).action, Action::Nothing);
        assert!(controller.is_recording());
        assert_eq!(controller.handle(&key("F9", Press::Down), start + LONG).action, Action::Stop);
        assert_eq!(controller.handle(&key("F9", Press::Up), start + LONG).action, Action::Nothing);
    }

    #[test]
    fn auto_repeat_changes_nothing() {
        let mut controller = controller();
        let start = Instant::now();
        controller.handle(&key("F9", Press::Down), start);
        assert_eq!(
            controller.handle(&key("F9", Press::Down), start + SHORT),
            Outcome { action: Action::Nothing, swallow: true }
        );
        assert!(controller.is_recording());
    }

    #[test]
    fn escape_cancels_only_a_recording() {
        let mut controller = controller();
        let start = Instant::now();
        assert_eq!(controller.handle(&key("Escape", Press::Down), start), PASS);
        controller.handle(&key("F9", Press::Down), start);
        assert_eq!(
            controller.handle(&key("Escape", Press::Down), start + LONG),
            Outcome { action: Action::Cancel, swallow: true }
        );
        assert!(!controller.is_recording());
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let mut controller = controller();
        let now = Instant::now();
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Down), now);
        assert_eq!(controller.handle(&key("F9", Press::Down), now), PASS);
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Up), now);
        assert_eq!(controller.handle(&key("F9", Press::Down), now).action, Action::Start);
    }

    #[test]
    fn a_combination_ends_even_if_its_modifier_is_released_first() {
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.triggers[1].input = "Ctrl+Space".into();
        config.triggers[1].mode = TriggerMode::Hold;
        let mut controller = Controller::new(&config).unwrap();
        let start = Instant::now();
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Down), start);
        assert_eq!(controller.handle(&key("Space", Press::Down), start).action, Action::Start);
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Up), start + LONG);
        assert_eq!(
            controller.handle(&key("Space", Press::Up), start + LONG),
            Outcome { action: Action::Stop, swallow: true }
        );
    }

    #[test]
    fn a_mouse_trigger_works_with_a_modifier_held() {
        let mut controller = controller();
        let now = Instant::now();
        controller.handle(&Event::Modifier(Modifier::Shift, Press::Down), now);
        assert_eq!(controller.handle(&mouse("X2", Press::Down), now).action, Action::Start);
    }

    #[test]
    fn another_trigger_does_not_end_a_recording() {
        let mut controller = controller();
        let start = Instant::now();
        controller.handle(&key("F9", Press::Down), start);
        controller.handle(&key("F9", Press::Up), start);
        assert_eq!(controller.handle(&mouse("X2", Press::Down), start + LONG).action, Action::Nothing);
        assert_eq!(controller.handle(&mouse("X2", Press::Up), start + LONG).action, Action::Nothing);
        assert!(controller.is_recording());
    }

    #[test]
    fn pass_through_lets_the_press_reach_the_program() {
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.triggers[0].pass_through = true;
        let mut controller = Controller::new(&config).unwrap();
        assert_eq!(
            controller.handle(&mouse("X2", Press::Down), Instant::now()),
            Outcome { action: Action::Start, swallow: false }
        );
    }

    #[test]
    fn other_keys_pass() {
        let mut controller = controller();
        assert_eq!(controller.handle(&key("A", Press::Down), Instant::now()), PASS);
        assert_eq!(controller.handle(&mouse("Left", Press::Down), Instant::now()), PASS);
    }

    fn with_again_key(key: &str) -> Controller {
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.insert.again_key = key.into();
        Controller::new(&config).unwrap()
    }

    #[test]
    fn the_again_key_writes_the_last_dictation_once_per_press() {
        let mut controller = with_again_key("Ctrl+Shift+V");
        let now = Instant::now();
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Down), now);
        controller.handle(&Event::Modifier(Modifier::Shift, Press::Down), now);
        assert_eq!(
            controller.handle(&key("V", Press::Down), now),
            Outcome { action: Action::InsertAgain, swallow: true }
        );
        assert_eq!(controller.handle(&key("V", Press::Down), now).action, Action::Nothing, "auto-repeat");
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Up), now);
        assert_eq!(controller.handle(&key("V", Press::Up), now), Outcome { action: Action::Nothing, swallow: true });
        assert_eq!(controller.handle(&key("V", Press::Up), now), PASS, "a plain V is the program's again");
    }

    #[test]
    fn the_again_key_does_nothing_while_recording_and_needs_its_modifiers() {
        let mut controller = with_again_key("Ctrl+Shift+V");
        let now = Instant::now();
        assert_eq!(controller.handle(&key("V", Press::Down), now), PASS);
        controller.handle(&key("F9", Press::Down), now);
        controller.handle(&Event::Modifier(Modifier::Ctrl, Press::Down), now);
        controller.handle(&Event::Modifier(Modifier::Shift, Press::Down), now);
        assert_eq!(controller.handle(&key("V", Press::Down), now), PASS);
    }

    #[test]
    fn finish_stops_a_recording_from_a_timer() {
        let mut controller = controller();
        assert_eq!(controller.finish(), Action::Nothing);
        controller.handle(&key("F9", Press::Down), Instant::now());
        assert_eq!(controller.finish(), Action::Stop);
        assert!(!controller.is_recording());
    }
}
