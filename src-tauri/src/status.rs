//! What Mundwerk is doing, shown to the user: the overlay (unless only the tray shows it), the tray icon and its
//! tooltip. An error stays for `overlay.error_shown_s`, then the state before it comes back.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use mundwerk_core::config::{OverlayMode, Placement};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::engine::{lock, Shared};
use crate::{badge, place, texts, tray, OVERLAY};

/// The event the overlay listens to.
pub const STATUS_EVENT: &str = "mundwerk://status";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "detail", rename_all = "snake_case")]
pub enum Status {
    Idle,
    Listening,
    Transcribing,
    Error(String),
}

#[derive(Clone)]
pub struct Reporter {
    app: AppHandle,
    // Each report counts up; the end of an error's time applies only if nothing was reported since.
    generation: Arc<AtomicU64>,
}

impl Reporter {
    pub fn new(app: AppHandle) -> Self {
        Self { app, generation: Arc::new(AtomicU64::new(0)) }
    }

    pub fn report(&self, status: Status) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        log::info!("status: {status:?}");
        if let Status::Error(message) = &status {
            log::error!("{message}");
            let shown =
                self.app.try_state::<Arc<Shared>>().map_or(0, |shared| lock(&shared.config).overlay.error_shown_s);
            let reporter = self.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(shown));
                if reporter.generation.load(Ordering::SeqCst) == generation {
                    // What is going on by now, which may be a new recording.
                    let now = reporter.app.try_state::<Arc<Shared>>().map_or(Status::Idle, |shared| shared.status());
                    reporter.report(now);
                }
            });
        }
        let app = self.app.clone();
        // Windows and tray belong to the main thread.
        let _ = self.app.run_on_main_thread(move || show(&app, &status));
    }
}

fn show(app: &AppHandle, status: &Status) {
    let Some(settings) = app.try_state::<Arc<Shared>>().map(|shared| lock(&shared.config).overlay.clone()) else {
        return;
    };
    let floating = settings.mode == OverlayMode::Floating;
    badge::show(app, status, settings.pointer);
    if let Some(overlay) = app.get_webview_window(OVERLAY) {
        if *status == Status::Listening && floating && settings.placement == Placement::NearText {
            let place = overlay.outer_size().ok().and_then(|size| place::near_text(app, size));
            if let Some(position) = place {
                if let Err(error) = overlay.set_position(position) {
                    log::error!("overlay position: {error}");
                }
            }
        }
        if let Err(error) = overlay.emit(STATUS_EVENT, status) {
            log::error!("overlay: {error}");
        }
        let visible = floating && *status != Status::Idle;
        if let Err(error) = if visible { overlay.show() } else { overlay.hide() } {
            log::error!("overlay show/hide: {error}");
        }
    }
    tray::show_status(app, status, &texts::status(status));
}
