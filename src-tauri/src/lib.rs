//! IA Usage Bar: monitor multi-proveedor para la bandeja de Windows.

mod commands;
mod config;
mod cost;
mod dashboard;
mod http;
mod jwt;
mod logfile;
mod model;
mod paths;
mod pricing;
mod providers;
mod state;
mod tray;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;

use config::AppConfig;
use dashboard::{do_refresh, run_loop, send_notification};
use state::{lock_or_recover, AppState, TrayMenuState};
use tray::{on_tray_left_click, show_window};

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
        .setup(|app| {
            let mut cfg = AppConfig::load();
            if config::migrate_legacy_credentials(&mut cfg) && !cfg.load_recovered {
                if let Err(error) = cfg.save() {
                    eprintln!("legacy credential migration could not update config: {error}");
                }
            }
            if let Err(error) = config::run_detect(&mut cfg) {
                eprintln!("provider detection state could not be saved: {error}");
            }
            let notifications = cfg.notifications;
            let always_on_top_on = cfg.always_on_top;
            let compact_mode_on = cfg.compact_mode;
            let primary_id = cfg.primary.clone();
            let catalog = providers::catalog(&cfg);
            app.manage(AppState {
                snapshots: Mutex::new(HashMap::new()),
                config: Mutex::new(cfg),
                catalog: Mutex::new(catalog),
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
            let primary_submenu =
                Submenu::with_id(app, "primary_provider", "Proveedor principal", true)?;
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
                    &primary_submenu,
                    &compact_i,
                    &pin_i,
                    &autostart_i,
                    &sep2,
                    &settings_i,
                    &quit_i,
                ],
            )?;
            app.manage(TrayMenuState {
                header: tray_header,
                status: tray_status,
                autostart: autostart_i,
                always_on_top: pin_i,
                compact_mode: compact_i,
                primary_submenu,
            });
            {
                let state = app.state::<AppState>();
                let catalog = lock_or_recover(&state.catalog).clone();
                tray::rebuild_primary_submenu(app.handle(), &catalog, &primary_id);
            }

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray::render(None))
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
                        let mut cfg = lock_or_recover(&state.config);
                        let mut candidate = cfg.clone();
                        match config::run_detect(&mut candidate) {
                            Ok(_) => {
                                *cfg = candidate;
                                let snapshot = cfg.clone();
                                drop(cfg);
                                commands::refresh_catalog_and_tray(app, &state, &snapshot);
                                do_refresh(app, None);
                            }
                            Err(error) => eprintln!("provider detection failed: {error}"),
                        }
                    }
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
            commands::get_dashboard,
            commands::refresh_now,
            commands::refresh_provider,
            commands::detect_providers,
            commands::get_app_config,
            commands::set_app_config,
            commands::set_provider_enabled,
            commands::save_api_key,
            commands::quit,
            commands::hide_panel,
            commands::set_notifications,
            commands::set_autostart_enabled,
            commands::set_always_on_top,
            commands::set_compact_mode,
            commands::open_logs_folder,
            commands::clear_logs,
            commands::export_diagnostics,
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
