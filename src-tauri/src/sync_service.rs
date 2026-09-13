//! Ciclo de vida del servidor sync dentro de la app Tauri.
//!
//! El servidor corre en un hilo dedicado mientras sync está activo y se
//! reinicia cuando cambia la configuración (on/off, passphrase, LAN).
//! Nunca expone nada sin cifrar: sirve el blob vía `sync_server`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use tauri::{AppHandle, Manager};

use crate::model::now_iso;
use crate::state::{lock_or_recover, AppState};

/// Estado del hilo del servidor. Vive en `AppState`.
pub(crate) struct SyncServerState {
    pub(crate) running: bool,
    pub(crate) addr: String,
    pub(crate) lan: bool,
    /// Motivo de la última falla (bind ocupado, passphrase ilegible, el
    /// hilo murió solo). `None` cuando `running` es `true` y consistente.
    pub(crate) last_error: Option<String>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Default for SyncServerState {
    fn default() -> Self {
        Self {
            running: false,
            addr: String::new(),
            lan: false,
            last_error: None,
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
}

fn stop_locked(state: &mut SyncServerState) {
    state.stop.store(true, Ordering::Relaxed);
    if let Some(handle) = state.handle.take() {
        let _ = handle.join();
    }
    state.stop = Arc::new(AtomicBool::new(false));
    state.running = false;
    state.addr.clear();
}

/// Arranca, reinicia o detiene el servidor según la config actual.
/// Idempotente: si ya corre con los mismos ajustes no hace nada.
///
/// `Err` = se pidió sync activo pero el servidor no quedó escuchando de
/// verdad (bind ocupado, passphrase ilegible, sin device id). El llamador
/// decide si eso debe revertir un toggle de la UI; este fn nunca deja
/// `running = true` sin un socket real detrás.
pub(crate) fn ensure_sync_server(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = lock_or_recover(&state.config).clone();
    let mut server = lock_or_recover(&state.sync_server);

    if !cfg.sync_enabled || cfg.load_recovered {
        if server.running {
            stop_locked(&mut server);
        }
        server.last_error = None;
        return Ok(());
    }
    let passphrase = match crate::sync::load_passphrase() {
        Ok(passphrase) => passphrase,
        Err(error) => {
            if server.running {
                stop_locked(&mut server);
            }
            server.last_error = Some(error.clone());
            return Err(error);
        }
    };
    let device_id = match crate::sync::load_or_create_device_id() {
        Ok(id) => id,
        Err(error) => {
            if server.running {
                stop_locked(&mut server);
            }
            server.last_error = Some(error.clone());
            return Err(error);
        }
    };

    let serve_cfg = if cfg.sync_lan {
        crate::sync_server::ServeConfig {
            bind: "0.0.0.0".into(),
            port: crate::sync_server::SYNC_DEFAULT_PORT,
            lan: true,
        }
    } else {
        crate::sync_server::ServeConfig::loopback(crate::sync_server::SYNC_DEFAULT_PORT)
    };
    if server.running && server.addr == serve_cfg.addr() && server.lan == cfg.sync_lan {
        return Ok(());
    }
    if server.running {
        stop_locked(&mut server);
    }

    // Bind síncrono ANTES de tocar `server.running`: si el puerto está
    // ocupado (u otra falla de bind), el estado nunca dice "activo" con
    // nada escuchando detrás.
    let http_server = match crate::sync_server::bind(&serve_cfg) {
        Ok(http_server) => http_server,
        Err(error) => {
            server.last_error = Some(error.clone());
            return Err(error);
        }
    };

    // Foto de versión al arrancar; el snapshot se construye por petición.
    let app_version = app.package_info().version.to_string();
    let supplier: crate::sync_server::PayloadFn = {
        let handle = app.clone();
        let device = device_id.clone();
        let version = app_version.clone();
        Arc::new(move || {
            let state = handle.state::<AppState>();
            let snaps = lock_or_recover(&state.snapshots).clone();
            let catalog = lock_or_recover(&state.catalog).clone();
            let snapshot =
                crate::snapshot_v1::build(&snaps, &catalog, now_iso(), Some(version.clone()));
            Ok(crate::sync::build_payload(
                device.clone(),
                now_iso(),
                snapshot,
            ))
        })
    };
    let stop = server.stop.clone();
    let addr = serve_cfg.addr();
    let lan = serve_cfg.lan;
    let this_stop = stop.clone();
    let app_for_thread = app.clone();
    let handle = std::thread::spawn(move || {
        let result = crate::sync_server::serve(
            &http_server,
            &app_version,
            &device_id,
            serve_cfg.lan,
            &supplier,
            &passphrase,
            &stop,
            None,
        );
        // El hilo terminó: si nadie llamó a stop_locked (que ya deja el
        // estado consistente) esto fue una muerte inesperada del socket.
        // No puede quedar "running = true" mintiendo indefinidamente.
        if let Err(error) = result {
            let state = app_for_thread.state::<AppState>();
            let mut server = lock_or_recover(&state.sync_server);
            if Arc::ptr_eq(&server.stop, &this_stop) {
                server.running = false;
                server.addr.clear();
                server.last_error = Some(error.clone());
            }
            eprintln!("sync server stopped with error: {error}");
        }
    });
    server.handle = Some(handle);
    server.running = true;
    server.addr = addr.clone();
    server.lan = lan;
    server.last_error = None;
    eprintln!("sync server listening on {addr}");
    Ok(())
}
