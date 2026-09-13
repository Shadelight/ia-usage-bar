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

/// Pure decision the rest of `ensure_sync_server` acts on. Kept as its own
/// function so the truth table can be tested without a Tauri AppHandle.
pub(crate) fn server_needed(
    sync_enabled: bool,
    has_pending_pairing: bool,
    has_active_paired_device: bool,
) -> bool {
    sync_enabled || has_pending_pairing || has_active_paired_device
}

pub(crate) struct PendingPairingRuntime {
    pub(crate) core: iausage_core::pairing::PendingPairing,
    pub(crate) expires_at_iso: String,
}

/// Arranca, reinicia o detiene el servidor según la config actual y el
/// pareo V2 en curso. Idempotente: si ya corre con los mismos ajustes no
/// hace nada.
///
/// `Err` = se necesitaba el servidor pero no quedó escuchando de verdad
/// (bind ocupado, passphrase V1 ilegible, sin device id). El llamador
/// decide si eso debe revertir un toggle de la UI; este fn nunca deja
/// `running = true` sin un socket real detrás.
pub(crate) fn ensure_sync_server(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = lock_or_recover(&state.config).clone();
    let mut server = lock_or_recover(&state.sync_server);

    // Descarta un pairing pendiente que ya venció ANTES de decidir si el
    // servidor sigue haciendo falta — el tick del refresh loop es lo que
    // hace que esto se vuelva a evaluar sin intervención del usuario.
    {
        let mut pending = lock_or_recover(&state.pending_pairing);
        if pending.as_ref().is_some_and(|p| p.core.is_expired()) {
            *pending = None;
        }
    }
    let has_pending_pairing = lock_or_recover(&state.pending_pairing).is_some();
    let needed = server_needed(
        cfg.sync_enabled,
        has_pending_pairing,
        cfg.has_active_paired_device(),
    );

    if !needed || cfg.load_recovered {
        if server.running {
            stop_locked(&mut server);
        }
        server.last_error = None;
        return Ok(());
    }

    // V1 passphrase se sigue cargando solo si V1 realmente lo necesita — un
    // pareo V2 puro nunca debería fallar por falta de passphrase. Una
    // passphrase vacía aquí es segura: la ruta `/v1/snapshot` fallará al
    // descifrar para quien la use sin una passphrase real, pero esa ruta
    // solo es alcanzable por un teléfono ya emparejado en V1, que por
    // definición ya tiene una passphrase real guardada; una instalación
    // solo-V2 (sin `sync_enabled`) nunca dispara este camino porque
    // `server_needed` no habría arrancado el servidor por razones V1 si no
    // hubiera una passphrase V1 real que hubiera puesto `sync_enabled =
    // true`.
    let passphrase = crate::sync::load_passphrase().unwrap_or_default();
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
    let pairing_hooks = build_pairing_hooks(app);
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
            Some(&pairing_hooks),
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

/// V2 pairing hooks backed by real `AppState` — this is the only place that
/// turns the closures `sync_server` expects into something touching config
/// and Credential Manager.
fn build_pairing_hooks(app: &AppHandle) -> iausage_core::sync_server::PairingHooks {
    use iausage_core::sync_server::PairOutcome;

    let try_consume = app.clone();
    let snapshot_key_for = app.clone();
    iausage_core::sync_server::PairingHooks {
        try_consume_token: Arc::new(move |token, client_device_id, name| {
            let state = try_consume.state::<AppState>();
            let mut pending = lock_or_recover(&state.pending_pairing);
            let Some(runtime) = pending.as_ref() else {
                return PairOutcome::NoPendingPairing;
            };
            if runtime.core.is_expired() || !runtime.core.matches_token(token) {
                return PairOutcome::Rejected;
            }
            let secret = runtime.core.secret;
            if crate::config::store_device_secret(client_device_id, &secret).is_err() {
                return PairOutcome::Rejected;
            }
            let mut cfg = lock_or_recover(&state.config).clone();
            cfg.upsert_paired_device(client_device_id, name);
            if cfg.save().is_err() {
                return PairOutcome::Rejected;
            }
            *lock_or_recover(&state.config) = cfg;
            *pending = None;
            PairOutcome::Paired
        }),
        snapshot_key_for: Arc::new(move |client_device_id| {
            let state = snapshot_key_for.state::<AppState>();
            let cfg = lock_or_recover(&state.config);
            let active = cfg
                .paired_devices
                .iter()
                .any(|d| d.client_device_id == client_device_id && !d.revoked);
            if !active {
                return None;
            }
            lock_or_recover(&state.last_seen).insert(client_device_id.to_string(), now_iso());
            crate::config::read_device_secret(client_device_id)
        }),
    }
}

/// Vuelca `AppState.last_seen` a `config.toml` (piggybacked on the refresh
/// loop's own tick — see the spec's "last_seen_at is not written on every
/// /v2/snapshot call" note). No-op, no save, when nothing changed since the
/// last flush.
pub(crate) fn flush_last_seen(app: &AppHandle) {
    let state = app.state::<AppState>();
    let pending: std::collections::HashMap<String, String> =
        lock_or_recover(&state.last_seen).drain().collect();
    if pending.is_empty() {
        return;
    }
    let mut cfg = lock_or_recover(&state.config).clone();
    let mut changed = false;
    for device in cfg.paired_devices.iter_mut() {
        if let Some(seen_at) = pending.get(&device.client_device_id) {
            device.last_seen_at = Some(seen_at.clone());
            changed = true;
        }
    }
    if changed && cfg.save().is_ok() {
        *lock_or_recover(&state.config) = cfg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_needed_truth_table() {
        assert!(!server_needed(false, false, false));
        assert!(
            server_needed(true, false, false),
            "legacy V1 toggle alone must keep the server up"
        );
        assert!(
            server_needed(false, true, false),
            "a pending pairing alone must start the server"
        );
        assert!(
            server_needed(false, false, true),
            "an active V2 device alone must keep the server up"
        );
        assert!(server_needed(true, true, true));
    }
}
