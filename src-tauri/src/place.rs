//! Where the overlay shows when it is to be next to where one writes: below and right of the text cursor, so it does
//! not cover the line. Many programs (browsers, Electron, others drawing their own text) do not tell Windows where
//! their text cursor is; then the mouse pointer stands for it, since one has just clicked into the field.

use tauri::{AppHandle, PhysicalPosition, PhysicalSize};

/// How far from the text cursor or pointer the overlay's corner is (physical pixels).
const GAP: i32 = 18;

/// The overlay's top left corner next to where one writes, kept on the screen that place is on.
pub fn near_text(app: &AppHandle, overlay_size: PhysicalSize<u32>) -> Option<PhysicalPosition<i32>> {
    let anchor = text_cursor().or_else(|| pointer(app))?;
    // Screen and window sizes in pixels fit an i32 by far.
    #[allow(clippy::cast_possible_wrap)]
    let (width, height) = (overlay_size.width as i32, overlay_size.height as i32);
    let mut x = anchor.x + GAP;
    let mut y = anchor.y + GAP;
    #[allow(clippy::cast_possible_truncation)] // screen coordinates
    if let Ok(Some(monitor)) = app.monitor_from_point(f64::from(anchor.x), f64::from(anchor.y)) {
        let origin = monitor.position();
        #[allow(clippy::cast_possible_wrap)]
        let (right, bottom) = (origin.x + monitor.size().width as i32, origin.y + monitor.size().height as i32);
        // Too far right or down: to the other side of the anchor, so it stays on this screen.
        if x + width > right {
            x = anchor.x - GAP - width;
        }
        if y + height > bottom {
            y = anchor.y - GAP - height;
        }
        x = x.clamp(origin.x, right - width);
        y = y.clamp(origin.y, bottom - height);
    }
    Some(PhysicalPosition::new(x, y))
}

#[allow(clippy::cast_possible_truncation)] // screen coordinates
fn pointer(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let position = app.cursor_position().ok()?;
    Some(PhysicalPosition::new(position.x.round() as i32, position.y.round() as i32))
}

/// The bottom left of the text cursor of the window in front, on the screen; `None` where the program does not
/// tell it.
#[cfg(windows)]
fn text_cursor() -> Option<PhysicalPosition<i32>> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
    };

    // SAFETY: plain queries about the window in front; the struct carries its own size as the API asks.
    unsafe {
        let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let mut info =
            GUITHREADINFO { cbSize: u32::try_from(std::mem::size_of::<GUITHREADINFO>()).ok()?, ..Default::default() };
        GetGUIThreadInfo(thread, &raw mut info).ok()?;
        if info.hwndCaret.is_invalid() {
            return None;
        }
        let mut corner = POINT { x: info.rcCaret.left, y: info.rcCaret.bottom };
        ClientToScreen(info.hwndCaret, &raw mut corner).as_bool().then(|| PhysicalPosition::new(corner.x, corner.y))
    }
}

#[cfg(not(windows))]
fn text_cursor() -> Option<PhysicalPosition<i32>> {
    None
}
