//! Comandos expuestos al frontend (`#[tauri::command]`).

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};

use crate::config;
use crate::config::AppConfig;
use crate::dashboard::{build_dashboard, do_refresh};
use crate::model::{Dashboard, VendorId};
use crate::state::{lock_or_recover, AppState};

pub(crate) fn parse_id(s: &str) -> Option<VendorId> {
    VendorId::all()
        .iter()
        .copied()
        .find(|id| id.slug() == s)
}

#[tauri::command]
pub(crate) fn get_dashboard(app: AppHandle, state: tauri::State<AppState>) -> Dashboard {
    build_dashboard(&app, &state)
}

#[tauri::command]
pub(crate) fn refresh_now(app: AppHandle) {
    do_refresh(&app, None);
}

#[tauri::command]
pub(crate) fn refresh_provider(app: AppHandle, id: String) {
    do_refresh(&app, Some(id));
}

#[tauri::command]
pub(crate) fn detect_providers(app: AppHandle, state: tauri::State<AppState>) -> Vec<String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    let newly = config::run_detect(&mut cfg);
    *lock_or_recover(&state.config) = cfg;
    do_refresh(&app, None);
    newly
}

#[tauri::command]
pub(crate) fn get_app_config(state: tauri::State<AppState>) -> AppConfig {
    lock_or_recover(&state.config).clone()
}

#[tauri::command]
pub(crate) fn set_app_config(app: AppHandle, state: tauri::State<AppState>, cfg: AppConfig) {
    let mut incoming = cfg;
    {
        let current = lock_or_recover(&state.config);
        for (id, existing) in &current.providers {
            let entry = incoming.providers.entry(id.clone()).or_default();
            if entry.api_key.is_none() {
                entry.api_key = existing.api_key.clone();
            }
            if entry.team_id.is_none() {
                entry.team_id = existing.team_id.clone();
            }
            if entry.region.is_none() {
                entry.region = existing.region.clone();
            }
        }
    }
    let _ = incoming.save();
    state
        .notifications_enabled
        .store(incoming.notifications, Ordering::Relaxed);
    *lock_or_recover(&state.config) = incoming;
    do_refresh(&app, None);
}

#[tauri::command]
pub(crate) fn set_provider_enabled(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
    enabled: bool,
) {
    let mut cfg = lock_or_recover(&state.config);
    if let Some(vid) = parse_id(&id) {
        cfg.set_enabled(vid, enabled);
        let _ = cfg.save();
    }
    drop(cfg);
    do_refresh(&app, None);
}

#[tauri::command]
pub(crate) fn save_api_key(app: AppHandle, state: tauri::State<AppState>, id: String, key: String) {
    let mut cfg = lock_or_recover(&state.config);
    let entry = cfg.providers.entry(id.clone()).or_default();
    if key.trim().is_empty() {
        entry.api_key = None;
    } else {
        entry.api_key = Some(key);
        if let Some(vid) = parse_id(&id) {
            cfg.set_enabled(vid, true);
        }
    }
    let _ = cfg.save();
    drop(cfg);
    do_refresh(&app, None);
}

#[tauri::command]
pub(crate) fn quit(app: AppHandle) {
    app.state::<AppState>()
        .allow_exit
        .store(true, Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
pub(crate) fn hide_panel(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
pub(crate) fn set_notifications(state: tauri::State<AppState>, enabled: bool) {
    state
        .notifications_enabled
        .store(enabled, Ordering::Relaxed);
    let mut cfg = lock_or_recover(&state.config);
    cfg.notifications = enabled;
    let _ = cfg.save();
}
