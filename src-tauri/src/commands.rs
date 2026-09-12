//! Comandos expuestos al frontend (`#[tauri::command]`).

use std::sync::atomic::Ordering;

use serde::Serialize;
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
    let snapshots = lock_or_recover(&state.snapshots).clone();
    crate::tray::rebuild_provider_submenu(app, &catalog, &cfg.primary, &snapshots);
}

pub(crate) fn parse_id(s: &str) -> Option<VendorId> {
    VendorId::all().iter().copied().find(|id| id.slug() == s)
}

/// Marca interacción del usuario para la política de refresh adaptativo.
fn touch_activity(state: &tauri::State<AppState>) {
    *lock_or_recover(&state.last_activity) = std::time::Instant::now();
}

#[tauri::command]
pub(crate) fn get_dashboard(app: AppHandle, state: tauri::State<AppState>) -> Dashboard {
    touch_activity(&state);
    build_dashboard(&app, &state)
}

/// Contrato estable para automatización embebida: mismo `DashboardSnapshotV1`
/// que emite `iausage --json`.
#[tauri::command]
pub(crate) fn get_snapshot_v1(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> crate::snapshot_v1::DashboardSnapshotV1 {
    touch_activity(&state);
    let snaps = lock_or_recover(&state.snapshots).clone();
    let catalog = lock_or_recover(&state.catalog).clone();
    crate::snapshot_v1::build(
        &snaps,
        &catalog,
        crate::model::now_iso(),
        Some(app.package_info().version.to_string()),
    )
}

/// Fija la fuente preferida de un provider (`oauth`/`cli`/`api`/`web`/`local`
/// o vacía para Automática). Debe pertenecer a sus estrategias declaradas.
#[tauri::command]
pub(crate) fn set_source_preference(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
    source: String,
) -> Result<(), String> {
    use crate::descriptor::FetchStrategyKind;
    let vid = parse_id(&id).ok_or_else(|| format!("Proveedor desconocido: {id}"))?;
    let preferred = if source.trim().is_empty() || source == "auto" {
        None
    } else {
        Some(
            FetchStrategyKind::parse(&source)
                .ok_or_else(|| format!("Fuente desconocida: {source}"))?,
        )
    };
    if let Some(strategy) = preferred {
        let descriptor = crate::descriptor::descriptor(vid);
        if !descriptor.strategies.contains(&strategy) {
            return Err(format!(
                "La fuente {} no está implementada para {}",
                strategy.label(),
                vid.display_name()
            ));
        }
    }
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.set_source_preference(vid, preferred);
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg.clone();
    refresh_catalog_and_tray(&app, &state, &cfg);
    do_refresh(&app, None);
    Ok(())
}

#[tauri::command]
pub(crate) fn refresh_now(app: AppHandle, state: tauri::State<AppState>) {
    touch_activity(&state);
    do_refresh(&app, None);
}

#[tauri::command]
pub(crate) fn refresh_provider(app: AppHandle, state: tauri::State<AppState>, id: String) {
    touch_activity(&state);
    do_refresh(&app, Some(id));
}

#[tauri::command]
pub(crate) fn detect_providers(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<Vec<String>, String> {
    // Sin locks durante el detect: sondea CLIs y red, y un hijo colgado no
    // debe congelar los comandos ni el refresh (que también piden config).
    let snapshot = lock_or_recover(&state.config).clone();
    let mut working = snapshot;
    let newly = config::run_detect(&mut working)?;
    {
        let mut cfg = lock_or_recover(&state.config);
        if cfg.load_recovered {
            // Recuperación pendiente de revisión: no tocar nada.
            return Ok(newly);
        }
        for id in &newly {
            if let Some(vid) = parse_id(id) {
                cfg.set_enabled(vid, true);
            }
        }
        cfg.save()?;
        refresh_catalog_and_tray(&app, &state, &cfg);
    }
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
            // La preferencia de fuente vive en Ajustes del provider; un save
            // general que no la trae no debe resetearla a Automática.
            if entry.source.is_none() {
                entry.source = existing.source;
            }
        }
    }
    // El sync se gobierna por sus comandos dedicados; un save general de
    // Ajustes nunca debe apagarlo ni borrar su carpeta, passphrase aparte
    // (la passphrase vive en keyring y ningún save la toca).
    {
        let current = lock_or_recover(&state.config);
        incoming.sync_enabled = current.sync_enabled;
        incoming.sync_export_dir.clone_from(&current.sync_export_dir);
        incoming.sync_lan = current.sync_lan;
    }
    incoming.normalize();
    incoming.save()?;
    state
        .notifications_enabled
        .store(incoming.notifications, Ordering::Relaxed);
    *lock_or_recover(&state.config) = incoming;    let current = lock_or_recover(&state.config).clone();
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
pub(crate) fn delete_api_key(
    app: AppHandle,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    let vid = parse_id(&id).ok_or_else(|| format!("Proveedor desconocido: {id}"))?;
    let cfg = lock_or_recover(&state.config).clone();
    // An empty value deletes the OS keyring entry (see store_api_key).
    // Enabled is deliberately left untouched: deleting a credential moves
    // the provider to needs_credential, it must never disable it.
    config::store_api_key(vid, "")?;
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
    crate::dashboard::emit_dashboard(app);
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateCheck {
    current: String,
    latest: String,
    url: String,
    update_available: bool,
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.split('-').next().unwrap_or("0").parse().unwrap_or(0))
        .collect()
}

#[tauri::command]
pub(crate) fn check_for_updates(app: AppHandle) -> Result<UpdateCheck, String> {
    let current = app.package_info().version.to_string();
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("IA-Usage-Bar")
        .build()
        .map_err(|_| "No se pudo preparar la búsqueda de actualizaciones".to_string())?;
    let response: serde_json::Value = client
        .get("https://api.github.com/repos/Shadelight/ia-usage-bar/releases/latest")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|_| "No se pudo consultar la última versión".to_string())?
        .json()
        .map_err(|_| "La respuesta de actualización no es válida".to_string())?;
    let latest = response
        .get("tag_name")
        .and_then(|value| value.as_str())
        .unwrap_or(&current)
        .trim_start_matches('v')
        .to_string();
    let url = response
        .get("html_url")
        .and_then(|value| value.as_str())
        .unwrap_or("https://github.com/Shadelight/ia-usage-bar/releases/latest")
        .to_string();
    Ok(UpdateCheck {
        update_available: version_parts(&latest) > version_parts(&current),
        current,
        latest,
        url,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncExportInfo {
    path: String,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncStatus {
    enabled: bool,
    device_id: String,
    fingerprint: String,
    export_dir: String,
    has_passphrase: bool,
    lan: bool,
    server_running: bool,
    server_addr: String,
    last_export: Option<SyncExportInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncPairing {
    uri: String,
    fingerprint: String,
    host: String,
    port: u16,
    qr_png_base64: String,
}

fn sync_last_export(cfg: &AppConfig, device_id: &str) -> Option<SyncExportInfo> {
    let path = crate::sync::resolve_export_dir(cfg).join(format!("{device_id}.json"));
    let bytes = std::fs::metadata(&path).ok()?.len();
    Some(SyncExportInfo {
        path: path.display().to_string(),
        bytes,
    })
}

#[tauri::command]
pub(crate) fn sync_get_status(
    _app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<SyncStatus, String> {
    let cfg = lock_or_recover(&state.config).clone();
    let device_id = crate::sync::load_or_create_device_id()?;
    let server = lock_or_recover(&state.sync_server);
    Ok(SyncStatus {
        enabled: cfg.sync_enabled,
        fingerprint: crate::sync::pairing_fingerprint(&device_id),
        device_id: device_id.clone(),
        export_dir: crate::sync::resolve_export_dir(&cfg).display().to_string(),
        has_passphrase: crate::sync::has_passphrase(),
        lan: cfg.sync_lan,
        server_running: server.running,
        server_addr: server.addr.clone(),
        last_export: sync_last_export(&cfg, &device_id),
    })
}

#[tauri::command]
pub(crate) fn sync_set_enabled(
    app: AppHandle,
    state: tauri::State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    if enabled && !crate::sync::has_passphrase() {
        return Err("sync: primero guarda una passphrase".into());
    }
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.sync_enabled = enabled;
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    crate::sync_service::ensure_sync_server(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn sync_set_passphrase(
    app: AppHandle,
    state: tauri::State<AppState>,
    passphrase: String,
) -> Result<(), String> {
    crate::sync::store_passphrase(passphrase.trim())?;
    if passphrase.trim().is_empty() {
        // Olvidar la passphrase con sync activo dejaría el servidor cifrando
        // con una clave que ya no existe: se apaga y se pide reactivar.
        let mut cfg = lock_or_recover(&state.config).clone();
        cfg.sync_enabled = false;
        cfg.save()?;
        *lock_or_recover(&state.config) = cfg;
    }
    crate::sync_service::ensure_sync_server(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn sync_set_export_dir(
    _app: AppHandle,
    state: tauri::State<AppState>,
    dir: String,
) -> Result<(), String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    let trimmed = dir.trim();
    cfg.sync_export_dir = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    Ok(())
}

#[tauri::command]
pub(crate) fn sync_set_lan(
    app: AppHandle,
    state: tauri::State<AppState>,
    lan: bool,
) -> Result<(), String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    cfg.sync_lan = lan;
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    // Rebind: el bind (loopback vs 0.0.0.0) exige reiniciar el hilo.
    crate::sync_service::ensure_sync_server(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn sync_get_pairing(lan: bool) -> Result<SyncPairing, String> {
    let device_id = crate::sync::load_or_create_device_id()?;
    let host = if lan {
        crate::sync_server::lan_ip().ok_or_else(|| "sync: sin IP LAN detectable".to_string())?
    } else {
        "127.0.0.1".to_string()
    };
    let info = crate::sync_server::PairingInfo::new(
        host.clone(),
        crate::sync_server::SYNC_DEFAULT_PORT,
        &device_id,
    );
    let uri = info.to_uri();
    let png = crate::sync_server::pairing_qr_png(&uri, 512)?;
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    Ok(SyncPairing {
        uri,
        fingerprint: info.fingerprint,
        host,
        port: info.port,
        qr_png_base64: B64.encode(&png),
    })
}

#[tauri::command]
pub(crate) fn sync_export_now(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<String, String> {
    let cfg = lock_or_recover(&state.config).clone();
    let snaps = lock_or_recover(&state.snapshots).clone();
    let version = app.package_info().version.to_string();
    match crate::sync::export_current_snapshot(&cfg, &snaps, Some(version))? {
        Some(path) => Ok(path.display().to_string()),
        None => Err("sync desactivado".into()),
    }
}

#[cfg(test)]
mod update_tests {
    use super::version_parts;

    #[test]
    fn compares_release_versions_numerically() {
        assert!(version_parts("v0.10.0") > version_parts("0.2.9"));
        assert_eq!(version_parts("v0.2.0"), vec![0, 2, 0]);
    }
}
