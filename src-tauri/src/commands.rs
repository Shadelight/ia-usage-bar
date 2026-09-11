//! Comandos expuestos al frontend (`#[tauri::command]`).

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::config;
use crate::config::AppConfig;
use crate::dashboard::{build_dashboard, do_refresh};
use crate::model::{Dashboard, VendorId};
use crate::state::{lock_or_recover, AppState, TrayMenuState};

pub(crate) fn refresh_catalog(state: &AppState, cfg: &AppConfig) {
    *lock_or_recover(&state.catalog) = crate::providers::catalog(cfg);
}

pub(crate) fn refresh_catalog_and_tray(app: &AppHandle, state: &AppState, cfg: &AppConfig) {
    refresh_catalog(state, cfg);
    let catalog = lock_or_recover(&state.catalog).clone();
    crate::tray::rebuild_primary_submenu(app, &catalog, &cfg.primary);
}

pub(crate) fn parse_id(s: &str) -> Option<VendorId> {
    VendorId::all().iter().copied().find(|id| id.slug() == s)
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
pub(crate) fn detect_providers(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<Vec<String>, String> {
    let mut cfg = lock_or_recover(&state.config);
    let mut candidate = cfg.clone();
    let newly = config::run_detect(&mut candidate)?;
    *cfg = candidate;
    refresh_catalog_and_tray(&app, &state, &cfg);
    drop(cfg);
    do_refresh(&app, None);
    Ok(newly)
}

#[tauri::command]
pub(crate) fn get_app_config(state: tauri::State<AppState>) -> AppConfig {
    lock_or_recover(&state.config).clone()
}

#[tauri::command]
pub(crate) fn set_app_config(
    app: AppHandle,
    state: tauri::State<AppState>,
    cfg: AppConfig,
) -> Result<(), String> {
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
    incoming.normalize();
    incoming.save()?;
    state
        .notifications_enabled
        .store(incoming.notifications, Ordering::Relaxed);
    *lock_or_recover(&state.config) = incoming;
    let current = lock_or_recover(&state.config).clone();
    refresh_catalog_and_tray(&app, &state, &current);
    do_refresh(&app, None);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_provider_enabled(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let vid = parse_id(&id).ok_or_else(|| format!("Proveedor desconocido: {id}"))?;
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.set_enabled(vid, enabled);
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg.clone();
    refresh_catalog_and_tray(&app, &state, &cfg);
    do_refresh(&app, None);
    Ok(())
}

#[tauri::command]
pub(crate) fn save_api_key(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
    key: String,
) -> Result<(), String> {
    let vid = parse_id(&id).ok_or_else(|| format!("Proveedor desconocido: {id}"))?;
    let mut cfg = lock_or_recover(&state.config).clone();
    config::store_api_key(vid, &key)?;
    let entry = cfg.providers.entry(id.clone()).or_default();
    entry.api_key = None;
    if !key.trim().is_empty() {
        cfg.set_enabled(vid, true);
    }
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg.clone();
    refresh_catalog_and_tray(&app, &state, &cfg);
    do_refresh(&app, None);
    Ok(())
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
pub(crate) fn set_notifications(
    state: tauri::State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .notifications_enabled
        .store(enabled, Ordering::Relaxed);
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.notifications = enabled;
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    Ok(())
}

#[tauri::command]
pub(crate) fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable().map_err(|e| e.to_string())?;
    } else {
        autostart.disable().map_err(|e| e.to_string())?;
    }
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.autostart.set_checked(enabled);
    }
    Ok(())
}

/// Shared by the `set_always_on_top` command and the tray "Siempre visible"
/// checkbox so persistence + window + tray-sync logic lives in one place.
pub(crate) fn apply_always_on_top(
    app: &AppHandle,
    state: &tauri::State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.always_on_top = enabled;
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(enabled);
    }
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.always_on_top.set_checked(enabled);
    }
    Ok(())
}

/// Shared by the `set_compact_mode` command and the tray "Modo compacto"
/// checkbox. Compact mode is a pure view state for the frontend, so there is
/// no window call here — only persistence + tray-sync.
pub(crate) fn apply_compact_mode(
    app: &AppHandle,
    state: &tauri::State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.compact_mode = enabled;
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.compact_mode.set_checked(enabled);
    }
    do_refresh(app, None);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_always_on_top(
    app: AppHandle,
    state: tauri::State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    apply_always_on_top(&app, &state, enabled)
}

#[tauri::command]
pub(crate) fn set_compact_mode(
    app: AppHandle,
    state: tauri::State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    apply_compact_mode(&app, &state, enabled)
}

/// Shared by the tray's primary-provider submenu (no dedicated command yet —
/// the frontend still switches primary through `set_app_config`).
pub(crate) fn set_primary(
    app: &AppHandle,
    state: &tauri::State<AppState>,
    id: &str,
) -> Result<(), String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.primary = id.to_string();
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    do_refresh(app, None);
    Ok(())
}

fn reveal_in_explorer(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::process::Command::new("explorer").arg(dir).spawn();
}

#[tauri::command]
pub(crate) fn open_logs_folder() {
    reveal_in_explorer(&crate::logfile::dir());
}

#[tauri::command]
pub(crate) fn clear_logs() {
    crate::logfile::clear();
}

#[tauri::command]
pub(crate) fn export_diagnostics(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let snapshots = lock_or_recover(&state.snapshots);
    let providers: Vec<_> = snapshots
        .values()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "status": s.status,
                "statusReason": s.status_reason,
                "updatedAt": s.updated_at,
            })
        })
        .collect();
    drop(snapshots);
    let payload = serde_json::json!({
        "appVersion": app.package_info().version.to_string(),
        "os": "windows",
        "providers": providers,
    });
    let dir = crate::logfile::dir();
    let file = dir.join(format!(
        "diagnostics-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(
        &file,
        serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    reveal_in_explorer(&dir);
    Ok(())
}
