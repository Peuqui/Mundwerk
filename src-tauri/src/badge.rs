//! Agent-Orc's dictation badge (the STT-Suite's logo) by the mouse pointer while a dictation runs, above and right of
//! the pointer's tip as Agent-Orc places it (a tooltip shows below the pointer), as `SpeechPulse` shows one: its bars move while Mundwerk listens, it stands still and grey while it
//! transcribes (`ui/badge.html`). A small window that takes no clicks and no focus; the system hook reports the
//! pointer's moves, and the window follows them while it shows.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::status::Status;
use crate::texts;

pub const BADGE: &str = "badge";
const PAGE: &str = "badge.html";
/// The window as small as it goes; Windows keeps it at its minimum size (about 136 x 39), and the page draws the
/// badge at 28 px in its top left corner, so the corner is where the badge goes (logical pixels).
const SIZE: LogicalSize<f64> = LogicalSize::new(28.0, 28.0);
/// Above and right of the pointer's tip, exactly as Agent-Orc places it (its DictationPointer.vue; logical pixels).
const OFFSET: LogicalPosition<f64> = LogicalPosition::new(14.0, -38.0);

// While the badge shows, the hook's moves move it.
static FOLLOWING: AtomicBool = AtomicBool::new(false);
// The newest pointer position, and whether a move of the window is already on its way to the main thread: moves
// arrive far faster than a window moves, so only the newest is carried out.
static X: AtomicI32 = AtomicI32::new(0);
static Y: AtomicI32 = AtomicI32::new(0);
static MOVE_QUEUED: AtomicBool = AtomicBool::new(false);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let badge = WebviewWindowBuilder::new(app, BADGE, WebviewUrl::App(PAGE.into()))
        .title(texts::tr("app"))
        .inner_size(SIZE.width, SIZE.height)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()?;
    // Clicks go through to what is below.
    badge.set_ignore_cursor_events(true)
}

/// Show it while listening or transcribing (if the settings want it), else hide it.
pub fn show(app: &AppHandle, status: &Status, wanted: bool) {
    let Some(badge) = app.get_webview_window(BADGE) else { return };
    let showing = wanted && matches!(status, Status::Listening | Status::Transcribing);
    if showing && !FOLLOWING.load(Ordering::SeqCst) {
        if let Ok(pointer) = app.cursor_position() {
            #[allow(clippy::cast_possible_truncation)] // screen coordinates
            let (x, y) = (pointer.x.round() as i32, pointer.y.round() as i32);
            X.store(x, Ordering::SeqCst);
            Y.store(y, Ordering::SeqCst);
            place(&badge);
        }
    }
    FOLLOWING.store(showing, Ordering::SeqCst);
    let shown = if showing { badge.show() } else { badge.hide() };
    if let Err(error) = shown {
        log::error!("badge show/hide: {error}");
    }
}

/// The hook's report of a pointer move (screen pixels); cheap while the badge does not show.
pub fn moved(app: &AppHandle, x: i32, y: i32) {
    if !FOLLOWING.load(Ordering::Relaxed) {
        return;
    }
    X.store(x, Ordering::Relaxed);
    Y.store(y, Ordering::Relaxed);
    if MOVE_QUEUED.swap(true, Ordering::AcqRel) {
        return;
    }
    let for_main = app.clone();
    // Windows are moved on the main thread.
    let queued = app.run_on_main_thread(move || {
        MOVE_QUEUED.store(false, Ordering::Release);
        if let Some(badge) = for_main.get_webview_window(BADGE) {
            place(&badge);
        }
    });
    if queued.is_err() {
        MOVE_QUEUED.store(false, Ordering::Release);
    }
}

fn place(badge: &tauri::WebviewWindow) {
    let scale = match badge.scale_factor() {
        Ok(scale) => scale,
        Err(error) => {
            log::error!("badge scale: {error}");
            return;
        }
    };
    let offset: PhysicalPosition<i32> = OFFSET.to_physical(scale);
    let position = PhysicalPosition::new(X.load(Ordering::Relaxed) + offset.x, Y.load(Ordering::Relaxed) + offset.y);
    if let Err(error) = badge.set_position(position) {
        log::error!("badge position: {error}");
    }
}
