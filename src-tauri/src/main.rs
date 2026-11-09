// No console window on Windows in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Mundwerk: dictate into any text field. A tray app with an overlay and a settings window; the logic is in
//! mundwerk-core.

mod audio;
mod badge;
mod engine;
mod hook;
mod insert;
mod place;
mod settings;
mod sound;
mod status;
mod texts;
mod tray;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mundwerk_core::capture::{Capture, Update};
use mundwerk_core::controller::Event;

use mundwerk_core::config::{Config, Placement};
use mundwerk_core::i18n::LanguageSetting;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::engine::{lock, Shared};
use crate::settings::{CaptureState, PositionKeeper, CAPTURE_EVENT};
use crate::status::{Reporter, Status};

pub const OVERLAY: &str = "overlay";
const OVERLAY_PAGE: &str = "overlay.html";
const OVERLAY_SIZE: LogicalSize<f64> = LogicalSize::new(260.0, 56.0);
/// The overlay's default place: this far above the bottom of the main screen (physical pixels).
const OVERLAY_BOTTOM_GAP: i32 = 120;
const CONFIG_FILE: &str = "config.toml";
/// The log file in the system's log folder of the app (`mundwerk.log`); the tray opens it.
pub const LOG_NAME: &str = "mundwerk";
/// A log larger than this starts anew (the old one is kept once).
const LOG_MAX_BYTES: u128 = 2_000_000;

/// Where the settings file is (known before it is read, so the settings window works also when it cannot be used).
pub struct Startup {
    pub config_path: PathBuf,
}

fn main() {
    let app = tauri::Builder::default()
        // A second start opens the settings of the first instead; logged with what started it, so a start that
        // nobody meant (an autostart, Windows restoring apps) can be found.
        .plugin(tauri_plugin_single_instance::init(|app, arguments, folder| {
            log::info!("second start (arguments {arguments:?}, folder {folder:?}): opening the settings");
            settings::open(app);
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(Target::new(TargetKind::LogDir { file_name: Some(LOG_NAME.into()) }))
                .level(log::LevelFilter::Info)
                .timezone_strategy(TimezoneStrategy::UseLocal)
                .max_file_size(LOG_MAX_BYTES)
                .rotation_strategy(RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            settings::load_settings,
            settings::default_settings,
            settings::save_settings,
            settings::microphones,
            settings::start_capture,
            settings::stop_capture,
            settings::page_log,
            settings::ui_language,
            settings::play_sound,
            settings::close_settings,
            overlay_opacity,
        ])
        .setup(|app| {
            setup(app.handle()).map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| panic!("Mundwerk cannot start: {error}"));
    app.run(|_, event| {
        // A tray app: closing the settings window does not end it, only quitting from the tray does.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

fn setup(app: &AppHandle) -> Result<(), String> {
    let folder = app.path().app_config_dir().map_err(|error| error.to_string())?;
    let config_path = folder.join(CONFIG_FILE);
    app.manage(Startup { config_path: config_path.clone() });
    let reporter = Reporter::new(app.clone());
    app.manage(reporter.clone());
    let capture: CaptureState = Arc::new(Mutex::new(Capture::default()));
    app.manage(Arc::clone(&capture));
    // The hook runs in any case: the settings window takes triggers through it, also while the settings cannot be
    // used.
    start_hook(app, capture, reporter.clone())?;
    let Ok(config) = Config::load_or_create(&config_path) else {
        // Nothing runs on settings that cannot work: the settings window shows why (in the system's language);
        // saving restarts.
        texts::set_language(LanguageSetting::Auto);
        settings::open(app);
        return Ok(());
    };
    texts::set_language(config.system.language);
    let autostart = config.system.autostart;
    let shared = engine::start(config_path.clone(), config, reporter.clone())?;
    app.manage(Arc::clone(&shared));
    if let Err(error) = settings::set_autostart(app, autostart) {
        reporter.report(Status::Error(format!("{}: {error}", texts::tr("error.autostart"))));
    }
    create_overlay(app, &config_path).map_err(|error| error.to_string())?;
    badge::create(app).map_err(|error| error.to_string())?;
    tray::create(app).map_err(|error| error.to_string())?;
    reporter.report(Status::Idle);
    Ok(())
}

/// Every press goes to a running capture first (the settings window sees what is held, then the trigger), then to
/// the dictation. Only the press that completes a capture is swallowed by it.
fn start_hook(app: &AppHandle, capture: CaptureState, reporter: Reporter) -> Result<(), String> {
    let for_moves = app.clone();
    let on_move = move |x: i32, y: i32| badge::moved(&for_moves, x, y);
    let app = app.clone();
    let handler = move |event: &Event| {
        let update = lock(&capture).observe(event);
        if let Some(update) = update {
            log::info!("capture: {update:?}");
            let done = matches!(update, Update::Done(_));
            if done {
                hook::TRACE.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            if let Err(error) = app.emit(CAPTURE_EVENT, update) {
                log::error!("capture: sending to the settings window: {error}");
            }
            if done {
                return true;
            }
        }
        app.try_state::<Arc<Shared>>().is_some_and(|shared| shared.press(event))
    };
    std::thread::Builder::new()
        .name("hook".into())
        .spawn(move || {
            if let Err(error) = hook::run(Box::new(handler), Box::new(on_move)) {
                reporter.report(Status::Error(format!("{}: {error}", texts::tr("error.hook"))));
            }
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// The small window that shows listening and transcribing: no frame, always on top, never takes the focus (the
/// text goes to the field that has it), moved with the mouse anywhere, any screen.
fn create_overlay(app: &AppHandle, config_path: &std::path::Path) -> tauri::Result<()> {
    let overlay = WebviewWindowBuilder::new(app, OVERLAY, WebviewUrl::App(OVERLAY_PAGE.into()))
        .title(texts::tr("app"))
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
            log::warn!("overlay position (the default place is used): {error}");
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
    let app = app.clone();
    overlay.on_window_event(move |event| {
        // Only a fixed place is remembered: next to the text the overlay is moved by Mundwerk itself.
        let fixed = app
            .try_state::<Arc<Shared>>()
            .is_some_and(|shared| lock(&shared.config).overlay.placement == Placement::Fixed);
        if let (WindowEvent::Moved(position), true) = (event, fixed) {
            keeper.moved(*position);
        }
    });
    Ok(())
}

// Tauri hands a command its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
fn overlay_opacity(shared: tauri::State<'_, Arc<Shared>>) -> f32 {
    mundwerk_core::config::fraction(lock(&shared.config).overlay.opacity_percent)
}
