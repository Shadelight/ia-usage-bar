//! IA Usage: monitor multi-proveedor para la bandeja de Windows.

//! IA Usage: monitor multi-proveedor para la bandeja de Windows.
//!
//! Todo el pipeline de providers vive en `iausage-core` (compartido con el
//! CLI). Este crate solo conserva ventana/tray/notificaciones/comandos.

pub use iausage_core::{
    cache, config, cost, descriptor, doctor, guard, health, http, jwt, logfile, model, pace,
    paths, pricing, providers, recommend, refresh_policy, snapshot_v1, sync, sync_server, watch,
};
mod commands;
mod dashboard;
mod state;
mod sync_service;
mod tray;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;

use config::AppConfig;
use dashboard::{do_refresh, run_loop, send_notification, start_local_watch};
use state::{lock_or_recover, AppState, TrayMenuState};
use tray::{on_tray_left_click, show_window};

/// Shared by the tray "Detectar proveedores" submenu entry (the old root
/// entry was removed to keep the root menu compact).
fn run_provider_detect(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    // Sin locks durante el detect (ver detect_providers): un probe colgado
    // no debe congelar comandos ni refresh.
    let snapshot = lock_or_recover(&state.config).clone();
    let mut candidate = snapshot;
    match config::run_detect(&mut candidate) {
        Ok(_) => {
            {
                let mut cfg = lock_or_recover(&state.config);
                if !cfg.load_recovered {
                    *cfg = candidate;
                }
            }
            let snapshot = lock_or_recover(&state.config).clone();
            commands::refresh_catalog_and_tray(app, &state, &snapshot);
            do_refresh(app, None);
        }
        Err(error) => eprintln!("provider detection failed: {error}"),
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app, None);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            let boot_started = Instant::now();
            let mut cfg = AppConfig::load();
            #[cfg(debug_assertions)]
            eprintln!("config loaded: {} ms", boot_started.elapsed().as_millis());
            if config::migrate_legacy_credentials(&mut cfg) && !cfg.load_recovered {
                if let Err(error) = cfg.save() {
                    eprintln!("legacy credential migration could not update config: {error}");
                }
            }
            let notifications = cfg.notifications;
            let always_on_top_on = cfg.always_on_top;
            let compact_mode_on = cfg.compact_mode;
            let primary_id = cfg.primary.clone();
            let catalog = providers::catalog(&cfg);
            let cached_snapshots = cache::load();
            #[cfg(debug_assertions)]
            eprintln!(
                "cache loaded: {} ms ({} providers)",
                boot_started.elapsed().as_millis(),
                cached_snapshots.len()
            );
            app.manage(AppState {
                snapshots: Mutex::new(cached_snapshots),
                config: Mutex::new(cfg),
                catalog: Mutex::new(catalog),
                notify: Mutex::new(HashMap::new()),
                notifications_enabled: AtomicBool::new(notifications),
                allow_exit: AtomicBool::new(false),
                last_refresh: Mutex::new(None),
                last_activity: Mutex::new(Instant::now()),
                loading_providers: Mutex::new(std::collections::HashSet::new()),
                app_bootstrapping: AtomicBool::new(true),
                refreshing: AtomicBool::new(false),
                rerun_requested: AtomicBool::new(false),
                backoff_until: Mutex::new(HashMap::new()),
                sync_server: Mutex::new(sync_service::SyncServerState::default()),
            });

            let tray_header =
                MenuItem::with_id(app, "tray_header", "IA Usage", false, None::<&str>)?;
            let open_i = MenuItem::with_id(app, "open", "Abrir IA Usage", true, None::<&str>)?;
            let refresh_i =
                MenuItem::with_id(app, "refresh", "Actualizar ahora", true, None::<&str>)?;
            let pair_phone_i =
                MenuItem::with_id(app, "pair_phone", "Vincular teléfono", true, None::<&str>)?;
            let primary_submenu = Submenu::with_id(app, "provider", "Proveedor", true)?;
            let compact_i = CheckMenuItem::with_id(
                app,
                "compact",
                "Modo compacto",
                true,
                compact_mode_on,
                None::<&str>,
            )?;
            let pin_i = CheckMenuItem::with_id(
                app,
                "pin",
                "Siempre visible",
                true,
                always_on_top_on,
                None::<&str>,
            )?;
            let settings_i =
                MenuItem::with_id(app, "settings", "Configuración", true, None::<&str>)?;
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
            let about_i = MenuItem::with_id(app, "about", "Acerca de", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[
                    &tray_header,
                    &sep1,
                    &open_i,
                    &refresh_i,
                    &pair_phone_i,
                    &primary_submenu,
                    &compact_i,
                    &pin_i,
                    &autostart_i,
                    &sep2,
                    &settings_i,
                    &about_i,
                    &quit_i,
                ],
            )?;
            app.manage(TrayMenuState {
                header: tray_header,
                autostart: autostart_i,
                always_on_top: pin_i,
                compact_mode: compact_i,
                primary_submenu,
            });
            {
                let state = app.state::<AppState>();
                let catalog = lock_or_recover(&state.catalog).clone();
                let snapshots = lock_or_recover(&state.snapshots).clone();
                tray::rebuild_provider_submenu(app.handle(), &catalog, &primary_id, &snapshots);
            }

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray::render(None))
                .tooltip("IA Usage")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_window(app, None),
                    "settings" => {
                        show_window(app, None);
                        let _ = app.emit("tray-cmd", "settings");
                    }
                    "about" => {
                        show_window(app, None);
                        let _ = app.emit("tray-cmd", "settings:about");
                    }
                    "manage_providers" => {
                        show_window(app, None);
                        let _ = app.emit("tray-cmd", "settings:providers");
                    }
                    "refresh" => do_refresh(app, None),
                    "pair_phone" => {
                        show_window(app, None);
                        let _ = app.emit("tray-cmd", "pair-phone");
                    }
                    "detect_submenu" => run_provider_detect(app),
                    "autostart" => {
                        let al = app.autolaunch();
                        if al.is_enabled().unwrap_or(false) {
                            let _ = al.disable();
                        } else {
                            let _ = al.enable();
                        }
                    }
                    "compact" => {
                        let state = app.state::<AppState>();
                        let checked = app
                            .try_state::<TrayMenuState>()
                            .map(|m| !m.compact_mode.is_checked().unwrap_or(false))
                            .unwrap_or(false);
                        let _ = commands::apply_compact_mode(app, &state, checked);
                    }
                    "pin" => {
                        let state = app.state::<AppState>();
                        let checked = app
                            .try_state::<TrayMenuState>()
                            .map(|m| !m.always_on_top.is_checked().unwrap_or(false))
                            .unwrap_or(false);
                        let _ = commands::apply_always_on_top(app, &state, checked);
                    }
                    "quit" => {
                        app.state::<AppState>()
                            .allow_exit
                            .store(true, Ordering::SeqCst);
                        app.exit(0);
                    }
                    other => {
                        if let Some(id) = other.strip_prefix("primary_") {
                            let state = app.state::<AppState>();
                            let _ = commands::set_primary(app, &state, id);
                        }
                    }
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
                let _ = win.set_always_on_top(always_on_top_on);
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
                        "IA Usage activo",
                        "Te avisaré cuando un plan se acerque al límite o se reinicie.",
                    );
                }
            }

            let h1 = app.handle().clone();
            std::thread::spawn(move || run_loop(h1));

            // M6 IDE/GUI: cambios de sesiones Codex actualizan el dashboard
            // con debounce sin adelantar el polling de proveedores remotos.
            start_local_watch(app.handle().clone());

            // Servidor sync M5: solo arranca si está activo en config. Un
            // fallo aquí (passphrase ilegible, puerto ocupado) ya queda en
            // SyncServerState.last_error para que la UI lo muestre.
            if let Err(error) = sync_service::ensure_sync_server(app.handle()) {
                eprintln!("sync server not started at launch: {error}");
            }

            let h2 = app.handle().clone();
            std::thread::spawn(move || {
                #[cfg(debug_assertions)]
                let started = Instant::now();
                let state = h2.state::<AppState>();
                let mut candidate = lock_or_recover(&state.config).clone();
                match config::run_detect(&mut candidate) {
                    Ok(newly) => {
                        *lock_or_recover(&state.config) = candidate.clone();
                        commands::refresh_catalog_and_tray(&h2, &state, &candidate);
                        emit_dashboard_after_detection(&h2);
                        if !newly.is_empty() {
                            do_refresh(&h2, None);
                        }
                    }
                    Err(error) => eprintln!("provider detection failed: {error}"),
                }
                #[cfg(debug_assertions)]
                eprintln!(
                    "provider detection completed: {} ms",
                    started.elapsed().as_millis()
                );
            });

            #[cfg(debug_assertions)]
            eprintln!(
                "app setup completed: {} ms",
                boot_started.elapsed().as_millis()
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::get_snapshot_v1,
            commands::set_source_preference,
            commands::refresh_now,
            commands::refresh_provider,
            commands::start_provider_login,
            commands::detect_providers,
            commands::get_app_config,
            commands::set_app_config,
            commands::set_provider_enabled,
            commands::save_api_key,
            commands::delete_api_key,
            commands::quit,
            commands::hide_panel,
            commands::set_notifications,
            commands::set_autostart_enabled,
            commands::set_always_on_top,
            commands::set_compact_mode,
            commands::open_logs_folder,
            commands::cli_install_status,
            commands::repair_cli_path,
            commands::cli_test,
            commands::clear_logs,
            commands::export_diagnostics,
            commands::check_for_updates,
            commands::install_update,
            commands::sync_get_status,
            commands::sync_set_enabled,
            commands::sync_set_passphrase,
            commands::sync_set_export_dir,
            commands::sync_set_lan,
            commands::sync_get_pairing,
            commands::sync_export_now,
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
        .expect("error al iniciar IA Usage")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !app.state::<AppState>().allow_exit.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}

fn emit_dashboard_after_detection(app: &tauri::AppHandle) {
    dashboard::emit_dashboard(app);
}
