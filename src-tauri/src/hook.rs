//! The system-wide keyboard and mouse hook. Every press goes to `handle`, which answers whether it is swallowed
//! (it then does not reach the program below). Runs on a thread of its own for the life of the app.

use mundwerk_core::controller::Event;

/// While set, the hook writes every raw press into the log (a running capture sets it): to see what a device
/// really sends, e.g. a mouse's macro.
pub static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Called for every press; `true` = swallow it. Must be quick: Windows drops a hook that keeps it waiting.
pub type Handler = Box<dyn Fn(&Event) -> bool + Send + Sync>;

/// Called for every move of the mouse pointer (screen pixels); as quick as [`Handler`].
pub type MoveHandler = Box<dyn Fn(i32, i32) + Send + Sync>;

#[cfg(windows)]
pub use windows_hook::run;

#[cfg(not(windows))]
pub use rdev_hook::run;

#[cfg(windows)]
mod windows_hook {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;

    use mundwerk_core::controller::{Event, Press};
    use mundwerk_core::windows_keys::{key_event, x_button};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_SYSKEYDOWN,
        WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    use super::{Handler, MoveHandler, TRACE};

    // The hook procedures are plain functions; they reach the handlers through these.
    static HANDLER: OnceLock<Handler> = OnceLock::new();
    static MOVE_HANDLER: OnceLock<MoveHandler> = OnceLock::new();
    // Set at the first key the hook sees: the log then says once that keys arrive at all.
    static KEYS_ARRIVE: AtomicBool = AtomicBool::new(false);

    /// Install the hooks and pump messages for them (low-level hooks are called on the thread that installed
    /// them, while it waits for messages). Does not return while the app runs.
    pub fn run(handler: Handler, on_move: MoveHandler) -> Result<(), String> {
        HANDLER.set(handler).map_err(|_| "the hook runs already".to_owned())?;
        MOVE_HANDLER.set(on_move).map_err(|_| "the hook runs already".to_owned())?;
        // SAFETY: the procedures are valid for the life of the program; no module handle for low-level hooks.
        unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), None, 0).map_err(|error| error.to_string())?;
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), None, 0).map_err(|error| error.to_string())?;
            log::info!("keyboard and mouse hook installed");
            let mut message = MSG::default();
            while GetMessageW(&raw mut message, None, 0, 0).as_bool() {}
        }
        Ok(())
    }

    /// Typed by Mundwerk itself (enigo marks everything it sends). Not "every injected press": a mouse's own
    /// software sends its key combinations that way too, and those must count.
    fn is_own(extra_info: usize) -> bool {
        extra_info == enigo::EVENT_MARKER as usize
    }

    fn swallowed(event: &Event) -> bool {
        HANDLER.get().is_some_and(|handler| handler(event))
    }

    unsafe extern "system" fn keyboard(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code == HC_ACTION.cast_signed() {
            // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lParam points to a KBDLLHOOKSTRUCT.
            let info = unsafe { &*(data.0 as *const KBDLLHOOKSTRUCT) };
            let own = is_own(info.dwExtraInfo);
            if !KEYS_ARRIVE.swap(true, Ordering::Relaxed) {
                log::info!("hook: the first key arrived (vk={:#x})", info.vkCode);
            }
            if TRACE.load(Ordering::Relaxed) {
                log::info!(
                    "hook: key message={:#x} vk={:#x} flags={:#x} extra={:#x} own={own}",
                    message.0,
                    info.vkCode,
                    info.flags.0,
                    info.dwExtraInfo
                );
            }
            #[allow(clippy::cast_possible_truncation)] // window messages fit in u32
            let press = match message.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(Press::Down),
                WM_KEYUP | WM_SYSKEYUP => Some(Press::Up),
                _ => None,
            };
            if let (false, Some(press)) = (own, press) {
                if swallowed(&key_event(info.vkCode, press)) {
                    return LRESULT(1);
                }
            }
        }
        // SAFETY: passing the event on, as every hook must that does not swallow it.
        unsafe { CallNextHookEx(None, code, message, data) }
    }

    unsafe extern "system" fn mouse(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code == HC_ACTION.cast_signed() {
            // SAFETY: for WH_MOUSE_LL with HC_ACTION, lParam points to an MSLLHOOKSTRUCT.
            let info = unsafe { &*(data.0 as *const MSLLHOOKSTRUCT) };
            let own = is_own(info.dwExtraInfo);
            #[allow(clippy::cast_possible_truncation)] // window messages fit in u32
            if message.0 as u32 == WM_MOUSEMOVE {
                if let Some(on_move) = MOVE_HANDLER.get() {
                    on_move(info.pt.x, info.pt.y);
                }
            }
            #[allow(clippy::cast_possible_truncation)] // window messages fit in u32
            if TRACE.load(Ordering::Relaxed) && message.0 as u32 != WM_MOUSEMOVE {
                log::info!(
                    "hook: mouse message={:#x} data={:#x} flags={:#x} extra={:#x} own={own}",
                    message.0,
                    info.mouseData,
                    info.flags,
                    info.dwExtraInfo
                );
            }
            // The high word of mouseData says which X button.
            #[allow(clippy::cast_possible_truncation)]
            let high_word = (info.mouseData >> 16) as u16;
            #[allow(clippy::cast_possible_truncation)]
            let event = match message.0 as u32 {
                WM_MBUTTONDOWN => Some(Event::Mouse("Middle".into(), Press::Down)),
                WM_MBUTTONUP => Some(Event::Mouse("Middle".into(), Press::Up)),
                WM_XBUTTONDOWN => Some(Event::Mouse(x_button(high_word), Press::Down)),
                WM_XBUTTONUP => Some(Event::Mouse(x_button(high_word), Press::Up)),
                _ => None,
            };
            if let (false, Some(event)) = (own, event) {
                if swallowed(&event) {
                    return LRESULT(1);
                }
            }
        }
        // SAFETY: as above.
        unsafe { CallNextHookEx(None, code, message, data) }
    }
}

#[cfg(not(windows))]
mod rdev_hook {
    use mundwerk_core::controller::{Event, Modifier, Press};
    use rdev::{Button, EventType, Key};

    use super::{Handler, MoveHandler};

    /// rdev's grab (macOS: accessibility permission; Linux: read access to /dev/input). Does not return while the
    /// app runs.
    pub fn run(handler: Handler, on_move: MoveHandler) -> Result<(), String> {
        rdev::grab(move |event| {
            if let EventType::MouseMove { x, y } = event.event_type {
                #[allow(clippy::cast_possible_truncation)] // screen coordinates
                on_move(x.round() as i32, y.round() as i32);
            }
            match translate(&event.event_type) {
                Some(translated) if handler(&translated) => None,
                _ => Some(event),
            }
        })
        .map_err(|error| format!("{error:?}"))
    }

    fn translate(event: &EventType) -> Option<Event> {
        match event {
            EventType::KeyPress(key) => Some(key_event(*key, Press::Down)),
            EventType::KeyRelease(key) => Some(key_event(*key, Press::Up)),
            EventType::ButtonPress(button) => button_name(*button).map(|name| Event::Mouse(name, Press::Down)),
            EventType::ButtonRelease(button) => button_name(*button).map(|name| Event::Mouse(name, Press::Up)),
            _ => None,
        }
    }

    fn button_name(button: Button) -> Option<String> {
        match button {
            Button::Middle => Some("Middle".into()),
            // X11 numbers the side buttons 8 and 9.
            Button::Unknown(8) => Some("X1".into()),
            Button::Unknown(9) => Some("X2".into()),
            _ => None,
        }
    }

    fn key_event(key: Key, press: Press) -> Event {
        let modifier = match key {
            Key::ControlLeft | Key::ControlRight => Some(Modifier::Ctrl),
            Key::Alt | Key::AltGr => Some(Modifier::Alt),
            Key::ShiftLeft | Key::ShiftRight => Some(Modifier::Shift),
            Key::MetaLeft | Key::MetaRight => Some(Modifier::Meta),
            _ => None,
        };
        match modifier {
            Some(modifier) => Event::Modifier(modifier, press),
            None => Event::Key(key_name(key), press),
        }
    }

    fn key_name(key: Key) -> String {
        let name = match key {
            Key::Space => "Space",
            Key::Return => "Enter",
            Key::Tab => "Tab",
            Key::Escape => "Escape",
            Key::Backspace => "Backspace",
            Key::Insert => "Insert",
            Key::Delete => "Delete",
            Key::Home => "Home",
            Key::End => "End",
            Key::PageUp => "PageUp",
            Key::PageDown => "PageDown",
            Key::UpArrow => "Up",
            Key::DownArrow => "Down",
            Key::LeftArrow => "Left",
            Key::RightArrow => "Right",
            Key::Pause => "Pause",
            Key::ScrollLock => "ScrollLock",
            Key::CapsLock => "CapsLock",
            Key::PrintScreen => "PrintScreen",
            Key::F1 => "F1",
            Key::F2 => "F2",
            Key::F3 => "F3",
            Key::F4 => "F4",
            Key::F5 => "F5",
            Key::F6 => "F6",
            Key::F7 => "F7",
            Key::F8 => "F8",
            Key::F9 => "F9",
            Key::F10 => "F10",
            Key::F11 => "F11",
            Key::F12 => "F12",
            // Letters and digits: rdev's names (KeyA, Num1) without the prefix.
            other => {
                let debug = format!("{other:?}");
                return debug
                    .strip_prefix("Key")
                    .or_else(|| debug.strip_prefix("Num"))
                    .filter(|rest| rest.len() == 1)
                    .map_or(debug.clone(), str::to_owned);
            }
        };
        name.to_owned()
    }
}
