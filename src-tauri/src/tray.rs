//! The tray icon: its colour and tooltip show the state, its menu has the settings, the overlay switch, the
//! history (a click copies an entry) and quitting.

use std::sync::Arc;

use mundwerk_core::config::OverlayMode;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::engine::{lock, Shared};
use crate::status::Status;
use crate::{insert, settings, texts};

const TRAY: &str = "mundwerk";
const SETTINGS_ITEM: &str = "settings";
const OVERLAY_ITEM: &str = "overlay";
const QUIT_ITEM: &str = "quit";
const LOG_ITEM: &str = "log";
const RESEND_ITEM: &str = "resend";
const HISTORY_PREFIX: &str = "history:";

const ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");
const ICON_LISTENING: &[u8] = include_bytes!("../icons/tray-listening.png");
const ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/tray-transcribing.png");
const ICON_ERROR: &[u8] = include_bytes!("../icons/tray-error.png");

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY)
        .icon(Image::from_bytes(ICON_IDLE)?)
        .tooltip(texts::status(&Status::Idle))
        .menu(&menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| clicked(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

/// Icon, tooltip and menu (the history may have grown) for `status`.
pub fn show_status(app: &AppHandle, status: &Status, tooltip: &str) {
    let Some(tray) = app.tray_by_id(TRAY) else { return };
    let icon = match status {
        Status::Idle => ICON_IDLE,
        Status::Listening => ICON_LISTENING,
        Status::Transcribing => ICON_TRANSCRIBING,
        Status::Error(_) => ICON_ERROR,
    };
    if let Ok(image) = Image::from_bytes(icon) {
        let _ = tray.set_icon(Some(image));
    }
    let _ = tray.set_tooltip(Some(tooltip));
    if let Ok(menu) = menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let shared = app.try_state::<Arc<Shared>>();
    let floating = shared.as_ref().is_some_and(|shared| lock(&shared.config).overlay.mode == OverlayMode::Floating);
    let entries: Vec<String> = shared
        .as_ref()
        .map(|shared| lock(&shared.history).entries().map(|entry| label(&entry.when, &entry.text)).collect())
        .unwrap_or_default();
    let history = Submenu::new(app, texts::HISTORY, true)?;
    if entries.is_empty() {
        history.append(&MenuItem::new(app, texts::HISTORY_EMPTY, false, None::<&str>)?)?;
    }
    for (index, entry) in entries.iter().enumerate() {
        history.append(&MenuItem::with_id(app, format!("{HISTORY_PREFIX}{index}"), entry, true, None::<&str>)?)?;
    }
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, SETTINGS_ITEM, texts::SETTINGS, true, None::<&str>)?,
            &CheckMenuItem::with_id(app, OVERLAY_ITEM, texts::OVERLAY_SHOWN, true, floating, None::<&str>)?,
            &history,
        ],
    )?;
    // Only while a recording the STT-Suite did not transcribe waits.
    if shared.as_ref().is_some_and(|shared| shared.has_failed()) {
        menu.append(&MenuItem::with_id(app, RESEND_ITEM, texts::RESEND, true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, LOG_ITEM, texts::OPEN_LOG, true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, QUIT_ITEM, texts::QUIT, true, None::<&str>)?)?;
    Ok(menu)
}

fn label(when: &str, text: &str) -> String {
    let mut short: String = text.chars().take(texts::HISTORY_LABEL_CHARS).collect();
    if text.chars().count() > texts::HISTORY_LABEL_CHARS {
        short.push('…');
    }
    format!("{when}  {short}")
}

fn clicked(app: &AppHandle, id: &str) {
    match id {
        SETTINGS_ITEM => settings::open(app),
        OVERLAY_ITEM => settings::toggle_overlay(app),
        QUIT_ITEM => app.exit(0),
        LOG_ITEM => open_log(app),
        RESEND_ITEM => resend(app),
        _ => {
            if let Some(index) = id.strip_prefix(HISTORY_PREFIX).and_then(|index| index.parse::<usize>().ok()) {
                copy_entry(app, index);
            }
        }
    }
}

fn resend(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<Shared>>() else { return };
    if shared.resend() {
        if let Some(reporter) = app.try_state::<crate::status::Reporter>() {
            reporter.report(shared.status());
        }
    }
}

fn open_log(app: &AppHandle) {
    let opened = app.path().app_log_dir().map_err(|error| error.to_string()).and_then(|folder| {
        let file = folder.join(format!("{}.log", crate::LOG_NAME));
        app.opener().open_path(file.display().to_string(), None::<&str>).map_err(|error| error.to_string())
    });
    if let Err(error) = opened {
        log::error!("open the log: {error}");
    }
}

fn copy_entry(app: &AppHandle, index: usize) {
    let Some(shared) = app.try_state::<Arc<Shared>>() else { return };
    let text = lock(&shared.history).entries().nth(index).map(|entry| entry.text.clone());
    if let Some(text) = text {
        if let Err(error) = insert::copy(&text) {
            log::error!("copy: {error}");
        }
    }
}
