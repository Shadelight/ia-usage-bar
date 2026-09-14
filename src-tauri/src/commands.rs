//! Comandos expuestos al frontend (`#[tauri::command]`).

use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::config;
use crate::config::AppConfig;
use crate::dashboard::{build_dashboard, do_refresh, emit_dashboard};
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
    // Changing one source only invalidates that provider. A full refresh here
    // unnecessarily produces dashboard events for every Settings control.
    lock_or_recover(&state.backoff_until).remove(vid.slug());
    do_refresh(&app, Some(id));
    Ok(())
}

#[tauri::command]
pub(crate) fn refresh_now(app: AppHandle, state: tauri::State<AppState>) {
    touch_activity(&state);
    // A manual refresh is an explicit retry request, not a decorative button:
    // do not silently skip providers because of an automatic backoff.
    lock_or_recover(&state.backoff_until).clear();
    do_refresh(&app, None);
}

#[tauri::command]
pub(crate) fn refresh_provider(app: AppHandle, state: tauri::State<AppState>, id: String) {
    touch_activity(&state);
    lock_or_recover(&state.backoff_until).remove(&id);
    do_refresh(&app, Some(id));
}

fn provider_login_command(id: VendorId) -> Result<(&'static str, &'static [&'static str]), String> {
    match id {
        VendorId::Anthropic => Ok(("claude", &[])),
        VendorId::Openai => Ok(("codex", &["login"])),
        _ => Err(format!(
            "{} no tiene un inicio de sesión automático disponible",
            id.display_name()
        )),
    }
}

/// Starts the provider's official interactive client in a separate terminal.
/// OAuth credentials remain owned by that client; this app only observes them.
#[tauri::command]
pub(crate) fn start_provider_login(id: String) -> Result<(), String> {
    let id = parse_id(&id).ok_or_else(|| format!("Proveedor desconocido: {id}"))?;
    let (program, args) = provider_login_command(id)?;
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("No se pudo iniciar {program}: {error}"))
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
        incoming
            .sync_export_dir
            .clone_from(&current.sync_export_dir);
        incoming.sync_lan = current.sync_lan;
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
    if enabled {
        lock_or_recover(&state.backoff_until).remove(vid.slug());
        do_refresh(&app, Some(id));
    } else {
        // The provider immediately disappears from the catalog/dashboard; no
        // network request is needed just to turn something off.
        crate::dashboard::emit_dashboard(&app);
    }
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
    if !key.trim().is_empty() {
        config::verify_keyring_api_key(vid, &key)?;
    }
    let entry = cfg.providers.entry(id.clone()).or_default();
    entry.api_key = None;
    if !key.trim().is_empty() {
        cfg.set_enabled(vid, true);
    }
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg.clone();
    refresh_catalog_and_tray(&app, &state, &cfg);
    // No network here: the frontend applies its optimistic credential state
    // first and then triggers a scoped `refresh_provider`, so a fast
    // validation can never finish before the UI enters "validating".
    emit_dashboard(&app);
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
    // Same split as save_api_key: emit the catalog, let the frontend drive
    // the scoped refresh that replaces the snapshot with needs_auth.
    emit_dashboard(&app);
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliInstallStatus {
    binary_exists: bool,
    binary_path: String,
    path_configured: bool,
    version: Option<String>,
}

fn normalized_path_segment(value: &str) -> String {
    let trimmed = value.trim().trim_matches('"');
    let without_trailing = trimmed.trim_end_matches(['\\', '/']);
    if without_trailing.len() == 2 && without_trailing.ends_with(':') {
        format!("{without_trailing}\\")
    } else {
        without_trailing.to_string()
    }
}

fn same_path_segment(left: &str, right: &str) -> bool {
    normalized_path_segment(left).eq_ignore_ascii_case(&normalized_path_segment(right))
}

fn path_has_segment(path: &str, target: &str) -> bool {
    path.split(';')
        .any(|segment| !segment.trim().is_empty() && same_path_segment(segment, target))
}

/// Adds the exact directory once. Similar substrings (for example `bin` and
/// `binary`) remain distinct; duplicate exact segments are collapsed.
fn add_path_segment(path: &str, target: &str) -> String {
    let target = normalized_path_segment(target);
    let mut out = Vec::new();
    let mut found = false;
    for segment in path
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if same_path_segment(segment, &target) {
            if !found {
                out.push(target.clone());
                found = true;
            }
        } else {
            out.push(segment.to_string());
        }
    }
    if !found {
        out.push(target);
    }
    out.join(";")
}

const CLI_RESOURCE_RELATIVE_PATH: &str = "resources/bin/iausage.exe";

fn cli_binary_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .resolve(CLI_RESOURCE_RELATIVE_PATH, BaseDirectory::Resource)
        .map_err(|error| format!("No se pudo localizar IA Usage CLI: {error}"))
}

fn cli_version(path: &std::path::Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

#[cfg(windows)]
fn read_user_path() -> Result<String, String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    let environment = winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ)
        .map_err(|error| format!("No se pudo leer el PATH de usuario: {error}"))?;
    Ok(environment.get_value("Path").unwrap_or_default())
}

#[cfg(not(windows))]
fn read_user_path() -> Result<String, String> {
    Ok(std::env::var("PATH").unwrap_or_default())
}

#[cfg(windows)]
fn write_user_path(value: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegValue;

    let environment = winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|error| format!("No se pudo abrir el PATH de usuario: {error}"))?;
    // Keep PATH as REG_EXPAND_SZ so existing %VARIABLE% segments retain their
    // Windows semantics. Encode the complete untruncated UTF-16 value.
    let mut bytes = Vec::new();
    for word in OsStr::new(value).encode_wide().chain(std::iter::once(0)) {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    environment
        .set_raw_value(
            "Path",
            &RegValue {
                bytes,
                vtype: RegType::REG_EXPAND_SZ,
            },
        )
        .map_err(|error| format!("No se pudo actualizar el PATH de usuario: {error}"))?;
    broadcast_environment_change();
    Ok(())
}

#[cfg(windows)]
fn broadcast_environment_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    let environment: Vec<u16> = "Environment\0".encode_utf16().collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            environment.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5_000,
            std::ptr::null_mut(),
        );
    }
}

#[tauri::command]
pub(crate) fn cli_install_status(app: AppHandle) -> Result<CliInstallStatus, String> {
    let binary = cli_binary_path(&app)?;
    let binary_exists = binary.is_file();
    let user_path = read_user_path().unwrap_or_default();
    let directory = binary.parent().unwrap_or(&binary);
    Ok(CliInstallStatus {
        binary_exists,
        binary_path: binary.display().to_string(),
        path_configured: path_has_segment(&user_path, &directory.display().to_string()),
        version: binary_exists.then(|| cli_version(&binary)).flatten(),
    })
}

#[tauri::command]
pub(crate) fn repair_cli_path(app: AppHandle) -> Result<(), String> {
    let binary = cli_binary_path(&app)?;
    if !binary.is_file() {
        return Err(format!("IA Usage CLI no existe en {}", binary.display()));
    }
    let directory = binary
        .parent()
        .ok_or_else(|| "La ruta de IA Usage CLI no tiene directorio".to_string())?;
    let current = read_user_path()?;
    let repaired = add_path_segment(&current, &directory.display().to_string());
    #[cfg(windows)]
    write_user_path(&repaired)?;
    #[cfg(not(windows))]
    return Err("La reparación automática de PATH solo está disponible en Windows".into());
    #[cfg(windows)]
    Ok(())
}

#[tauri::command]
pub(crate) fn cli_test(app: AppHandle) -> Result<String, String> {
    let binary = cli_binary_path(&app)?;
    if !binary.is_file() {
        return Err(format!("IA Usage CLI no existe en {}", binary.display()));
    }
    cli_version(&binary).ok_or_else(|| "IA Usage CLI no respondió correctamente".to_string())
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

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.split('-').next().unwrap_or("0").parse().unwrap_or(0))
        .collect()
}

/// Fases estables del updater emitidas al frontend vía `updater-status`.
/// El protocolo IPC transporta el identificador snake_case, nunca el texto
/// localizado: el frontend mapea cada fase a su clave i18n.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum UpdaterPhase {
    Downloading,
    Verifying,
    Preparing,
    Restarting,
}

fn emit_updater_phase(app: &AppHandle, phase: UpdaterPhase) {
    // El progreso es informativo: un listener ausente o una ventana ya
    // cerrada nunca debe abortar la actualización.
    let _ = app.emit("updater-status", phase);
}

/// Escapa una ruta Windows para interpolarla como literal single-quoted de
/// PowerShell (`'` → `''`). Se aplica a installer, current_exe y log.
fn ps_quote(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "''"))
}

/// Extrae el hash SHA-256 publicado para `asset_name` desde el contenido de
/// `SHA256SUMS.txt`. Tolera el prefijo `*` de modo binario y exige 64
/// dígitos hexadecimales. Función pura para poder testearla sin red.
fn parse_checksum(body: &str, asset_name: &str) -> Option<String> {
    body.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let hash = fields.next()?;
            let name = fields.next()?;
            // GitHub sustituye espacios por puntos al subir assets; el
            // checksum ya viene con el nombre con puntos, igual que la API.
            (name.trim_start_matches('*') == asset_name).then(|| hash.to_ascii_lowercase())
        })
        .next()
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// Sanea una versión para usarla en nombres de fichero del helper/log.
fn sanitize_version_tag(tag: &str) -> String {
    let trimmed = tag.trim_start_matches('v').trim();
    let mut out: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push_str("unknown");
    }
    // Evita `..` o separadores accidentales aunque el tag viniera corrupto.
    out.replace("..", "__")
}

fn updater_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("iausagebar-update")
}

/// Construye el script PowerShell del helper de relanzamiento.
///
/// El helper es un proceso independiente que sobrevive a `app.exit(0)`:
/// espera a que el PID padre termine de verdad (no un `Sleep` mágico),
/// ejecuta el instalador silencioso con `-Wait`, comprueba su `ExitCode`
/// y solo entonces relanza `current_exe` si sigue existiendo.
fn build_relaunch_ps1(
    installer: &std::path::Path,
    current_exe: &std::path::Path,
    log: &std::path::Path,
    parent_pid: u32,
) -> String {
    let installer_q = ps_quote(installer);
    let app_q = ps_quote(current_exe);
    let log_q = ps_quote(log);
    format!(
        "$Installer = {installer_q}\r\n\
         $App = {app_q}\r\n\
         $Log = {log_q}\r\n\
         $ParentPid = {parent_pid}\r\n\
         Add-Content -Path $Log -Value \"helper started\"\r\n\
         try {{\r\n\
         \x20   Wait-Process -Id $ParentPid -Timeout 30 -ErrorAction SilentlyContinue\r\n\
         }} catch {{}}\r\n\
         Start-Sleep -Milliseconds 300\r\n\
         $proc = Start-Process -FilePath $Installer -ArgumentList \"/S\" -PassThru -Wait\r\n\
         Add-Content -Path $Log -Value (\"installer exit code: \" + $proc.ExitCode)\r\n\
         if ($proc.ExitCode -eq 0) {{\r\n\
         \x20   Add-Content -Path $Log -Value \"installer ok, relaunching\"\r\n\
         \x20   Start-Process -FilePath $App\r\n\
         }} else {{\r\n\
         \x20   Add-Content -Path $Log -Value \"installer failed\"\r\n\
         \x20   if (Test-Path $App) {{\r\n\
         \x20       Add-Content -Path $Log -Value \"previous exe still present, relaunching\"\r\n\
         \x20       Start-Process -FilePath $App\r\n\
         \x20   }}\r\n\
         }}\r\n"
    )
}

fn download_verified_installer(
    app: &AppHandle,
    release: &GitHubRelease,
) -> Result<std::path::PathBuf, String> {
    let installer = windows_installer(release)?;
    let asset_name = installer.name.clone();
    let download_url = installer.browser_download_url.clone();
    emit_updater_phase(app, UpdaterPhase::Downloading);
    crate::logfile::append(&format!("updater: downloading {asset_name}"));
    let expected_hash = published_checksum(release, &asset_name)?;
    let file_name = std::path::Path::new(&asset_name)
        .file_name()
        .ok_or_else(|| "Nombre de instalador no válido".to_string())?;
    let dir = updater_dir();
    std::fs::create_dir_all(&dir).map_err(|_| "No se pudo preparar la descarga".to_string())?;
    let path = dir.join(file_name);
    let mut response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent("IA-Usage-Bar")
        .build()
        .map_err(|_| "No se pudo preparar la descarga".to_string())?
        .get(&download_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|_| "No se pudo descargar el instalador".to_string())?;
    let mut file =
        File::create(&path).map_err(|_| "No se pudo guardar el instalador".to_string())?;
    response
        .copy_to(&mut file)
        .map_err(|_| "La descarga del instalador se interrumpió".to_string())?;
    drop(file);
    emit_updater_phase(app, UpdaterPhase::Verifying);
    crate::logfile::append(&format!("updater: verifying {asset_name}"));
    let actual = file_sha256(&path)?;
    if actual != expected_hash {
        let _ = std::fs::remove_file(&path);
        crate::logfile::append("updater: checksum mismatch");
        return Err("La verificación de seguridad del instalador falló".into());
    }
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    crate::logfile::append(&format!("updater: verified installer ({bytes} bytes)"));
    Ok(path)
}

/// CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP. Nunca DETACHED_PROCESS:
/// powershell.exe sin consola termina antes de ejecutar el script, así que el
/// instalador nunca corría (ningún `updater-*.log` llegó a escribirse).
#[cfg(windows)]
const HELPER_SPAWN_FLAGS: u32 = 0x0800_0000 | 0x0000_0200;
#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

/// Escribe el helper `.ps1` y lo lanza en segundo plano, sin ventana, para
/// que sobreviva a `app.exit(0)`. Devuelve `Err` sin cerrar la app si algo falla.
fn spawn_relaunch_helper(
    app: &AppHandle,
    installer: &std::path::Path,
    current_exe: &std::path::Path,
    version_tag: &str,
) -> Result<(), String> {
    emit_updater_phase(app, UpdaterPhase::Preparing);
    crate::logfile::append("updater: preparing relaunch helper");
    let dir = updater_dir();
    std::fs::create_dir_all(&dir).map_err(|_| "No se pudo preparar la instalación".to_string())?;
    let safe = sanitize_version_tag(version_tag);
    let ps1_path = dir.join(format!("relaunch-{safe}.ps1"));
    let log_path = dir.join(format!("updater-{safe}.log"));
    let parent_pid = std::process::id();
    let script = build_relaunch_ps1(installer, current_exe, &log_path, parent_pid);
    std::fs::write(&ps1_path, script)
        .map_err(|_| "No se pudo preparar la instalación".to_string())?;
    crate::logfile::append("updater: relaunch helper created");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let ps1 = ps1_path.to_string_lossy().into_owned();
        let spawn = |flags: u32| {
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-WindowStyle",
                    "Hidden",
                    "-File",
                    ps1.as_str(),
                ])
                .creation_flags(flags)
                .spawn()
        };
        // La app corre dentro de un Job de Windows: breakaway para que un job
        // kill-on-close no mate al helper con `app.exit(0)`. Un job que no
        // permite breakaway hace fallar el spawn, así que se reintenta dentro.
        spawn(HELPER_SPAWN_FLAGS | CREATE_BREAKAWAY_FROM_JOB)
            .or_else(|_| spawn(HELPER_SPAWN_FLAGS))
            .map_err(|_| "No se pudo preparar la instalación".to_string())?;
    }
    #[cfg(not(windows))]
    {
        let _ = (&ps1_path, &log_path, parent_pid);
        return Err("La actualización automática solo está disponible en Windows".into());
    }
    crate::logfile::append("updater: helper spawned");
    Ok(())
}

#[tauri::command]
pub(crate) fn check_for_updates(app: AppHandle) -> Result<UpdateCheck, String> {
    let current = app.package_info().version.to_string();
    let release = latest_release()?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    Ok(UpdateCheck {
        update_available: version_parts(&latest) > version_parts(&current),
        current,
        latest,
        url: release.html_url,
    })
}

fn latest_release() -> Result<GitHubRelease, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("IA-Usage-Bar")
        .build()
        .map_err(|_| "No se pudo preparar la búsqueda de actualizaciones".to_string())?;
    client
        .get("https://api.github.com/repos/Shadelight/ia-usage-bar/releases/latest")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|_| "No se pudo consultar la última versión".to_string())?
        .json()
        .map_err(|_| "La respuesta de actualización no es válida".to_string())
}

fn windows_installer(release: &GitHubRelease) -> Result<&ReleaseAsset, String> {
    release
        .assets
        .iter()
        .find(|asset| asset.name.to_ascii_lowercase().ends_with(".exe"))
        .ok_or_else(|| "La versión publicada no incluye un instalador de Windows".to_string())
}

fn published_checksum(release: &GitHubRelease, asset_name: &str) -> Result<String, String> {
    let checksums = release
        .assets
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case("SHA256SUMS.txt"))
        .ok_or_else(|| "La versión publicada no incluye SHA256SUMS.txt".to_string())?;
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("IA-Usage-Bar")
        .build()
        .map_err(|_| "No se pudo preparar la verificación de actualización".to_string())?
        .get(&checksums.browser_download_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|_| "No se pudo descargar SHA256SUMS.txt".to_string())?
        .text()
        .map_err(|_| "No se pudo leer SHA256SUMS.txt".to_string())?;
    parse_checksum(&body, asset_name).ok_or_else(|| format!("No hay checksum para {asset_name}"))
}

fn file_sha256(path: &std::path::Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|_| "No se pudo abrir el instalador descargado".to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "No se pudo verificar el instalador descargado".to_string())?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[tauri::command]
pub(crate) fn install_update(app: AppHandle) -> Result<(), String> {
    let current = app.package_info().version.to_string();
    let release = latest_release()?;
    let latest_tag = release.tag_name.clone();
    if version_parts(latest_tag.trim_start_matches('v')) <= version_parts(&current) {
        return Err("Ya estás usando la versión más reciente".into());
    }
    // 1-2. Descarga + verificación SHA-256. Cualquier fallo devuelve Err
    // con la app todavía abierta: nunca se cierra en este punto.
    let installer_path = download_verified_installer(&app, &release)?;
    // 3. Ruta instalada actual: el helper la relanzará cuando NSIS termine.
    let current_exe = std::env::current_exe()
        .map_err(|_| "No se pudo localizar la aplicación instalada".to_string())?;
    // 4. Helper externo desacoplado. Si no se puede crear/lanzar, NO salir.
    spawn_relaunch_helper(&app, &installer_path, &current_exe, &latest_tag)?;
    // 5. Salida limpia: marcar allow_exit para no chocar con el
    // comportamiento tray (CloseRequested/ExitRequested la ocultarían).
    emit_updater_phase(&app, UpdaterPhase::Restarting);
    crate::logfile::append("updater: exiting for update");
    app.state::<AppState>()
        .allow_exit
        .store(true, Ordering::SeqCst);
    app.exit(0);
    Ok(())
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
    server_error: Option<String>,
    last_export: Option<SyncExportInfo>,
    paired_devices: Vec<PairedDeviceDto>,
    pending_pairing: Option<PendingPairingDto>,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairedDeviceDto {
    client_device_id: String,
    name: String,
    created_at: String,
    last_seen_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingPairingDto {
    fingerprint: String,
    expires_at: String,
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
    let paired_devices = cfg
        .paired_devices
        .iter()
        .filter(|d| !d.revoked)
        .map(|d| PairedDeviceDto {
            client_device_id: d.client_device_id.clone(),
            name: d.name.clone(),
            created_at: d.created_at.clone(),
            last_seen_at: d.last_seen_at.clone(),
        })
        .collect();
    let pending_pairing = lock_or_recover(&state.pending_pairing)
        .as_ref()
        .filter(|p| !p.core.is_expired())
        .map(|p| PendingPairingDto {
            fingerprint: crate::sync::pairing_fingerprint(&device_id),
            expires_at: p.expires_at_iso.clone(),
        });
    Ok(SyncStatus {
        enabled: cfg.sync_enabled,
        fingerprint: crate::sync::pairing_fingerprint(&device_id),
        device_id: device_id.clone(),
        export_dir: crate::sync::resolve_export_dir(&cfg).display().to_string(),
        has_passphrase: crate::sync::has_passphrase(),
        lan: cfg.sync_lan,
        server_running: server.running,
        server_addr: server.addr.clone(),
        server_error: server.last_error.clone(),
        last_export: sync_last_export(&cfg, &device_id),
        paired_devices,
        pending_pairing,
    })
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
    crate::sync_service::ensure_sync_server(&app)
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
    let previous = cfg.sync_lan;
    cfg.sync_lan = lan;
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg.clone();
    // Rebind: el bind (loopback vs 0.0.0.0) exige reiniciar el hilo.
    if let Err(error) = crate::sync_service::ensure_sync_server(&app) {
        cfg.sync_lan = previous;
        cfg.save()?;
        *lock_or_recover(&state.config) = cfg;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn sync_get_pairing(
    state: tauri::State<AppState>,
    lan: bool,
) -> Result<SyncPairing, String> {
    if !lan {
        return Err("sync: la vinculación móvil requiere la red local".into());
    }
    let cfg = lock_or_recover(&state.config).clone();
    if !crate::sync::has_passphrase() {
        return Err("sync: primero guarda una frase secreta".into());
    }
    if !cfg.sync_lan {
        return Err("sync: activa Exponer en la red local".into());
    }
    if !cfg.sync_enabled {
        return Err("sync: activa la sincronización antes de vincular".into());
    }
    {
        let server = lock_or_recover(&state.sync_server);
        if !server.running || !server.lan {
            return Err("sync: el servidor LAN todavía no está listo".into());
        }
    }
    let device_id = crate::sync::load_or_create_device_id()?;
    let host =
        crate::sync_server::lan_ip().ok_or_else(|| "sync: sin IP LAN detectable".to_string())?;
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
pub(crate) fn sync_start_pairing(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<SyncPairing, String> {
    let cfg = lock_or_recover(&state.config).clone();
    if !cfg.sync_lan {
        return Err("sync: activa Exponer en la red local".into());
    }
    let device_id = crate::sync::load_or_create_device_id()?;
    let new_pairing = iausage_core::pairing::start_pairing();
    let expires_at_iso = (chrono::Local::now()
        + chrono::Duration::from_std(iausage_core::pairing::PAIRING_TTL).unwrap())
    .to_rfc3339();
    *lock_or_recover(&state.pending_pairing) = Some(crate::sync_service::PendingPairingRuntime {
        core: new_pairing.pending,
        expires_at_iso,
    });
    crate::sync_service::ensure_sync_server(&app)?;
    let host =
        crate::sync_server::lan_ip().ok_or_else(|| "sync: sin IP LAN detectable".to_string())?;
    let fingerprint = crate::sync::pairing_fingerprint(&device_id);
    let info = iausage_core::pairing::PairingInfoV2 {
        host: host.clone(),
        port: crate::sync_server::SYNC_DEFAULT_PORT,
        pc_device_id: device_id,
        fingerprint: fingerprint.clone(),
        token: new_pairing.token_hex,
        secret: new_pairing.secret_b64,
    };
    let uri = info.to_uri();
    let png = crate::sync_server::pairing_qr_png(&uri, 512)?;
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    Ok(SyncPairing {
        // The v2 URI carries the device secret. The QR image is already
        // rendered server-side from it and sent as PNG bytes below; nothing
        // on the frontend reads `.uri` for the v2 pairing flow (only
        // `.qrPngBase64`/`.host`/`.port`/`.fingerprint`), so the raw
        // secret-bearing string must not sit in the webview at all. `uri`
        // stays on the shared `SyncPairing` struct only because V1's
        // `sync_get_pairing` (no secret in its URI) still needs it.
        uri: String::new(),
        fingerprint,
        host,
        port: info.port,
        qr_png_base64: B64.encode(&png),
    })
}

#[tauri::command]
pub(crate) fn sync_revoke_device(
    app: AppHandle,
    state: tauri::State<AppState>,
    client_device_id: String,
) -> Result<(), String> {
    {
        // One critical section for the whole read-modify-write-save (Fix
        // 6): the guard must be dropped before `ensure_sync_server` below
        // takes the same lock again, so this mutation is scoped to a block.
        let mut cfg = lock_or_recover(&state.config);
        crate::config::revoke_paired_device(
            &mut cfg,
            &crate::config::OS_CREDENTIAL_STORE,
            &client_device_id,
            |updated| updated.save(),
        )?;
    }
    // Revocar el último dispositivo (sin V1 activo ni pairing pendiente)
    // debe apagar el servidor de inmediato, no esperar el próximo tick.
    crate::sync_service::ensure_sync_server(&app)?;
    Ok(())
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
    use super::{
        add_path_segment, build_relaunch_ps1, parse_checksum, path_has_segment,
        provider_login_command, ps_quote, sanitize_version_tag, version_parts, UpdaterPhase,
        CLI_RESOURCE_RELATIVE_PATH,
    };
    use crate::model::VendorId;

    #[test]
    fn compares_release_versions_numerically() {
        assert!(version_parts("v0.10.0") > version_parts("0.2.9"));
        assert_eq!(version_parts("v0.2.0"), vec![0, 2, 0]);
        // Casos borde: prefijo v, sufijos pre-release, longitudes distintas.
        assert_eq!(version_parts("1.2"), vec![1, 2]);
        assert!(version_parts("v1.2.0") > version_parts("1.2"));
        assert_eq!(version_parts("v1.0.0-beta"), version_parts("1.0.0"));
        assert!(version_parts("v0.2.8") > version_parts("v0.2.7"));
        assert!(!(version_parts("v0.2.7") > version_parts("v0.2.7")));
    }

    #[test]
    fn user_path_matches_complete_segments_only() {
        let cli = r"C:\Program Files\IA Usage Bar\resources\bin";
        assert!(!path_has_segment("", cli));
        assert!(path_has_segment(
            &format!(r"C:\Windows;{cli};C:\Tools"),
            cli
        ));
        assert!(path_has_segment(
            r"C:\Windows;c:\program files\ia usage bar\resources\bin\",
            cli
        ));
        assert!(!path_has_segment(
            r"C:\Program Files\IA Usage Bar\resources\binary",
            cli
        ));
    }

    #[test]
    fn cli_status_resolves_the_installer_resource_contract() {
        let path = std::path::Path::new(CLI_RESOURCE_RELATIVE_PATH);
        assert_eq!(path.file_name().unwrap(), "iausage.exe");
        assert_eq!(
            path.parent().unwrap(),
            std::path::Path::new("resources/bin")
        );
    }

    #[test]
    fn adding_cli_path_is_idempotent_and_collapses_duplicates() {
        let cli = r"C:\Apps\IA Usage Bar\resources\bin";
        assert_eq!(add_path_segment("", cli), cli);
        assert_eq!(
            add_path_segment(&format!(r"C:\Tools;{cli};{cli};"), cli),
            format!(r"C:\Tools;{cli}")
        );
        assert_eq!(
            add_path_segment(r"C:\Tools;C:\Apps\IA Usage Bar\resources\binary;", cli),
            format!(r"C:\Tools;C:\Apps\IA Usage Bar\resources\binary;{cli}")
        );
    }

    #[test]
    fn oauth_login_actions_use_the_official_clients() {
        assert_eq!(
            provider_login_command(VendorId::Anthropic),
            Ok(("claude", &[][..]))
        );
        assert_eq!(
            provider_login_command(VendorId::Openai),
            Ok(("codex", &["login"][..]))
        );
        assert!(provider_login_command(VendorId::Cursor).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn updater_helper_is_never_spawned_detached() {
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        assert_eq!(super::HELPER_SPAWN_FLAGS & DETACHED_PROCESS, 0);
        assert_ne!(super::HELPER_SPAWN_FLAGS & CREATE_NO_WINDOW, 0);
    }

    #[test]
    fn updater_selects_only_a_windows_installer() {
        let release = super::GitHubRelease {
            tag_name: "v0.2.2".into(),
            html_url: "https://example.invalid/release".into(),
            assets: vec![
                super::ReleaseAsset {
                    name: "SHA256SUMS.txt".into(),
                    browser_download_url: "https://example.invalid/checksums".into(),
                },
                super::ReleaseAsset {
                    name: "IA_Usage_Bar_0.2.2_x64-setup.exe".into(),
                    browser_download_url: "https://example.invalid/installer".into(),
                },
            ],
        };
        assert_eq!(
            super::windows_installer(&release).unwrap().name,
            "IA_Usage_Bar_0.2.2_x64-setup.exe"
        );
    }

    #[test]
    fn updater_installer_selection_is_case_insensitive_and_exe_only() {
        let release = super::GitHubRelease {
            tag_name: "v0.2.8".into(),
            html_url: "https://example.invalid/release".into(),
            assets: vec![
                super::ReleaseAsset {
                    name: "IA-Usage-0.2.8.msi".into(),
                    browser_download_url: "https://example.invalid/msi".into(),
                },
                super::ReleaseAsset {
                    name: "IA.Usage.Bar_0.2.8_x64-SETUP.EXE".into(),
                    browser_download_url: "https://example.invalid/exe".into(),
                },
            ],
        };
        assert_eq!(
            super::windows_installer(&release).unwrap().name,
            "IA.Usage.Bar_0.2.8_x64-SETUP.EXE"
        );
        let empty = super::GitHubRelease {
            tag_name: "v0.2.8".into(),
            html_url: "https://example.invalid/release".into(),
            assets: vec![super::ReleaseAsset {
                name: "SHA256SUMS.txt".into(),
                browser_download_url: "https://example.invalid/sums".into(),
            }],
        };
        assert!(super::windows_installer(&empty).is_err());
    }

    #[test]
    fn parses_checksums_with_dots_star_and_invalid_hashes() {
        let good = "deadbeef".repeat(8);
        let body = format!(
            "{good}  IA.Usage.Bar_0.2.8_x64-setup.exe\n{good} *other.exe\nshort  bad.exe\n"
        );
        assert_eq!(
            parse_checksum(&body, "IA.Usage.Bar_0.2.8_x64-setup.exe"),
            Some(good.clone())
        );
        assert_eq!(parse_checksum(&body, "other.exe"), Some(good));
        assert_eq!(parse_checksum(&body, "missing.exe"), None);
        assert_eq!(parse_checksum(&body, "bad.exe"), None);
        // Mayúsculas del checksum se normalizan a minúsculas.
        let upper = format!("{}  a.exe\n", "ABCDEF01".repeat(8));
        assert_eq!(parse_checksum(&upper, "a.exe"), Some("abcdef01".repeat(8)));
    }

    #[test]
    fn ps_quote_escapes_single_quotes() {
        let path = std::path::Path::new(r"C:\Users\O'Brien\IA Usage Bar.exe");
        assert_eq!(ps_quote(path), r"'C:\Users\O''Brien\IA Usage Bar.exe'");
    }

    #[test]
    fn sanitize_version_tag_never_escapes_temp_dir() {
        assert_eq!(sanitize_version_tag("v0.2.8"), "0.2.8");
        assert_eq!(sanitize_version_tag(""), "unknown");
        assert!(!sanitize_version_tag("../../evil").contains('/'));
        assert!(!sanitize_version_tag("../../evil").contains('\\'));
        assert!(!sanitize_version_tag("a..b").contains(".."));
    }

    #[test]
    fn relaunch_script_waits_for_parent_checks_exit_code_and_relaunches() {
        let installer = std::path::Path::new(r"C:\Temp\iausagebar-update\setup.exe");
        let exe = std::path::Path::new(r"C:\Program Files\IA Usage Bar\IA Usage Bar.exe");
        let log = std::path::Path::new(r"C:\Temp\iausagebar-update\updater-0.2.8.log");
        let script = build_relaunch_ps1(installer, exe, log, 1234);
        assert!(script.contains("Wait-Process -Id $ParentPid"));
        assert!(script.contains("$ParentPid = 1234"));
        assert!(script.contains("-ArgumentList \"/S\""));
        assert!(script.contains("-PassThru -Wait"));
        assert!(script.contains("installer exit code:"));
        assert!(script.contains("Test-Path $App"));
        // Rutas entrecomilladas con ps_quote, sin secretos.
        assert!(script.contains("'C:\\Temp\\iausagebar-update\\setup.exe'"));
        assert!(script.contains("'C:\\Program Files\\IA Usage Bar\\IA Usage Bar.exe'"));
        assert!(!script.contains("api.github.com"));
    }

    #[test]
    fn updater_phases_serialize_to_stable_snake_case() {
        let phase = serde_json::to_value(UpdaterPhase::Downloading).unwrap();
        assert_eq!(phase, serde_json::json!("downloading"));
        assert_eq!(
            serde_json::to_value(UpdaterPhase::Verifying).unwrap(),
            serde_json::json!("verifying")
        );
        assert_eq!(
            serde_json::to_value(UpdaterPhase::Preparing).unwrap(),
            serde_json::json!("preparing")
        );
        assert_eq!(
            serde_json::to_value(UpdaterPhase::Restarting).unwrap(),
            serde_json::json!("restarting")
        );
    }
}
