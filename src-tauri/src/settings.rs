//! The settings window and what changes the settings: a form over the settings (saving checks them, writes them
//! into the commented file and puts them into effect at once), taking a trigger live through the system hook, the
//! tray's overlay switch, starting with the system, and where the overlay was left (a file of its own: it is a
//! window's place, not a setting).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};
use mundwerk_core::capture::Capture;
use mundwerk_core::config::{Config, OverlayMode, DEFAULT_CONFIG};
use mundwerk_core::i18n::{Language, LanguageSetting};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

use crate::engine::{lock, Shared};
use crate::status::{Reporter, Status};
use crate::{texts, Startup};

pub const SETTINGS: &str = "settings";
/// The event that tells the settings window how a live capture goes (`mundwerk_core::capture::Update`).
pub const CAPTURE_EVENT: &str = "mundwerk://capture";
const SETTINGS_PAGE: &str = "settings.html";
const POSITION_FILE: &str = "overlay-position.json";
/// The overlay is saved once it has stood still this long (moving it reports every pixel).
const POSITION_SETTLED: Duration = Duration::from_millis(800);

/// The live capture of a trigger, shared by the hook and the settings window's commands.
pub type CaptureState = Arc<Mutex<Capture>>;

pub fn open(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(SETTINGS_PAGE.into()))
        .title(texts::tr("settings.title"))
        .inner_size(820.0, 860.0)
        .build();
    match built {
        // A capture the window started ends with it, or the next key somewhere would be taken.
        Ok(window) => {
            let app = app.clone();
            window.on_window_event(move |event| {
                if matches!(event, WindowEvent::Destroyed) {
                    if let Some(capture) = app.try_state::<CaptureState>() {
                        lock(&capture).stop();
                    }
                }
            });
        }
        Err(error) => log::error!("settings window: {error}"),
    }
}

/// What the settings window shows: the file, and the settings in it or why they cannot be used.
#[derive(Serialize)]
pub struct Loaded {
    path: String,
    settings: Option<Config>,
    error: Option<String>,
}

// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn load_settings(startup: State<'_, Startup>) -> Loaded {
    let path = startup.config_path.display().to_string();
    let read = std::fs::read_to_string(&startup.config_path).map_err(|error| error.to_string());
    match read.and_then(|text| Config::parse(&text).map_err(|error| error.to_string())) {
        Ok(settings) => Loaded { path, settings: Some(settings), error: None },
        Err(error) => Loaded { path, settings: None, error: Some(error) },
    }
}

/// The defaults, for a fresh start when the file cannot be used (the window asks first).
#[tauri::command]
pub fn default_settings() -> Result<Config, String> {
    Config::parse(DEFAULT_CONFIG).map_err(|error| error.to_string())
}

/// Check the settings, write them and put them into effect; the error names what cannot work (nothing is written
/// then). After settings that could not be used at the start, Mundwerk starts anew with these.
// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn save_settings(app: AppHandle, startup: State<'_, Startup>, settings: Config) -> Result<(), String> {
    settings.save(&startup.config_path).map_err(|error| {
        log::error!("settings not saved: {error}");
        error.to_string()
    })?;
    log::info!("settings saved to {}", startup.config_path.display());
    match app.try_state::<Arc<Shared>>() {
        Some(shared) => take_effect(&app, &shared, settings),
        None => app.restart(),
    }
}

#[tauri::command]
pub fn microphones() -> Result<Vec<String>, String> {
    let devices = cpal::default_host().input_devices().map_err(|error| error.to_string())?;
    Ok(devices.filter_map(|device| device.name().ok()).collect())
}

/// The next key, combination or mouse button becomes a trigger ([`CAPTURE_EVENT`]); it is not passed on.
// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn start_capture(capture: State<'_, CaptureState>) {
    log::info!("capture: started");
    lock(&capture).start();
    crate::hook::TRACE.store(true, std::sync::atomic::Ordering::Relaxed);
}

// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn stop_capture(capture: State<'_, CaptureState>) {
    log::info!("capture: stopped");
    lock(&capture).stop();
    crate::hook::TRACE.store(false, std::sync::atomic::Ordering::Relaxed);
}

/// The language the pages show: of `setting` (the settings page while it is changed there), or the app's.
#[tauri::command]
pub fn ui_language(setting: Option<LanguageSetting>) -> Language {
    setting.map_or_else(texts::language, |setting| setting.resolve(sys_locale::get_locale().as_deref()))
}

/// What the settings page reports for the log: its errors and the steps of a capture.
// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn page_log(message: String) {
    log::info!("settings page: {message}");
}

pub fn toggle_overlay(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<Shared>>() else { return };
    let mut config = lock(&shared.config).clone();
    config.overlay.mode = match config.overlay.mode {
        OverlayMode::Floating => OverlayMode::TrayOnly,
        OverlayMode::TrayOnly => OverlayMode::Floating,
    };
    let result = config
        .save(&shared.config_path)
        .map_err(|error| error.to_string())
        .and_then(|()| take_effect(app, &shared, config));
    if let Err(error) = result {
        if let Some(reporter) = app.try_state::<Reporter>() {
            reporter.report(Status::Error(error));
        }
    }
}

fn take_effect(app: &AppHandle, shared: &Shared, config: Config) -> Result<(), String> {
    set_autostart(app, config.system.autostart)?;
    texts::set_language(config.system.language);
    if let Some(window) = app.get_webview_window(SETTINGS) {
        if let Err(error) = window.set_title(&texts::tr("settings.title")) {
            log::error!("settings window title: {error}");
        }
    }
    shared.apply(config)?;
    if let Some(reporter) = app.try_state::<Reporter>() {
        reporter.report(shared.status());
    }
    Ok(())
}

pub fn set_autostart(app: &AppHandle, wanted: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    let now = autostart.is_enabled().map_err(|error| error.to_string())?;
    match (now, wanted) {
        (false, true) => autostart.enable().map_err(|error| error.to_string()),
        (true, false) => autostart.disable().map_err(|error| error.to_string()),
        _ => Ok(()),
    }
}

#[derive(Serialize, Deserialize)]
struct Position {
    x: i32,
    y: i32,
}

fn position_file(config_path: &Path) -> PathBuf {
    config_path.with_file_name(POSITION_FILE)
}

/// Where the overlay was left, `None` before it was ever moved.
pub fn overlay_position(config_path: &Path) -> Result<Option<PhysicalPosition<i32>>, String> {
    let path = position_file(config_path);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let position: Position = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Some(PhysicalPosition::new(position.x, position.y)))
}

/// Remembers the overlay's place once it stands still.
#[derive(Clone)]
pub struct PositionKeeper {
    path: PathBuf,
    generation: Arc<AtomicU64>,
}

impl PositionKeeper {
    pub fn new(config_path: &Path) -> Self {
        Self { path: position_file(config_path), generation: Arc::new(AtomicU64::new(0)) }
    }

    pub fn moved(&self, position: PhysicalPosition<i32>) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let keeper = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(POSITION_SETTLED);
            if keeper.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            let text = serde_json::to_string(&Position { x: position.x, y: position.y }).unwrap_or_default();
            if let Err(error) = std::fs::write(&keeper.path, text) {
                log::error!("overlay position: {error}");
            }
        });
    }
}
