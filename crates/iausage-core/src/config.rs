//! Configuración persistente (`%APPDATA%\ia-usagebar\config.toml`).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::descriptor::FetchStrategyKind;
use crate::model::{CredentialSource, VendorId};
use crate::paths::{app_config_dir, config_path, detect_path};

const CREDENTIAL_SERVICE: &str = "com.alberth.iausagebar";
pub(crate) const SYNC_PASSPHRASE_ACCOUNT: &str = "sync-passphrase";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_refresh")]
    pub refresh_minutes: u64,
    /// `true` = intervalos adaptativos por actividad (2/5/15/30 min);
    /// `false` = `refresh_minutes` fijo.
    #[serde(default = "default_adaptive")]
    pub refresh_adaptive: bool,
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default = "default_true")]
    pub notifications: bool,
    #[serde(default = "default_notify_thresholds")]
    pub notify_thresholds: Vec<u8>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// Default `true`: the window has always opened always-on-top, so a
    /// `config.toml` written before this field existed must keep behaving
    /// that way once it gains the field.
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default)]
    pub compact_mode: bool,
    /// M5 sync con el teléfono: escribe el blob cifrado tras cada refresh.
    #[serde(default)]
    pub sync_enabled: bool,
    /// Carpeta del blob `ia-sync/<device>.json`. Vacía = carpeta por defecto
    /// dentro del config dir. La passphrase vive en keyring, nunca aquí.
    #[serde(default)]
    pub sync_export_dir: Option<String>,
    /// Exponer el sync HTTP en LAN (0.0.0.0). Solo con opt-in explícito.
    #[serde(default)]
    pub sync_lan: bool,
    /// Teléfonos vinculados por Sync V2. Los secretos NUNCA viven aquí — solo
    /// en Credential Manager, cuenta `device-secret-<client_device_id>`.
    #[serde(default)]
    pub paired_devices: Vec<PairedDevice>,
    /// True when `load()` could not parse `config.toml` and fell back to
    /// defaults after backing the bad file aside. Never persisted; callers
    /// must not overwrite `config.toml` while this is set.
    #[serde(skip)]
    pub load_recovered: bool,
}

fn default_refresh() -> u64 {
    5
}
fn default_adaptive() -> bool {
    true
}
fn default_primary() -> String {
    "anthropic".into()
}
fn default_true() -> bool {
    true
}
fn default_notify_thresholds() -> Vec<u8> {
    vec![75, 90, 95]
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Fuente preferida (`None` = Automática). Debe pertenecer a
    /// `descriptor(id).strategies`; si no, se ignora y se usa el default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FetchStrategyKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedDevice {
    pub client_device_id: String,
    pub name: String,
    pub created_at: String,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub revoked: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_minutes: 5,
            refresh_adaptive: true,
            primary: "anthropic".into(),
            notifications: true,
            notify_thresholds: default_notify_thresholds(),
            providers: HashMap::new(),
            always_on_top: true,
            compact_mode: false,
            sync_enabled: false,
            sync_export_dir: None,
            sync_lan: false,
            paired_devices: Vec::new(),
            load_recovered: false,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    fn load_from(path: &Path) -> Self {
        let mut cfg: AppConfig = match Self::try_load(path) {
            Ok(Some(cfg)) => cfg,
            Ok(None) => AppConfig::default(),
            Err(_) => {
                // A config.toml that exists but will not parse is NOT an
                // invitation to overwrite it with defaults (that would destroy
                // every stored API key). Back it up and flag the recovery so
                // no caller re-saves over the backup.
                back_up_corrupt(path);
                AppConfig {
                    load_recovered: true,
                    ..AppConfig::default()
                }
            }
        };
        cfg.normalize();
        cfg
    }

    pub fn normalize(&mut self) {
        self.refresh_minutes = match self.refresh_minutes {
            1 | 2 | 5 | 15 | 30 => self.refresh_minutes,
            n if n < 2 => 1,
            n if n < 4 => 2,
            n if n < 10 => 5,
            n if n < 22 => 15,
            _ => 30,
        };
        self.notify_thresholds.retain(|n| (1..=99).contains(n));
        self.notify_thresholds.sort_unstable();
        self.notify_thresholds.dedup();
        if self.notify_thresholds.is_empty() {
            self.notify_thresholds = default_notify_thresholds();
        }
        // Old builds could persist a strategy that a provider never actually
        // implemented. Keep the provider enabled but fall back to Auto so the
        // saved config remains executable after restart.
        for id in VendorId::all().iter().copied() {
            let Some(provider) = self.providers.get_mut(id.slug()) else {
                continue;
            };
            if let Some(source) = provider.source {
                if !crate::descriptor::descriptor(id)
                    .strategies
                    .contains(&source)
                {
                    provider.source = None;
                }
            }
        }
    }

    /// `Ok(None)` = file absent (a normal first run). `Ok(Some)` = parsed.
    /// `Err` = the file exists but is unreadable or unparseable.
    fn try_load(path: &Path) -> Result<Option<Self>, String> {
        let raw = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        toml::from_str(&raw).map(Some).map_err(|e| e.to_string())
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = app_config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let body = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        atomic_write(&config_path(), body.as_bytes())
    }

    pub fn provider(&self, id: VendorId) -> ProviderConfig {
        self.providers.get(id.slug()).cloned().unwrap_or_default()
    }

    pub fn is_enabled(&self, id: VendorId) -> bool {
        self.provider(id).enabled
    }

    pub fn set_enabled(&mut self, id: VendorId, enabled: bool) {
        self.providers
            .entry(id.slug().to_string())
            .or_default()
            .enabled = enabled;
    }

    pub fn source_preference(&self, id: VendorId) -> Option<FetchStrategyKind> {
        self.providers.get(id.slug()).and_then(|p| p.source)
    }

    pub fn set_source_preference(&mut self, id: VendorId, source: Option<FetchStrategyKind>) {
        self.providers
            .entry(id.slug().to_string())
            .or_default()
            .source = source;
    }

    pub fn api_key(&self, id: VendorId) -> Option<String> {
        if let Some(env) = id.env_key() {
            if let Ok(v) = std::env::var(env) {
                if !v.trim().is_empty() {
                    return Some(v);
                }
            }
        }
        if let Some(value) = keyring_api_key(id) {
            return Some(value);
        }
        self.provider(id).api_key.filter(|s| !s.trim().is_empty())
    }

    /// Credential precedence mirrors `api_key()` and reports only its origin.
    pub fn credential_source(&self, id: VendorId) -> Option<CredentialSource> {
        if let Some(env) = id.env_key() {
            if std::env::var(env).is_ok_and(|value| !value.trim().is_empty()) {
                return Some(CredentialSource::Environment);
            }
        }
        if keyring_api_key(id).is_some() {
            return Some(CredentialSource::Keyring);
        }
        self.provider(id)
            .api_key
            .filter(|value| !value.trim().is_empty())
            .map(|_| CredentialSource::Legacy)
    }

    pub fn enabled_ids(&self) -> Vec<VendorId> {
        VendorId::all()
            .iter()
            .copied()
            .filter(|id| self.is_enabled(*id))
            .collect()
    }

    pub fn upsert_paired_device(&mut self, client_device_id: &str, name: &str) -> &PairedDevice {
        if let Some(existing) = self
            .paired_devices
            .iter_mut()
            .find(|d| d.client_device_id == client_device_id)
        {
            existing.name = name.to_string();
            existing.revoked = false;
            existing.last_seen_at = None;
        } else {
            self.paired_devices.push(PairedDevice {
                client_device_id: client_device_id.to_string(),
                name: name.to_string(),
                created_at: crate::model::now_iso(),
                last_seen_at: None,
                revoked: false,
            });
        }
        self.paired_devices
            .iter()
            .find(|d| d.client_device_id == client_device_id)
            .expect("just inserted or updated above")
    }

    pub fn revoke_device(&mut self, client_device_id: &str) -> bool {
        let Some(device) = self
            .paired_devices
            .iter_mut()
            .find(|d| d.client_device_id == client_device_id)
        else {
            return false;
        };
        device.revoked = true;
        true
    }

    pub fn has_active_paired_device(&self) -> bool {
        self.paired_devices.iter().any(|d| !d.revoked)
    }
}

/// Windows can pad a Credential Manager blob to an even byte count with a
/// trailing NUL, which fails get_password()'s strict UTF-8 decode even
/// though the credential was written correctly. keyring-rs's own docs point
/// at get_secret() as the fallback for exactly this case.
pub(crate) fn read_keyring_entry(entry: &keyring::Entry) -> Option<String> {
    if let Ok(value) = entry.get_password() {
        return Some(value);
    }
    let secret = entry.get_secret().ok()?;
    Some(decode_keyring_secret(&secret))
}

fn decode_keyring_secret(secret: &[u8]) -> String {
    String::from_utf8_lossy(secret)
        .trim_end_matches('\0')
        .to_string()
}

/// Reads a credential from the OS keyring only. It never falls back to an
/// environment variable or legacy config, so callers can verify persistence.
pub fn keyring_api_key(id: VendorId) -> Option<String> {
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, id.slug()).ok()?;
    read_keyring_entry(&entry).filter(|value| !value.trim().is_empty())
}

/// Proves that the value just written can be read from the OS credential
/// store. This intentionally bypasses `AppConfig::api_key()`: an old
/// environment variable must not hide a failed keyring round-trip.
pub fn verify_keyring_api_key(id: VendorId, expected: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, id.slug()).map_err(|e| e.to_string())?;
    let (stored, password_err, secret_err) = match entry.get_password() {
        Ok(value) => (Some(value), None, None),
        Err(password_err) => match entry.get_secret() {
            Ok(secret) => (
                Some(decode_keyring_secret(&secret)),
                Some(password_err),
                None,
            ),
            Err(secret_err) => (None, Some(password_err), Some(secret_err)),
        },
    };
    let stored = stored.filter(|v| !v.trim().is_empty()).ok_or_else(|| {
        format!(
            "La credencial se escribió pero no pudo recuperarse del almacén seguro \
             (get_password: {}, get_secret: {})",
            password_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "ok".to_string()),
            secret_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "n/a".to_string())
        )
    })?;
    if stored.trim().is_empty() || stored.trim() != expected.trim() {
        return Err("La credencial guardada no coincide con la que se escribió".into());
    }
    Ok(())
}

pub fn store_api_key(id: VendorId, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, id.slug()).map_err(|e| e.to_string())?;
    if value.trim().is_empty() {
        // Misma guarda NUL que en sync: get_password() puede fallar aunque
        // la credencial exista, así que la existencia se comprueba con el
        // lector tolerante antes de borrar.
        if read_keyring_entry(&entry).is_some() {
            entry.delete_credential().map_err(|e| e.to_string())?;
        }
    } else {
        entry
            .set_password(value.trim())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Move credentials written by pre-0.1 builds out of config.toml. A value is
/// removed from the config only after Windows Credential Manager accepts it.
pub fn migrate_legacy_credentials(cfg: &mut AppConfig) -> bool {
    let mut changed = false;
    for id in VendorId::all().iter().copied() {
        let legacy = cfg
            .providers
            .get(id.slug())
            .and_then(|provider| provider.api_key.clone());
        if let Some(value) = legacy {
            if store_api_key(id, &value).is_ok() {
                if let Some(provider) = cfg.providers.get_mut(id.slug()) {
                    provider.api_key = None;
                    changed = true;
                }
            }
        }
    }
    changed
}

fn device_secret_account(client_device_id: &str) -> String {
    format!("device-secret-{client_device_id}")
}

/// Secure byte storage. The pairing/revoke logic takes it as a parameter so
/// it can be tested against an in-memory store instead of the OS keyring.
pub trait CredentialStore {
    fn write(&self, account: &str, value: &[u8]) -> Result<(), String>;
    /// `Ok(None)` when the account has no credential.
    fn read(&self, account: &str) -> Result<Option<Vec<u8>>, String>;
    /// Idempotent: deleting an absent credential is not an error.
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// OS credential store (Windows Credential Manager, macOS Keychain, Secret
/// Service) through `keyring`.
pub struct KeyringStore {
    pub service: &'static str,
}

pub const OS_CREDENTIAL_STORE: KeyringStore = KeyringStore {
    service: CREDENTIAL_SERVICE,
};

/// Errors are reduced to their kind: a keyring error can carry credential
/// bytes (`BadEncoding`), which must never reach a log or a UI message.
fn keyring_error_kind(e: &keyring::Error) -> String {
    match e {
        keyring::Error::PlatformFailure(err) => format!("PlatformFailure: {err}"),
        keyring::Error::NoStorageAccess(err) => format!("NoStorageAccess: {err}"),
        keyring::Error::NoEntry => "NoEntry".into(),
        keyring::Error::BadEncoding(_) => "BadEncoding".into(),
        keyring::Error::TooLong(attr, max) => format!("TooLong: {attr} > {max}"),
        keyring::Error::Invalid(attr, reason) => format!("Invalid: {attr}: {reason}"),
        keyring::Error::Ambiguous(_) => "Ambiguous".into(),
        _ => "Unknown".into(),
    }
}

impl KeyringStore {
    fn entry(&self, account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(self.service, account).map_err(|e| keyring_error_kind(&e))
    }
}

/// Raw bytes — deliberately `set_secret`/`get_secret`, not the NUL-tolerant
/// string path `read_keyring_entry` uses for API keys: a random secret is not
/// a NUL-terminated string, so there's no padding quirk to work around.
impl CredentialStore for KeyringStore {
    fn write(&self, account: &str, value: &[u8]) -> Result<(), String> {
        self.entry(account)?
            .set_secret(value)
            .map_err(|e| keyring_error_kind(&e))
    }

    fn read(&self, account: &str) -> Result<Option<Vec<u8>>, String> {
        match self.entry(account)?.get_secret() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_error_kind(&e)),
        }
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keyring_error_kind(&e)),
        }
    }
}

pub fn store_device_secret(
    store: &dyn CredentialStore,
    client_device_id: &str,
    secret: &[u8; 32],
) -> Result<(), String> {
    store.write(&device_secret_account(client_device_id), secret)
}

pub fn read_device_secret(store: &dyn CredentialStore, client_device_id: &str) -> Option<[u8; 32]> {
    store
        .read(&device_secret_account(client_device_id))
        .ok()
        .flatten()?
        .try_into()
        .ok()
}

/// Idempotent: revoking a device that was already deleted (or never had a
/// secret) is not an error.
pub fn delete_device_secret(
    store: &dyn CredentialStore,
    client_device_id: &str,
) -> Result<(), String> {
    store.delete(&device_secret_account(client_device_id))
}

/// Revokes a paired device: marks its row revoked, persists it with `save`,
/// then deletes its secret. The row is saved first so that even if the
/// delete fails, the device can no longer fetch snapshots.
pub fn revoke_paired_device(
    cfg: &mut AppConfig,
    store: &dyn CredentialStore,
    client_device_id: &str,
    save: impl FnOnce(&AppConfig) -> Result<(), String>,
) -> Result<(), String> {
    if !cfg.revoke_device(client_device_id) {
        return Err("sync: dispositivo no encontrado".into());
    }
    save(cfg)?;
    delete_device_secret(store, client_device_id)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DetectState {
    #[serde(default)]
    known: HashSet<String>,
}

pub fn detected_ids() -> HashSet<String> {
    fs::read_to_string(detect_path())
        .ok()
        .and_then(|body| serde_json::from_str::<DetectState>(&body).ok())
        .map(|state| state.known)
        .unwrap_or_default()
}

pub fn run_detect(cfg: &mut AppConfig) -> Result<Vec<String>, String> {
    let path = detect_path();
    let mut state: DetectState = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut newly = Vec::new();
    for id in VendorId::all() {
        let slug = id.slug().to_string();
        let detected = crate::providers::has_local_credentials(*id, cfg);
        if detected && !state.known.contains(&slug) {
            state.known.insert(slug.clone());
            cfg.set_enabled(*id, true);
            newly.push(slug);
        } else if !detected {
            // A later manual detection must be able to discover a login that
            // did not exist during the first run.
            state.known.remove(&slug);
        }
    }
    if !cfg.load_recovered {
        cfg.save()?;
    }
    save_detect(&path, &state)?;
    Ok(newly)
}

/// Rename an unparseable `config.toml` to `config.toml.bak-<unix-ts>` so its
/// contents (API keys included) stay recoverable instead of being overwritten.
fn back_up_corrupt(path: &Path) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bak = path.with_extension(format!("toml.bak-{ts}"));
    let _ = fs::rename(path, bak);
}

fn save_detect(path: &Path, state: &DetectState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    atomic_write(
        path,
        &serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?,
    )
}

pub fn atomic_write(path: &Path, body: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, body).map_err(|e| e.to_string())?;
    match fs::rename(&tmp, path) {
        Ok(()) => return Ok(()),
        Err(error) if !path.exists() => return Err(error.to_string()),
        Err(_) => {}
    }
    // Windows does not replace an existing destination with rename(). Keep a
    // recoverable backup until the replacement has landed.
    let backup = path.with_extension(format!("replace-bak-{}", std::process::id()));
    let _ = fs::remove_file(&backup);
    fs::rename(path, &backup).map_err(|e| e.to_string())?;
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::rename(&backup, path);
        return Err(error.to_string());
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cfg-test-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // F-H2: an unparseable config.toml must not be silently replaced by
    // defaults — try_load reports the error and load_from backs the file up
    // (keys recoverable) and flags the recovery so nothing re-saves over it.
    #[test]
    fn corrupt_config_is_backed_up_not_destroyed() {
        let dir = scratch_dir("corrupt");
        let path = dir.join("config.toml");
        fs::write(&path, "not = valid = toml = {{{").unwrap();

        assert!(AppConfig::try_load(&path).is_err());

        let cfg = AppConfig::load_from(&path);
        assert!(cfg.load_recovered);
        assert!(!path.exists(), "bad file should have been renamed aside");

        let baks: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("toml.bak-"))
            .collect();
        assert_eq!(baks.len(), 1, "exactly one backup");
        let recovered = fs::read_to_string(baks[0].path()).unwrap();
        assert!(recovered.contains("not = valid = toml"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn absent_config_is_a_normal_default_load() {
        let dir = scratch_dir("absent");
        let path = dir.join("config.toml");
        assert!(matches!(AppConfig::try_load(&path), Ok(None)));
        let cfg = AppConfig::load_from(&path);
        assert!(!cfg.load_recovered);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn defaults_match_current_window_behavior() {
        let cfg = AppConfig::default();
        assert!(cfg.always_on_top, "window has always opened always-on-top");
        assert!(!cfg.compact_mode);
    }

    // Regresión: sin el feature `windows-native` (y equivalentes en otros
    // SO), la crate `keyring` cae en su backend `mock`, que NO persiste
    // entre instancias de `Entry` — cada `Entry::new()` es un almacén vacío
    // propio. `store_api_key()` y `verify_keyring_api_key()` crean cada uno
    // su propio `Entry`, así que bajo `mock` la escritura "funciona" pero la
    // lectura inmediata siempre falla con "no pudo recuperarse". Este test
    // falla bajo `mock` y pasa contra el almacén real del SO.
    #[test]
    fn keyring_persists_across_separate_entry_instances() {
        let account = format!("iausage-keyring-smoke-{}", std::process::id());
        let service = "com.alberth.iausagebar-test";
        let secret = "smoke-test-secret-value";

        let write_entry = keyring::Entry::new(service, &account).unwrap();
        write_entry.set_password(secret).unwrap();
        drop(write_entry);

        let read_entry = keyring::Entry::new(service, &account).unwrap();
        let stored = read_keyring_entry(&read_entry);
        read_entry.delete_credential().ok();

        assert_eq!(
            stored.as_deref(),
            Some(secret),
            "la credencial no sobrevivió entre instancias de Entry: \
             ¿falta el feature nativo del backend de keyring para este SO?"
        );
    }

    #[test]
    fn keyring_secret_decoder_tolerates_windows_nul_padding() {
        assert_eq!(
            decode_keyring_secret(b"cuatro-palabras-123\0"),
            "cuatro-palabras-123"
        );
        assert_eq!(decode_keyring_secret(b"sk-abc123\0\0"), "sk-abc123");
        assert_eq!(decode_keyring_secret(b"plain"), "plain");
    }

    #[test]
    fn normalize_clamps_refresh_and_notification_thresholds() {
        let mut cfg = AppConfig {
            refresh_minutes: 7,
            notify_thresholds: vec![95, 0, 75, 75, 101],
            ..AppConfig::default()
        };
        cfg.normalize();
        assert_eq!(cfg.refresh_minutes, 5);
        assert_eq!(cfg.notify_thresholds, vec![75, 95]);
    }

    #[test]
    fn normalize_removes_a_source_that_is_not_implemented() {
        let mut cfg = AppConfig::default();
        cfg.set_source_preference(VendorId::Anthropic, Some(FetchStrategyKind::Api));
        cfg.normalize();
        assert_eq!(cfg.source_preference(VendorId::Anthropic), None);
    }

    #[test]
    fn provider_enabled_and_source_preference_survive_reload() {
        let dir = scratch_dir("provider-persistence");
        let path = dir.join("config.toml");
        let mut cfg = AppConfig::default();
        cfg.set_enabled(VendorId::Anthropic, true);
        cfg.set_source_preference(VendorId::Anthropic, Some(FetchStrategyKind::Oauth));
        atomic_write(&path, toml::to_string_pretty(&cfg).unwrap().as_bytes()).unwrap();

        let reloaded = AppConfig::load_from(&path);
        assert!(reloaded.is_enabled(VendorId::Anthropic));
        assert_eq!(
            reloaded.source_preference(VendorId::Anthropic),
            Some(FetchStrategyKind::Oauth)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_write_replaces_existing_file_without_partial_content() {
        let dir = scratch_dir("atomic");
        let path = dir.join("state.json");
        fs::write(&path, b"old").unwrap();
        atomic_write(&path, b"new complete value").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new complete value");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn upsert_paired_device_inserts_then_updates_in_place() {
        let mut cfg = AppConfig::default();
        cfg.upsert_paired_device("phone-1", "Galaxy S26");
        assert_eq!(cfg.paired_devices.len(), 1);
        let created_at = cfg.paired_devices[0].created_at.clone();
        assert!(!cfg.paired_devices[0].revoked);
        assert!(cfg.paired_devices[0].last_seen_at.is_none());

        cfg.revoke_device("phone-1");
        assert!(cfg.paired_devices[0].revoked);

        // Re-pairing the same phone: upsert, not a second row.
        cfg.upsert_paired_device("phone-1", "Galaxy S26 (renamed)");
        assert_eq!(
            cfg.paired_devices.len(),
            1,
            "re-pairing must not create a second row"
        );
        assert!(!cfg.paired_devices[0].revoked, "re-pairing clears revoked");
        assert_eq!(cfg.paired_devices[0].name, "Galaxy S26 (renamed)");
        assert_eq!(
            cfg.paired_devices[0].created_at, created_at,
            "first-paired date is preserved"
        );
        assert!(
            cfg.paired_devices[0].last_seen_at.is_none(),
            "fresh secret resets last_seen_at"
        );
    }

    #[test]
    fn has_active_paired_device_ignores_revoked_rows() {
        let mut cfg = AppConfig::default();
        assert!(!cfg.has_active_paired_device());
        cfg.upsert_paired_device("phone-1", "Galaxy");
        assert!(cfg.has_active_paired_device());
        cfg.revoke_device("phone-1");
        assert!(!cfg.has_active_paired_device());
    }

    #[test]
    fn revoke_device_reports_whether_a_row_existed() {
        let mut cfg = AppConfig::default();
        assert!(!cfg.revoke_device("no-such-device"));
        cfg.upsert_paired_device("phone-1", "Galaxy");
        assert!(cfg.revoke_device("phone-1"));
    }

    #[test]
    fn paired_devices_survive_a_config_reload() {
        let dir = scratch_dir("paired-devices");
        let path = dir.join("config.toml");
        let mut cfg = AppConfig::default();
        cfg.upsert_paired_device("phone-1", "Galaxy S26");
        atomic_write(&path, toml::to_string_pretty(&cfg).unwrap().as_bytes()).unwrap();

        let reloaded = AppConfig::load_from(&path);
        assert_eq!(reloaded.paired_devices.len(), 1);
        assert_eq!(reloaded.paired_devices[0].client_device_id, "phone-1");
        let _ = fs::remove_dir_all(dir);
    }

    /// Test-only in-memory `CredentialStore`: pairing/revoke logic is checked
    /// deterministically, without the OS credential manager.
    #[derive(Default)]
    struct MemoryCredentialStore(std::cell::RefCell<HashMap<String, Vec<u8>>>);

    impl CredentialStore for MemoryCredentialStore {
        fn write(&self, account: &str, value: &[u8]) -> Result<(), String> {
            self.0
                .borrow_mut()
                .insert(account.to_string(), value.to_vec());
            Ok(())
        }

        fn read(&self, account: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(self.0.borrow().get(account).cloned())
        }

        fn delete(&self, account: &str) -> Result<(), String> {
            self.0.borrow_mut().remove(account);
            Ok(())
        }
    }

    #[test]
    fn device_secret_write_read_delete_round_trip() {
        let store = MemoryCredentialStore::default();
        let secret = [7u8; 32];
        store_device_secret(&store, "phone-1", &secret).unwrap();
        assert_eq!(read_device_secret(&store, "phone-1"), Some(secret));
        delete_device_secret(&store, "phone-1").unwrap();
        assert_eq!(read_device_secret(&store, "phone-1"), None);
        // Deleting an already-absent secret is not an error (revoke is idempotent).
        assert!(delete_device_secret(&store, "phone-1").is_ok());
    }

    #[test]
    fn revoking_a_paired_device_deletes_its_secret() {
        let store = MemoryCredentialStore::default();
        let mut cfg = AppConfig::default();
        cfg.upsert_paired_device("phone-1", "Galaxy");
        cfg.upsert_paired_device("phone-2", "Pixel");
        store_device_secret(&store, "phone-1", &[1u8; 32]).unwrap();
        store_device_secret(&store, "phone-2", &[2u8; 32]).unwrap();

        let mut saved_as_revoked = None;
        revoke_paired_device(&mut cfg, &store, "phone-1", |saved| {
            saved_as_revoked = Some(saved.paired_devices[0].revoked);
            Ok(())
        })
        .unwrap();

        assert_eq!(saved_as_revoked, Some(true), "the revoked row is persisted");
        assert!(cfg.paired_devices[0].revoked);
        assert_eq!(
            read_device_secret(&store, "phone-1"),
            None,
            "a revoked device's secret must not be recoverable"
        );
        assert!(!cfg.paired_devices[1].revoked);
        assert_eq!(
            read_device_secret(&store, "phone-2"),
            Some([2u8; 32]),
            "other devices keep their secret"
        );
    }

    #[test]
    fn revoking_an_unknown_device_is_an_error_and_touches_no_secret() {
        let store = MemoryCredentialStore::default();
        store_device_secret(&store, "phone-1", &[1u8; 32]).unwrap();
        let mut cfg = AppConfig::default();
        let result = revoke_paired_device(&mut cfg, &store, "phone-1", |_| {
            panic!("nothing to save for an unknown device")
        });
        assert!(result.is_err());
        assert_eq!(read_device_secret(&store, "phone-1"), Some([1u8; 32]));
    }

    /// Smoke test against the real Windows Credential Manager, in an isolated
    /// test namespace. Ignored by default because on GitHub-hosted Windows
    /// runners `CredReadW` still returns the credential right after
    /// `CredDeleteW` succeeds. Run it on a real desktop session:
    /// `cargo test -p iausage-core --lib -- --ignored windows_credential_manager_smoke`
    #[cfg(windows)]
    #[test]
    #[ignore = "real Windows Credential Manager; not deterministic on GitHub Actions runners"]
    fn windows_credential_manager_smoke() {
        const SERVICE: &str = "com.alberth.iausagebar-test";
        let store = KeyringStore { service: SERVICE };
        let account = format!("device-secret-test-{:016x}", rand::random::<u64>());
        let secret: [u8; 32] = rand::random();
        // Failures report only operation, error kind, backend and the test
        // service/account — never the secret.
        let fail = |op: &str, kind: &str| {
            format!(
                "credential smoke failed: op={op} kind={kind} \
                 backend=windows-credential-manager service={SERVICE} account={account}"
            )
        };

        let _ = store.delete(&account); // best-effort cleanup before
        let result = (|| {
            store
                .write(&account, &secret)
                .map_err(|kind| fail("write", &kind))?;
            match store.read(&account).map_err(|kind| fail("read", &kind))? {
                Some(value) if value == secret => {}
                Some(_) => return Err(fail("read", "Mismatch")),
                None => return Err(fail("read", "Missing")),
            }
            store
                .delete(&account)
                .map_err(|kind| fail("delete", &kind))?;
            match store
                .read(&account)
                .map_err(|kind| fail("read-after-delete", &kind))?
            {
                None => Ok(()),
                Some(_) => Err(fail("read-after-delete", "StillPresent")),
            }
        })();
        let _ = store.delete(&account); // best-effort cleanup after
        if let Err(message) = result {
            panic!("{message}");
        }
    }
}
