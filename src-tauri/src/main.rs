// No console window on Windows in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Mundwerk: dictate into any text field. A tray app with an overlay and a settings window; the logic is in
//! mundwerk-core.

mod audio;
mod engine;
mod hook;
mod insert;
mod settings;
mod sound;
mod status;
mod texts;
mod tray;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mundwerk_core::config::Config;
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;

use crate::engine::{lock, Shared};
use crate::settings::PositionKeeper;
use crate::status::{Reporter, Status};

pub const OVERLAY: &str = "overlay";
const OVERLAY_PAGE: &str = "overlay.html";
const OVERLAY_SIZE: LogicalSize<f64> = LogicalSize::new(260.0, 56.0);
/// The overlay's default place: this far above the bottom of the main screen (physical pixels).
const OVERLAY_BOTTOM_GAP: i32 = 120;
const CONFIG_FILE: &str = "config.toml";

/// Where the settings file is, and why it could not be used at the start (the settings window shows it).
pub struct Startup {
    pub config_path: PathBuf,
    pub error: Mutex<Option<String>>,
}

fn main() {
    let app = tauri::Builder::default()
        // A second start opens the settings of the first instead.
        .plugin(tauri_plugin_single_instance::init(|app, _arguments, _folder| settings::open(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            settings::settings_text,
            settings::default_settings_text,
            settings::save_settings,
            settings::microphones,
            settings::startup_error,
            overlay_opacity,
        ])
        .setup(|app| {
            setup(app.handle()).map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| panic!("Mundwerk cannot start: {error}"));
    app.run(|_, event| {
        // A tray app: closing the settings window does not end it, only "Beenden" does.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

fn setup(app: &AppHandle) -> Result<(), String> {
    let folder = app.path().app_config_dir().map_err(|error| error.to_string())?;
    let config_path = folder.join(CONFIG_FILE);
    let loaded = Config::load_or_create(&config_path);
    app.manage(Startup { config_path: config_path.clone(), error: Mutex::new(None) });
    let config = match loaded {
        Ok(config) => config,
        Err(error) => {
            // Nothing runs on settings that cannot work: the settings window shows why; saving restarts.
            *lock(&app.state::<Startup>().error) = Some(error.to_string());
            settings::open(app);
            return Ok(());
        }
    };
    let reporter = Reporter::new(app.clone());
    app.manage(reporter.clone());
    let autostart = config.system.autostart;
    let shared = engine::start(config_path.clone(), config, reporter.clone())?;
    app.manage(Arc::clone(&shared));
    if let Err(error) = settings::set_autostart(app, autostart) {
        reporter.report(Status::Error(format!("autostart: {error}")));
    }
    create_overlay(app, &config_path).map_err(|error| error.to_string())?;
    tray::create(app).map_err(|error| error.to_string())?;
    let hook_reporter = reporter.clone();
    std::thread::Builder::new()
        .name("hook".into())
        .spawn(move || {
            if let Err(error) = hook::run(Box::new(move |event| shared.press(event))) {
                hook_reporter.report(Status::Error(format!("keyboard and mouse hook: {error}")));
            }
        })
        .map_err(|error| error.to_string())?;
    reporter.report(Status::Idle);
    Ok(())
}

/// The small window that shows listening and transcribing: no frame, always on top, never takes the focus (the
/// text goes to the field that has it), moved with the mouse anywhere, any screen.
fn create_overlay(app: &AppHandle, config_path: &std::path::Path) -> tauri::Result<()> {
    let overlay = WebviewWindowBuilder::new(app, OVERLAY, WebviewUrl::App(OVERLAY_PAGE.into()))
        .title(texts::APP)
        .inner_size(OVERLAY_SIZE.width, OVERLAY_SIZE.height)
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
    let position = match settings::overlay_position(config_path) {
        Ok(position) => position,
        Err(error) => {
            eprintln!("Mundwerk: {error}");
            None
        }
    };
    match position {
        Some(position) => overlay.set_position(position)?,
        None => {
            if let Some(monitor) = overlay.primary_monitor()? {
                let size = overlay.outer_size()?;
                let screen = monitor.size();
                let origin = monitor.position();
                // Screen and window sizes in pixels fit an i32 by far.
                #[allow(clippy::cast_possible_wrap)]
                let (x, y) = (
                    origin.x + (screen.width as i32 - size.width as i32) / 2,
                    origin.y + screen.height as i32 - size.height as i32 - OVERLAY_BOTTOM_GAP,
                );
                overlay.set_position(PhysicalPosition::new(x, y))?;
            }
        }
    }
    let keeper = PositionKeeper::new(config_path);
    overlay.on_window_event(move |event| {
        if let WindowEvent::Moved(position) = event {
            keeper.moved(*position);
        }
    });
    Ok(())
}

// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
fn overlay_opacity(shared: tauri::State<'_, Arc<Shared>>) -> f32 {
    lock(&shared.config).overlay.opacity
}
