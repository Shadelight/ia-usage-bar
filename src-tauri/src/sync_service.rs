//! Ciclo de vida del servidor sync dentro de la app Tauri.
//!
//! El servidor corre en un hilo dedicado mientras sync está activo y se
//! reinicia cuando cambia la configuración (on/off, passphrase, LAN).
//! Nunca expone nada sin cifrar: sirve el blob vía `sync_server`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use tauri::{AppHandle, Manager};

use crate::config::AppConfig;
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

    // Si V1 está encendido (`sync_enabled`), una passphrase real DEBE existir
    // — su ausencia es un fallo de keyring real que hay que frenar y mostrar
    // (igual que antes de la Tarea 6): el servidor no arranca y el estado
    // queda "Detenido" con el motivo. Si el servidor solo está vivo por
    // razones V2 (`sync_enabled` en false, pero hay un pareo pendiente o un
    // dispositivo V2 activo), las rutas V1 son inalcanzables/no usadas por
    // esa instalación, así que una passphrase ausente es esperable: cae a
    // cadena vacía sin abortar el arranque (`/v1/snapshot` seguiría
    // rechazando esa cadena vacía al descifrar si alguien la alcanzara).
    let passphrase = if cfg.sync_enabled {
        match crate::sync::load_passphrase() {
            Ok(value) => value,
            Err(error) => {
                if server.running {
                    stop_locked(&mut server);
                }
                server.last_error = Some(error.clone());
                return Err(error);
            }
        }
    } else {
        crate::sync::load_passphrase().unwrap_or_default()
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

/// Pure decision logic behind `/v2/pair`'s token consumption. Operates only
/// on in-memory `pending`/`cfg` — the keyring write and the config save are
/// injected as closures so this is testable with plain in-memory fakes, no
/// real `AppHandle`/keyring/config-file I/O.
///
/// Callers (see `build_pairing_hooks` below) must hold `state.config`'s
/// (and `state.pending_pairing`'s) `MutexGuard` for the WHOLE call — the
/// clone/mutate/save/commit sequence below happens inside one critical
/// section, never clone-then-reassign-later (Fix 6).
pub(crate) fn try_consume_pairing_token(
    pending: &mut Option<PendingPairingRuntime>,
    cfg: &mut AppConfig,
    token: &str,
    client_device_id: &str,
    name: &str,
    store_secret: impl FnOnce(&str, &[u8; 32]) -> Result<(), String>,
    save_cfg: impl FnOnce(&AppConfig) -> Result<(), String>,
) -> iausage_core::sync_server::PairOutcome {
    use iausage_core::sync_server::PairOutcome;

    let Some(runtime) = pending.as_ref() else {
        return PairOutcome::NoPendingPairing;
    };
    // Expired/wrong token: reject WITHOUT clearing `pending` as a side
    // effect, so a second, correct attempt can still land before the true
    // 120s TTL expires it for real.
    if runtime.core.is_expired() || !runtime.core.matches_token(token) {
        return PairOutcome::Rejected;
    }
    let secret = runtime.core.secret;
    if store_secret(client_device_id, &secret).is_err() {
        return PairOutcome::Rejected;
    }

    let mut updated = cfg.clone();
    updated.upsert_paired_device(client_device_id, name);
    if save_cfg(&updated).is_err() {
        return PairOutcome::Rejected;
    }
    *cfg = updated;
    *pending = None;
    PairOutcome::Paired
}

/// Pure decision behind `/v2/snapshot`'s key lookup: a revoked (or unknown)
/// device is never active, regardless of whether a secret still happens to
/// sit in the keyring.
pub(crate) fn is_active_paired_device(cfg: &AppConfig, client_device_id: &str) -> bool {
    cfg.paired_devices
        .iter()
        .any(|d| d.client_device_id == client_device_id && !d.revoked)
}

/// V2 pairing hooks backed by real `AppState` — this is the only place that
/// turns the closures `sync_server` expects into something touching config
/// and Credential Manager. Thin wrappers around the pure functions above.
fn build_pairing_hooks(app: &AppHandle) -> iausage_core::sync_server::PairingHooks {
    let try_consume = app.clone();
    let snapshot_key_for = app.clone();
    iausage_core::sync_server::PairingHooks {
        try_consume_token: Arc::new(move |token, client_device_id, name| {
            let state = try_consume.state::<AppState>();
            let mut pending = lock_or_recover(&state.pending_pairing);
            let mut cfg = lock_or_recover(&state.config);
            try_consume_pairing_token(
                &mut pending,
                &mut cfg,
                token,
                client_device_id,
                name,
                |id, secret| {
                    crate::config::store_device_secret(
                        &crate::config::OS_CREDENTIAL_STORE,
                        id,
                        secret,
                    )
                },
                |updated| updated.save(),
            )
        }),
        snapshot_key_for: Arc::new(move |client_device_id| {
            let state = snapshot_key_for.state::<AppState>();
            let cfg = lock_or_recover(&state.config);
            if !is_active_paired_device(&cfg, client_device_id) {
                return None;
            }
            drop(cfg);
            lock_or_recover(&state.last_seen).insert(client_device_id.to_string(), now_iso());
            crate::config::read_device_secret(&crate::config::OS_CREDENTIAL_STORE, client_device_id)
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
    // Hold `state.config`'s lock for the whole read-modify-write-save: two
    // threads (this refresh-loop tick and, e.g., a pairing HTTP request)
    // racing a clone-then-reassign-later pattern could otherwise silently
    // clobber one side's update (Fix 6).
    let mut cfg = lock_or_recover(&state.config);
    let mut changed = false;
    for device in cfg.paired_devices.iter_mut() {
        if let Some(seen_at) = pending.get(&device.client_device_id) {
            device.last_seen_at = Some(seen_at.clone());
            changed = true;
        }
    }
    if changed {
        let _ = cfg.save();
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

    fn pending_runtime() -> (PendingPairingRuntime, String) {
        let new_pairing = iausage_core::pairing::start_pairing();
        let runtime = PendingPairingRuntime {
            core: new_pairing.pending,
            expires_at_iso: now_iso(),
        };
        (runtime, new_pairing.token_hex)
    }

    // Spec "Testing" section, behavior 1: an expired token is rejected
    // (410-equivalent `Rejected`) while `PendingPairing` is left INTACT —
    // expiry rejection must not clear it as a side effect, so a second,
    // correct attempt before the true TTL could still succeed.
    #[test]
    fn expired_token_is_rejected_without_clearing_pending() {
        let (mut runtime, token) = pending_runtime();
        runtime.core.expires_at = std::time::Instant::now() - std::time::Duration::from_secs(1);
        let mut pending = Some(runtime);
        let mut cfg = AppConfig::default();

        let outcome = try_consume_pairing_token(
            &mut pending,
            &mut cfg,
            &token,
            "phone-1",
            "Galaxy",
            |_, _| Ok(()),
            |_| Ok(()),
        );

        assert!(matches!(
            outcome,
            iausage_core::sync_server::PairOutcome::Rejected
        ));
        assert!(pending.is_some(), "expiry must not clear PendingPairing");
        assert!(cfg.paired_devices.is_empty());
    }

    // Spec behavior 2: a replayed token, after the pending pairing was
    // already cleared by a prior success, is rejected as `NoPendingPairing`
    // (not `Rejected`) — there is nothing left to compare against.
    #[test]
    fn replayed_token_after_pending_cleared_is_no_pending_pairing() {
        let (_, token) = pending_runtime();
        let mut pending: Option<PendingPairingRuntime> = None;
        let mut cfg = AppConfig::default();

        let outcome = try_consume_pairing_token(
            &mut pending,
            &mut cfg,
            &token,
            "phone-1",
            "Galaxy",
            |_, _| Ok(()),
            |_| Ok(()),
        );

        assert!(matches!(
            outcome,
            iausage_core::sync_server::PairOutcome::NoPendingPairing
        ));
    }

    // Spec behavior 3: a successful pair registers a `PairedDevice`, the
    // secret becomes readable back (stood in here by a fake keyring), and
    // `PendingPairing` is cleared.
    #[test]
    fn successful_pair_registers_device_stores_secret_and_clears_pending() {
        let (runtime, token) = pending_runtime();
        let expected_secret = runtime.core.secret;
        let mut pending = Some(runtime);
        let mut cfg = AppConfig::default();
        let mut stored: Option<(String, [u8; 32])> = None;

        let outcome = try_consume_pairing_token(
            &mut pending,
            &mut cfg,
            &token,
            "phone-1",
            "Galaxy S26",
            |id, secret| {
                stored = Some((id.to_string(), *secret));
                Ok(())
            },
            |_| Ok(()),
        );

        assert!(matches!(
            outcome,
            iausage_core::sync_server::PairOutcome::Paired
        ));
        assert!(pending.is_none(), "success must clear PendingPairing");
        assert_eq!(cfg.paired_devices.len(), 1);
        assert_eq!(cfg.paired_devices[0].client_device_id, "phone-1");
        assert!(!cfg.paired_devices[0].revoked);
        let (stored_id, stored_secret) = stored.expect("store_secret must be called on success");
        assert_eq!(stored_id, "phone-1");
        assert_eq!(stored_secret, expected_secret, "secret must round-trip");
    }

    // If `save_cfg` fails, the whole attempt must roll back rather than
    // leave `pending` cleared with an unsaved config change floating.
    #[test]
    fn pair_is_rejected_when_config_save_fails() {
        let (runtime, token) = pending_runtime();
        let mut pending = Some(runtime);
        let mut cfg = AppConfig::default();

        let outcome = try_consume_pairing_token(
            &mut pending,
            &mut cfg,
            &token,
            "phone-1",
            "Galaxy",
            |_, _| Ok(()),
            |_| Err("disk full".to_string()),
        );

        assert!(matches!(
            outcome,
            iausage_core::sync_server::PairOutcome::Rejected
        ));
        assert!(pending.is_some(), "a failed save must not clear pending");
        assert!(
            cfg.paired_devices.is_empty(),
            "a failed save must not commit the device row"
        );
    }

    // Spec behavior 4: `/v2/snapshot`'s key lookup returns `None` (not the
    // secret) for a revoked device.
    #[test]
    fn revoked_device_is_never_active() {
        let mut cfg = AppConfig::default();
        cfg.upsert_paired_device("phone-1", "Galaxy");
        assert!(is_active_paired_device(&cfg, "phone-1"));

        cfg.revoke_device("phone-1");
        assert!(!is_active_paired_device(&cfg, "phone-1"));
        assert!(!is_active_paired_device(&cfg, "unknown-device"));
    }
}
