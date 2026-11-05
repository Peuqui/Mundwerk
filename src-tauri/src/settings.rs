//! The settings window and what changes the settings: it edits the settings file as text (comments and all),
//! checks it and puts it into effect; the tray's overlay switch; starting with the system; where the overlay
//! was left (a file of its own: it is a window's place, not a setting).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};
use mundwerk_core::config::{Config, OverlayMode, DEFAULT_CONFIG};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

use crate::engine::{lock, Shared};
use crate::status::{Reporter, Status};
use crate::{texts, Startup};

pub const SETTINGS: &str = "settings";
const SETTINGS_PAGE: &str = "settings.html";
const POSITION_FILE: &str = "overlay-position.json";
/// The overlay is saved once it has stood still this long (moving it reports every pixel).
const POSITION_SETTLED: Duration = Duration::from_millis(800);

pub fn open(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(SETTINGS_PAGE.into()))
        .title(texts::SETTINGS_TITLE)
        .inner_size(900.0, 760.0)
        .build();
    if let Err(error) = built {
        eprintln!("Mundwerk: settings window: {error}");
    }
}

#[derive(Serialize)]
pub struct SettingsText {
    path: String,
    text: String,
}

// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn settings_text(startup: State<'_, Startup>) -> Result<SettingsText, String> {
    let text = std::fs::read_to_string(&startup.config_path).map_err(|error| error.to_string())?;
    Ok(SettingsText { path: startup.config_path.display().to_string(), text })
}

/// Why the settings could not be used at the start (nothing runs then), `None` when they could.
// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn startup_error(startup: State<'_, Startup>) -> Option<String> {
    lock(&startup.error).clone()
}

#[tauri::command]
pub fn default_settings_text() -> String {
    DEFAULT_CONFIG.to_owned()
}

/// Check `text`, write it and put it into effect; the error names what cannot work (nothing is written then).
/// After settings that could not be used at the start, Mundwerk starts anew with these.
// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn save_settings(app: AppHandle, startup: State<'_, Startup>, text: String) -> Result<(), String> {
    let config = Config::save_text(&text, &startup.config_path).map_err(|error| error.to_string())?;
    match app.try_state::<Arc<Shared>>() {
        Some(shared) => take_effect(&app, &shared, config),
        None => app.restart(),
    }
}

#[tauri::command]
pub fn microphones() -> Result<Vec<String>, String> {
    let devices = cpal::default_host().input_devices().map_err(|error| error.to_string())?;
    Ok(devices.filter_map(|device| device.name().ok()).collect())
}

pub fn toggle_overlay(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<Shared>>() else { return };
    let mode = match lock(&shared.config).overlay.mode {
        OverlayMode::Floating => OverlayMode::TrayOnly,
        OverlayMode::TrayOnly => OverlayMode::Floating,
    };
    let result = Config::save_overlay_mode(&shared.config_path, mode)
        .map_err(|error| error.to_string())
        .and_then(|config| take_effect(app, &shared, config));
    if let Err(error) = result {
        if let Some(reporter) = app.try_state::<Reporter>() {
            reporter.report(Status::Error(error));
        }
    }
}

fn take_effect(app: &AppHandle, shared: &Shared, config: Config) -> Result<(), String> {
    set_autostart(app, config.system.autostart)?;
    shared.apply(config)?;
    if let Some(reporter) = app.try_state::<Reporter>() {
        reporter.report(Status::Idle);
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
                eprintln!("Mundwerk: overlay position: {error}");
            }
        });
    }
}
