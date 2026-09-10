//! IA Usage Bar: monitor multi-proveedor para la bandeja de Windows.

mod config;
mod cost;
mod http;
mod jwt;
mod model;
mod paths;
mod pricing;
mod providers;
mod tray_icon;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use config::AppConfig;
use model::{most_headroom, monthly_spend, Dashboard, ProviderSnapshot, SpendRow, VendorId};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent, Wry,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

struct AppState {
    snapshots: Mutex<HashMap<String, ProviderSnapshot>>,
    config: Mutex<AppConfig>,
    notify: Mutex<HashMap<String, NotifyState>>,
    notifications_enabled: AtomicBool,
    allow_exit: AtomicBool,
    last_refresh: Mutex<Option<Instant>>,
    /// True while a refresh fan-out is in flight (F-H3 overlap guard).
    refreshing: AtomicBool,
    /// Set when a refresh was requested while `refreshing` was held; consumed
    /// as a single coalesced rerun when the in-flight refresh finishes.
    rerun_requested: AtomicBool,
    backoff_until: Mutex<HashMap<String, Instant>>,
}

/// Lock a mutex, tolerating poisoning instead of cascading the panic. A single
/// transient panic while a lock is held must not permanently kill refreshing.
fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// F-H3 overlap guard: returns `true` for exactly one caller at a time. The
/// winner must `flag.store(false, …)` when its refresh finishes.
fn claim_refresh(flag: &AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

struct TrayMenuState {
    header: MenuItem<Wry>,
    status: MenuItem<Wry>,
}

#[derive(Default, Clone)]
struct NotifyState {
    initialized: bool,
    prev_resets: Option<String>,
    notified_75: bool,
    notified_90: bool,
    notified_limit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsageAlert {
    At75,
    At90,
    At95,
}

#[tauri::command]
fn get_dashboard(app: AppHandle, state: tauri::State<AppState>) -> Dashboard {
    build_dashboard(&app, &state)
}

#[tauri::command]
fn refresh_now(app: AppHandle) {
    do_refresh(&app, None);
}

#[tauri::command]
fn refresh_provider(app: AppHandle, id: String) {
    do_refresh(&app, Some(id));
}

#[tauri::command]
fn detect_providers(app: AppHandle, state: tauri::State<AppState>) -> Vec<String> {
    let mut cfg = lock_or_recover(&state.config).clone();
    let newly = config::run_detect(&mut cfg);
    *lock_or_recover(&state.config) = cfg;
    do_refresh(&app, None);
    newly
}

#[tauri::command]
fn get_app_config(state: tauri::State<AppState>) -> AppConfig {
    lock_or_recover(&state.config).clone()
}

#[tauri::command]
fn set_app_config(app: AppHandle, state: tauri::State<AppState>, cfg: AppConfig) {
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
fn set_provider_enabled(app: AppHandle, state: tauri::State<AppState>, id: String, enabled: bool) {
    let mut cfg = lock_or_recover(&state.config);
    if let Some(vid) = parse_id(&id) {
        cfg.set_enabled(vid, enabled);
        let _ = cfg.save();
    }
    drop(cfg);
    do_refresh(&app, None);
}

#[tauri::command]
fn save_api_key(app: AppHandle, state: tauri::State<AppState>, id: String, key: String) {
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
fn quit(app: AppHandle) {
    app.state::<AppState>()
        .allow_exit
        .store(true, Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
fn hide_panel(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn set_notifications(state: tauri::State<AppState>, enabled: bool) {
    state
        .notifications_enabled
        .store(enabled, Ordering::Relaxed);
    let mut cfg = lock_or_recover(&state.config);
    cfg.notifications = enabled;
    let _ = cfg.save();
}

fn parse_id(s: &str) -> Option<VendorId> {
    VendorId::all()
        .iter()
        .copied()
        .find(|id| id.slug() == s)
}

fn build_dashboard(_app: &AppHandle, state: &AppState) -> Dashboard {
    let cfg = lock_or_recover(&state.config).clone();
    let snaps = lock_or_recover(&state.snapshots);
    let providers: Vec<ProviderSnapshot> = cfg
        .enabled_ids()
        .into_iter()
        .filter_map(|id| snaps.get(id.slug()).cloned())
        .collect();
    let elapsed = lock_or_recover(&state.last_refresh)
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);
    let interval = cfg.refresh_minutes.saturating_mul(60);
    let mut spend = Vec::new();
    let mut spend_month_usd = 0.0;
    for p in &providers {
        if let Some((label, usd)) = monthly_spend(p) {
            spend_month_usd += usd;
            spend.push(SpendRow {
                id: p.id.clone(),
                name: p.name.clone(),
                label,
                usd,
            });
        }
    }
    let (recommend_id, recommend_name, recommend_left) = match most_headroom(&providers) {
        Some((id, name, left)) => (Some(id), Some(name), Some(left)),
        None => (None, None, None),
    };
    Dashboard {
        providers,
        catalog: providers::catalog(&cfg),
        refresh_minutes: cfg.refresh_minutes,
        primary: cfg.primary.clone(),
        notifications: cfg.notifications,
        show_usage_as: cfg.show_usage_as.clone(),
        reset_times: cfg.reset_times.clone(),
        next_update_in_secs: interval.saturating_sub(elapsed),
        spend_month_usd,
        spend,
        recommend_id,
        recommend_name,
        recommend_left,
    }
}

fn emit_dashboard(app: &AppHandle) {
    let dash = build_dashboard(app, &app.state::<AppState>());
    update_tray_from_dashboard(app, &dash);
    let _ = app.emit("dashboard-updated", dash);
}

fn position_window(win: &tauri::WebviewWindow, anchor_x: f64, anchor_y: f64) {
    let size = win
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(360, 500));
    let w = size.width as f64;
    let h = size.height as f64;
    let mut x = anchor_x - w + 12.0;
    let mut y = anchor_y - h - 12.0;

    if let Ok(Some(mon)) = win.current_monitor() {
        let mp = mon.position();
        let ms = mon.size();
        let left = mp.x as f64;
        let top = mp.y as f64;
        let right = left + ms.width as f64;
        let bottom = top + ms.height as f64;
        if x + w > right {
            x = right - w - 4.0;
        }
        if x < left {
            x = left + 4.0;
        }
        if y + h > bottom {
            y = bottom - h - 4.0;
        }
        if y < top {
            y = top + 4.0;
        }
    } else {
        if x < 0.0 {
            x = anchor_x + 12.0;
        }
        if y < 0.0 {
            y = anchor_y + 12.0;
        }
    }
    let _ = win.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
}

fn show_window(app: &AppHandle, anchor: Option<(f64, f64)>) {
    if let Some(win) = app.get_webview_window("main") {
        if let Some((x, y)) = anchor {
            position_window(&win, x, y);
        }
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn on_tray_left_click(app: &AppHandle, x: f64, y: f64) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        let minimized = win.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = win.hide();
        } else {
            show_window(app, Some((x, y)));
        }
    }
}

fn tooltip(dash: &Dashboard) -> String {
    let bits: Vec<String> = dash
        .providers
        .iter()
        .filter(|p| p.connected)
        .filter_map(|p| {
            p.primary_utilization
                .map(|u| format!("{} {:.0}%", p.short, u))
        })
        .take(4)
        .collect();
    if bits.is_empty() {
        "IA Usage Bar".into()
    } else {
        format!("IA Usage Bar — {}", bits.join(" · "))
    }
}

fn tray_percent(dash: &Dashboard) -> Option<f64> {
    let primary = dash
        .providers
        .iter()
        .find(|p| p.id == dash.primary)
        .and_then(|p| p.primary_utilization);
    if primary.is_some() {
        return primary;
    }
    dash.providers
        .iter()
        .filter_map(|p| p.primary_utilization)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn update_tray_from_dashboard(app: &AppHandle, dash: &Dashboard) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(tray_icon::render(tray_percent(dash))));
        let _ = tray.set_tooltip(Some(tooltip(dash)));
    }
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let n = dash.providers.iter().filter(|p| p.connected).count();
        let _ = menu
            .header
            .set_text(format!("IA Usage Bar — {n} proveedores"));
        let _ = menu.status.set_text(tooltip(dash));
    }
}

fn send_notification(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn parse_ts(s: &Option<String>) -> Option<i64> {
    s.as_ref()
        .and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok())
        .map(|d| d.timestamp())
}

fn reset_happened(prev: &Option<String>, cur: &Option<String>) -> bool {
    match (parse_ts(prev), parse_ts(cur)) {
        (Some(p), Some(c)) => (c - p) > 120,
        _ => false,
    }
}

fn take_usage_alerts(util: f64, ns: &mut NotifyState, window_reset: bool) -> Vec<UsageAlert> {
    if window_reset {
        ns.notified_75 = false;
        ns.notified_90 = false;
        ns.notified_limit = false;
    }
    if util < 70.0 {
        ns.notified_75 = false;
        ns.notified_90 = false;
        ns.notified_limit = false;
        return Vec::new();
    }
    let mut out = Vec::new();
    if util >= 75.0 && !ns.notified_75 {
        out.push(UsageAlert::At75);
        ns.notified_75 = true;
    }
    if util >= 90.0 && !ns.notified_90 {
        out.push(UsageAlert::At90);
        ns.notified_90 = true;
    }
    if util >= 95.0 && !ns.notified_limit {
        out.push(UsageAlert::At95);
        ns.notified_limit = true;
    }
    out
}

fn check_notifications(app: &AppHandle, snap: &ProviderSnapshot) {
    let state = app.state::<AppState>();
    if !state.notifications_enabled.load(Ordering::Relaxed) {
        return;
    }
    if !snap.connected {
        return;
    }
    let Some(util) = snap.primary_utilization else {
        return;
    };
    let mut map = lock_or_recover(&state.notify);
    let ns = map.entry(snap.id.clone()).or_default();
    let cur = snap.lines.iter().find_map(|l| l.resets_at().map(|s| s.to_string()));
    if ns.initialized {
        let reset = reset_happened(&ns.prev_resets, &cur);
        if reset {
            send_notification(
                app,
                &format!("{} reiniciado", snap.name),
                "Tu límite se reinició. Listo para seguir.",
            );
        }
        for alert in take_usage_alerts(util, ns, reset) {
            let (title, body) = match alert {
                UsageAlert::At75 => (
                    format!("{} al 75%", snap.name),
                    format!("Vas al {:.0}% de tu cuota. Queda margen, pero ya está cerca.", util),
                ),
                UsageAlert::At90 => (
                    format!("{} al 90%", snap.name),
                    format!("Llegaste al {:.0}%. Conviene cambiar de herramienta antes de cortarte.", util),
                ),
                UsageAlert::At95 => (
                    format!("Límite {}", snap.name),
                    format!("Llegaste al {:.0}% de tu cuota.", util),
                ),
            };
            send_notification(app, &title, &body);
        }
    }
    ns.prev_resets = cur;
    ns.initialized = true;
}

fn do_refresh(app: &AppHandle, only: Option<String>) {
    let a = app.clone();
    std::thread::spawn(move || {
        refresh_sync(&a, only.as_deref());
    });
}

fn refresh_sync(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    // F-H3: run at most one refresh fan-out at a time. A request that lands
    // while one is in flight sets a rerun flag instead of spawning a competing
    // fan-out (which could drive concurrent writes to the same auth.json — F-H1).
    if !claim_refresh(&state.refreshing) {
        if only.is_none() {
            state.rerun_requested.store(true, Ordering::Release);
        }
        return;
    }
    state.rerun_requested.store(false, Ordering::Release);

    // F-H4: a panic on this thread must not kill refreshing permanently.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        refresh_once(app, only);
    }));
    state.refreshing.store(false, Ordering::Release);

    if outcome.is_err() {
        eprintln!("refresh_sync: refresh panicked; marking data stale and continuing");
        for snap in lock_or_recover(&state.snapshots).values_mut() {
            snap.stale = true;
        }
        emit_dashboard(app);
    }

    // Coalesced rerun for whatever asked while we were busy.
    if only.is_none() && state.rerun_requested.swap(false, Ordering::AcqRel) {
        do_refresh(app, None);
    }
}

fn refresh_once(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    let cfg = lock_or_recover(&state.config).clone();
    let ids: Vec<VendorId> = if let Some(only) = only {
        parse_id(only).into_iter().collect()
    } else {
        let enabled = cfg.enabled_ids();
        if enabled.is_empty() {
            VendorId::all().to_vec()
        } else {
            enabled
        }
    };
    let now = Instant::now();
    let backoff = lock_or_recover(&state.backoff_until).clone();
    let mut handles = Vec::new();
    for id in ids {
        if let Some(until) = backoff.get(id.slug()) {
            if now < *until {
                continue;
            }
        }
        let cfg = cfg.clone();
        handles.push(std::thread::spawn(move || (id, providers::refresh(id, &cfg))));
    }
    for h in handles {
        if let Ok((id, snap)) = h.join() {
            let rate_limited = snap
                .error
                .as_deref()
                .is_some_and(|e| e.contains("Límite de peticiones"));
            if rate_limited {
                lock_or_recover(&state.backoff_until).insert(
                    id.slug().to_string(),
                    Instant::now() + Duration::from_secs(300),
                );
            }
            let stored = {
                let mut snaps = lock_or_recover(&state.snapshots);
                if snap.stale {
                    if let Some(prev) = snaps.get_mut(id.slug()) {
                        if !prev.lines.is_empty() {
                            prev.stale = true;
                            prev.error = snap.error.clone();
                            prev.updated_at = snap.updated_at.clone();
                            prev.clone()
                        } else {
                            snaps.insert(id.slug().to_string(), snap.clone());
                            snap
                        }
                    } else {
                        snaps.insert(id.slug().to_string(), snap.clone());
                        snap
                    }
                } else {
                    snaps.insert(id.slug().to_string(), snap.clone());
                    snap
                }
            };
            check_notifications(app, &stored);
        }
    }
    *lock_or_recover(&state.last_refresh) = Some(Instant::now());
    emit_dashboard(app);
}

fn run_loop(app: AppHandle) {
    loop {
        refresh_sync(&app, None);
        let mins = lock_or_recover(&app.state::<AppState>().config)
            .refresh_minutes
            .max(1);
        std::thread::sleep(Duration::from_secs(mins * 60));
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app, None);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let mut cfg = AppConfig::load();
            let _ = config::run_detect(&mut cfg);
            let notifications = cfg.notifications;
            app.manage(AppState {
                snapshots: Mutex::new(HashMap::new()),
                config: Mutex::new(cfg),
                notify: Mutex::new(HashMap::new()),
                notifications_enabled: AtomicBool::new(notifications),
                allow_exit: AtomicBool::new(false),
                last_refresh: Mutex::new(None),
                refreshing: AtomicBool::new(false),
                rerun_requested: AtomicBool::new(false),
                backoff_until: Mutex::new(HashMap::new()),
            });

            let tray_header =
                MenuItem::with_id(app, "tray_header", "IA Usage Bar", false, None::<&str>)?;
            let tray_status =
                MenuItem::with_id(app, "tray_status", "○ Iniciando...", false, None::<&str>)?;
            let open_i = MenuItem::with_id(app, "open", "Abrir panel", true, None::<&str>)?;
            let refresh_i =
                MenuItem::with_id(app, "refresh", "Actualizar ahora", true, None::<&str>)?;
            let detect_i =
                MenuItem::with_id(app, "detect", "Detectar proveedores", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Ajustes", true, None::<&str>)?;
            let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
            let autostart_i = CheckMenuItem::with_id(
                app,
                "autostart",
                "Iniciar con Windows",
                true,
                autostart_on,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[
                    &tray_header,
                    &tray_status,
                    &sep1,
                    &open_i,
                    &refresh_i,
                    &detect_i,
                    &settings_i,
                    &sep2,
                    &autostart_i,
                    &quit_i,
                ],
            )?;
            app.manage(TrayMenuState {
                header: tray_header,
                status: tray_status,
            });

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon::render(None))
                .tooltip("IA Usage Bar")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_window(app, None),
                    "settings" => {
                        show_window(app, None);
                        let _ = app.emit("tray-cmd", "settings");
                    }
                    "refresh" => do_refresh(app, None),
                    "detect" => {
                        let state = app.state::<AppState>();
                        let mut cfg = lock_or_recover(&state.config).clone();
                        let _ = config::run_detect(&mut cfg);
                        *lock_or_recover(&state.config) = cfg;
                        do_refresh(app, None);
                    }
                    "autostart" => {
                        let al = app.autolaunch();
                        if al.is_enabled().unwrap_or(false) {
                            let _ = al.disable();
                        } else {
                            let _ = al.enable();
                        }
                    }
                    "quit" => {
                        app.state::<AppState>()
                            .allow_exit
                            .store(true, Ordering::SeqCst);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        on_tray_left_click(tray.app_handle(), position.x, position.y);
                    }
                })
                .build(app)?;

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.center();
                let _ = win.show();
                let _ = win.set_focus();
            }

            if let Ok(cfg_dir) = app.path().app_config_dir() {
                let marker = cfg_dir.join(".initialized");
                if !marker.exists() {
                    let _ = app.autolaunch().enable();
                    let _ = std::fs::create_dir_all(&cfg_dir);
                    let _ = std::fs::write(&marker, b"1");
                    send_notification(
                        app.handle(),
                        "IA Usage Bar activo",
                        "Te avisaré cuando un plan se acerque al límite o se reinicie.",
                    );
                }
            }

            let h1 = app.handle().clone();
            std::thread::spawn(move || run_loop(h1));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard,
            refresh_now,
            refresh_provider,
            detect_providers,
            get_app_config,
            set_app_config,
            set_provider_enabled,
            save_api_key,
            quit,
            hide_panel,
            set_notifications,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if !app.state::<AppState>().allow_exit.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error al iniciar IA Usage Bar")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !app.state::<AppState>().allow_exit.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok, Dashboard, VendorId};

    #[test]
    fn reset_happened_ignores_microsecond_jitter() {
        let a = Some("2099-01-01T00:00:00Z".into());
        let b = Some("2099-01-01T00:00:01Z".into());
        assert!(!reset_happened(&a, &b));
    }

    #[test]
    fn reset_happened_detects_new_window() {
        let a = Some("2099-01-01T00:00:00Z".into());
        let b = Some("2099-01-08T00:00:00Z".into());
        assert!(reset_happened(&a, &b));
    }

    #[test]
    fn reset_happened_ignores_vanished_timestamp() {
        let a = Some("2099-01-01T00:00:00Z".into());
        assert!(!reset_happened(&a, &None));
    }

    #[test]
    fn tooltip_lists_connected_percents() {
        let claude = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct("session", "Sesión", 42.0, None, 18_000, "always")],
        );
        let cursor = snapshot_ok(
            VendorId::Cursor,
            "Ultra",
            vec![progress_pct("total", "Uso", 71.0, None, 2_592_000, "always")],
        );
        let dash = Dashboard {
            providers: vec![claude, cursor],
            primary: "anthropic".into(),
            ..Default::default()
        };
        let tip = tooltip(&dash);
        assert!(tip.contains("CLD"));
        assert!(tip.contains("42"));
        assert!(tip.contains("CUR"));
        assert!(tip.contains("71"));
    }

    #[test]
    fn tray_percent_prefers_primary() {
        let claude = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct("session", "Sesión", 10.0, None, 18_000, "always")],
        );
        let cursor = snapshot_ok(
            VendorId::Cursor,
            "Ultra",
            vec![progress_pct("total", "Uso", 90.0, None, 2_592_000, "always")],
        );
        let dash = Dashboard {
            providers: vec![claude, cursor],
            primary: "anthropic".into(),
            ..Default::default()
        };
        assert_eq!(tray_percent(&dash).map(|n| n.round()), Some(10.0));
    }

    // F-H3: two threads racing the refresh flag — exactly one wins the claim,
    // the loser is turned away (it will set the rerun flag instead).
    #[test]
    fn claim_refresh_admits_exactly_one() {
        use std::sync::Arc;
        let flag = Arc::new(AtomicBool::new(false));
        let a = flag.clone();
        let b = flag.clone();
        let ha = std::thread::spawn(move || claim_refresh(&a));
        let hb = std::thread::spawn(move || claim_refresh(&b));
        let (ra, rb) = (ha.join().unwrap(), hb.join().unwrap());
        assert!(ra ^ rb, "exactly one thread claims the refresh");
        // flag stays held until the winner releases it
        assert!(!claim_refresh(&flag));
        flag.store(false, Ordering::Release);
        assert!(claim_refresh(&flag));
    }

    // F-H4: a mutex poisoned by a panicking thread must still be usable.
    #[test]
    fn lock_or_recover_tolerates_poison() {
        use std::sync::Arc;
        let m = Arc::new(Mutex::new(7));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("poison the mutex");
        })
        .join();
        assert!(m.lock().is_err(), "mutex is poisoned");
        assert_eq!(*lock_or_recover(&m), 7);
    }

    #[test]
    fn usage_alerts_fire_at_75_and_90() {
        let mut ns = NotifyState::default();
        assert_eq!(
            take_usage_alerts(76.0, &mut ns, false),
            vec![UsageAlert::At75]
        );
        assert!(take_usage_alerts(80.0, &mut ns, false).is_empty());
        assert_eq!(
            take_usage_alerts(91.0, &mut ns, false),
            vec![UsageAlert::At90]
        );
        assert_eq!(
            take_usage_alerts(96.0, &mut ns, false),
            vec![UsageAlert::At95]
        );
        assert!(take_usage_alerts(50.0, &mut ns, false).is_empty());
        assert!(!ns.notified_75);
        assert_eq!(
            take_usage_alerts(92.0, &mut ns, true),
            vec![UsageAlert::At75, UsageAlert::At90]
        );
    }
}
