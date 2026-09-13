//! M5 sync: `SyncPayload` + `EncryptedBlob`.
//!
//! Lo único que cruza al teléfono es este payload normalizado, cifrado con
//! una passphrase que nunca viaja. Credenciales, cookies, rutas absolutas y
//! el estado interno (`ProviderSnapshot`) jamás se serializan aquí: el tipo
//! del cable es distinto del tipo interno a propósito.
//!
//! Formato del cable (JSON):
//! ```json
//! { "v": 1, "alg": "xchacha20poly1305+argon2id",
//!   "salt": "<b64>", "nonce": "<b64>", "ciphertext": "<b64>" }
//! ```
//! El `ciphertext` descifra a JSON canónico (`serde_json::to_vec`, orden de
//! campos de la struct, determinista) de `SyncPayload`.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{atomic_write, read_keyring_entry, AppConfig, SYNC_PASSPHRASE_ACCOUNT};
use crate::paths::{app_config_dir, sync_device_path};
use crate::snapshot_v1::DashboardSnapshotV1;
use crate::SNAPSHOT_SCHEMA_VERSION;

/// Versión del envelope cifrado (independiente del schema del snapshot).
pub const SYNC_BLOB_VERSION: u32 = 1;
pub const SYNC_BLOB_ALG: &str = "xchacha20poly1305+argon2id";
/// Cuenta de keyring donde vive la passphrase de sync (igual que las API keys).
pub const SYNC_KEYRING_ACCOUNT: &str = "sync-passphrase";
/// V2 (Sync V2 passwordless pairing): the key is already 256 bits of CSPRNG
/// output, so there is no passphrase to derive it from — same cipher, no KDF.
pub const SYNC_BLOB_ALG_V2: &str = "xchacha20poly1305";

const ARGON_M_COST_KIB: u32 = 64 * 1024;
const ARGON_T_COST: u32 = 3;
const ARGON_P_COST: u32 = 4;
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
/// Dominio de separación para el fingerprint de verificación de pareo.
const FINGERPRINT_DOMAIN: &str = "iausage-pairing-v1:";
/// Alfabeto Crockford (sin I/L/O/U ambiguos) para el fingerprint legible.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPayload {
    pub schema_version: u32,
    pub device_id: String,
    pub generated_at: String,
    pub snapshot: DashboardSnapshotV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedBlob {
    pub v: u32,
    pub alg: String,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

pub fn build_payload(
    device_id: String,
    generated_at: String,
    snapshot: DashboardSnapshotV1,
) -> SyncPayload {
    SyncPayload {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        device_id,
        generated_at,
        snapshot,
    }
}

/// JSON canónico del payload: `serde_json::to_vec` emite los campos en orden
/// de declaración, así que el mismo payload siempre cifra igual con
/// misma salt/nonce (útil para vectores de test y debugging).
pub fn encode_payload(payload: &SyncPayload) -> Result<Vec<u8>, String> {
    serde_json::to_vec(payload).map_err(|e| e.to_string())
}

pub fn decode_payload(bytes: &[u8]) -> Result<SyncPayload, String> {
    let payload: SyncPayload = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if payload.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(format!(
            "sync: schema_version {} no soportado (se esperaba {})",
            payload.schema_version, SNAPSHOT_SCHEMA_VERSION
        ));
    }
    Ok(payload)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    OsRng.fill_bytes(&mut buf);
    buf
}

pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    if passphrase.is_empty() {
        return Err("sync: passphrase vacía".into());
    }
    let params = argon2::Params::new(ARGON_M_COST_KIB, ARGON_T_COST, ARGON_P_COST, Some(KEY_LEN))
        .map_err(|e| e.to_string())?;
    let ctx = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    ctx.hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| e.to_string())?;
    Ok(key)
}

fn seal(key: &[u8; KEY_LEN], nonce: &[u8; NONCE_LEN], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use chacha20poly1305::aead::{Aead, KeyInit};
    let cipher =
        chacha20poly1305::XChaCha20Poly1305::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = chacha20poly1305::XNonce::from_slice(nonce);
    cipher.encrypt(nonce, plaintext).map_err(|e| e.to_string())
}

fn open(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
) -> Result<Vec<u8>, String> {
    use chacha20poly1305::aead::{Aead, KeyInit};
    let cipher =
        chacha20poly1305::XChaCha20Poly1305::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = chacha20poly1305::XNonce::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
}

fn decode_b64(field: &str, value: &str) -> Result<Vec<u8>, String> {
    B64.decode(value)
        .map_err(|_| format!("sync: campo {field} no es base64 válido"))
}

/// Variante determinista (sales/nonce explícitos) para vectores y tests.
/// En producción se usa [`encrypt_payload`], siempre aleatoria.
pub fn encrypt_payload_with(
    payload: &SyncPayload,
    passphrase: &str,
    salt: &[u8; SALT_LEN],
    nonce: &[u8; NONCE_LEN],
) -> Result<EncryptedBlob, String> {
    let key = derive_key(passphrase, salt)?;
    let ciphertext = seal(&key, nonce, &encode_payload(payload)?)?;
    Ok(EncryptedBlob {
        v: SYNC_BLOB_VERSION,
        alg: SYNC_BLOB_ALG.into(),
        salt: B64.encode(salt),
        nonce: B64.encode(nonce),
        ciphertext: B64.encode(&ciphertext),
    })
}

pub fn encrypt_payload(payload: &SyncPayload, passphrase: &str) -> Result<EncryptedBlob, String> {
    encrypt_payload_with(payload, passphrase, &random_bytes(), &random_bytes())
}

pub fn decrypt_blob(blob: &EncryptedBlob, passphrase: &str) -> Result<SyncPayload, String> {
    if blob.v != SYNC_BLOB_VERSION {
        return Err(format!("sync: blob v{} no soportado", blob.v));
    }
    if blob.alg != SYNC_BLOB_ALG {
        return Err(format!("sync: algoritmo {} no soportado", blob.alg));
    }
    let salt = decode_b64("salt", &blob.salt)?;
    let nonce = decode_b64("nonce", &blob.nonce)?;
    let ciphertext = decode_b64("ciphertext", &blob.ciphertext)?;
    if nonce.len() != NONCE_LEN {
        return Err("sync: nonce con longitud inválida".into());
    }
    let key = derive_key(passphrase, &salt)?;
    let nonce_arr: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| "sync: nonce inválido")?;
    let plaintext = open(&key, &nonce_arr, &ciphertext)
        .map_err(|_| "sync: no se pudo descifrar (¿passphrase incorrecta?)")?;
    decode_payload(&plaintext)
}

/// V2 encrypt: same cipher and wire shape as `encrypt_payload_with`, no
/// Argon2id. `salt` is still populated (16 random, unused-by-design bytes)
/// so `EncryptedBlob` needs no second struct — see the design spec's
/// "Cryptography" section for why this is deliberate, not an oversight.
pub fn encrypt_payload_with_key(
    payload: &SyncPayload,
    key: &[u8; KEY_LEN],
) -> Result<EncryptedBlob, String> {
    let salt: [u8; SALT_LEN] = random_bytes();
    let nonce: [u8; NONCE_LEN] = random_bytes();
    let ciphertext = seal(key, &nonce, &encode_payload(payload)?)?;
    Ok(EncryptedBlob {
        v: SYNC_BLOB_VERSION,
        alg: SYNC_BLOB_ALG_V2.into(),
        salt: B64.encode(salt),
        nonce: B64.encode(nonce),
        ciphertext: B64.encode(&ciphertext),
    })
}

pub fn decrypt_payload_with_key(
    blob: &EncryptedBlob,
    key: &[u8; KEY_LEN],
) -> Result<SyncPayload, String> {
    if blob.v != SYNC_BLOB_VERSION {
        return Err(format!("sync: blob v{} no soportado", blob.v));
    }
    if blob.alg != SYNC_BLOB_ALG_V2 {
        return Err(format!("sync: algoritmo {} no soportado", blob.alg));
    }
    let nonce = decode_b64("nonce", &blob.nonce)?;
    let ciphertext = decode_b64("ciphertext", &blob.ciphertext)?;
    if nonce.len() != NONCE_LEN {
        return Err("sync: nonce con longitud inválida".into());
    }
    let nonce_arr: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| "sync: nonce inválido")?;
    let plaintext = open(key, &nonce_arr, &ciphertext).map_err(|_| "sync: no se pudo descifrar")?;
    decode_payload(&plaintext)
}

fn sync_entry() -> Result<keyring::Entry, String> {
    // El servicio es el mismo que el de las API keys; la cuenta distingue.
    let service = "com.alberth.iausagebar";
    keyring::Entry::new(service, SYNC_PASSPHRASE_ACCOUNT).map_err(|e| e.to_string())
}

/// Guarda la passphrase de sync en el Credential Manager. Vacía = olvidar.
/// Usa el lector tolerante al NUL final de Windows (igual que las API keys):
/// `get_password()` puede fallar aunque la credencial exista.
pub fn store_passphrase(value: &str) -> Result<(), String> {
    let entry = sync_entry()?;
    if value.trim().is_empty() {
        if read_keyring_entry(&entry).is_some() {
            entry.delete_credential().map_err(|e| e.to_string())?;
        }
    } else {
        entry.set_password(value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Solo responde si hay passphrase guardada, nunca su valor.
pub fn has_passphrase() -> bool {
    sync_entry()
        .ok()
        .and_then(|entry| read_keyring_entry(&entry))
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn load_passphrase() -> Result<String, String> {
    let entry = sync_entry()?;
    let value = read_keyring_entry(&entry)
        .ok_or_else(|| "sync: no hay frase secreta guardada".to_string())?;
    if value.trim().is_empty() {
        return Err("sync: no hay frase secreta guardada".into());
    }
    Ok(value)
}

/// Carpeta del blob. Por defecto `<config-dir>/ia-sync`; el usuario la
/// apunta a su Syncthing/OneDrive o la deja local.
pub fn default_export_dir() -> std::path::PathBuf {
    app_config_dir().join("ia-sync")
}

pub fn resolve_export_dir(cfg: &AppConfig) -> std::path::PathBuf {
    match cfg.sync_export_dir.as_deref().map(str::trim) {
        Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
        _ => default_export_dir(),
    }
}

fn blob_name(device_id: &str) -> String {
    format!("{device_id}.json")
}

/// Cifra y escribe el blob de forma atómica. Devuelve la ruta escrita.
pub fn export_blob_to_dir(
    dir: &std::path::Path,
    device_id: &str,
    payload: &SyncPayload,
    passphrase: &str,
) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let blob = encrypt_payload(payload, passphrase)?;
    let body = serde_json::to_vec_pretty(&blob).map_err(|e| e.to_string())?;
    let path = dir.join(blob_name(device_id));
    atomic_write(&path, &body)?;
    Ok(path)
}

/// Exporta el snapshot actual si sync está activo (`None` = desactivado,
/// no es error). Lee device-id y passphrase del sistema.
pub fn export_current_snapshot(
    cfg: &AppConfig,
    snaps: &std::collections::HashMap<String, crate::model::ProviderSnapshot>,
    app_version: Option<String>,
) -> Result<Option<std::path::PathBuf>, String> {
    if !cfg.sync_enabled {
        return Ok(None);
    }
    if cfg.load_recovered {
        return Err("sync: config en recuperación, no se exporta hasta revisar config.toml".into());
    }
    let passphrase = load_passphrase().map_err(|_| {
        "sync: activado sin passphrase; usa `iausage sync set-passphrase` o la pantalla Sync"
            .to_string()
    })?;
    let device_id = load_or_create_device_id()?;
    let catalog = crate::providers::catalog(cfg);
    let snapshot = crate::snapshot_v1::build(snaps, &catalog, crate::model::now_iso(), app_version);
    let payload = build_payload(device_id.clone(), crate::model::now_iso(), snapshot);
    let path = export_blob_to_dir(&resolve_export_dir(cfg), &device_id, &payload, &passphrase)?;
    Ok(Some(path))
}

/// Lee y valida un blob de disco (sin descifrar).
pub fn read_blob_file(path: &std::path::Path) -> Result<EncryptedBlob, String> {
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    let blob: EncryptedBlob = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    if blob.v != SYNC_BLOB_VERSION {
        return Err(format!("sync: blob v{} no soportado", blob.v));
    }
    Ok(blob)
}

/// Lee el id estable del colector o lo genera (hex aleatorio, sin secretos).
pub fn load_or_create_device_id() -> Result<String, String> {
    load_or_create_device_id_in(&sync_device_path())
}

fn load_or_create_device_id_in(path: &std::path::Path) -> Result<String, String> {
    if let Ok(raw) = std::fs::read_to_string(path) {
        let id = raw.trim().to_string();
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let id = hex_id(&random_bytes::<16>());
    atomic_write(path, id.as_bytes())?;
    Ok(id)
}

fn hex_id(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Código corto `XXXX-XXXX` para verificar el pareo en ambas pantallas
/// (anti-MITM en LAN). Determinista por dispositivo; NO es un secreto y
/// jamás se usa como clave: solo confirma que ambos lados ven lo mismo.
pub fn pairing_fingerprint(device_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(FINGERPRINT_DOMAIN.as_bytes());
    hasher.update(device_id.as_bytes());
    let digest = hasher.finalize();
    let mut bits: u64 = 0;
    for (i, byte) in digest.iter().take(5).enumerate() {
        bits |= (*byte as u64) << (8 * (4 - i));
    }
    let chars: String = (0..8)
        .map(|i| CROCKFORD[((bits >> (5 * (7 - i))) & 31) as usize] as char)
        .collect();
    format!("{}-{}", &chars[..4], &chars[4..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::model::{progress_pct, snapshot_ok, VendorId};

    fn sample_payload() -> SyncPayload {
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session", "Sesión", 10.0, None, 18_000, "always",
            )],
        );
        let mut snaps = std::collections::HashMap::new();
        snaps.insert(snap.id.clone(), snap);
        let catalog = vec![crate::model::VendorInfo {
            id: "anthropic".into(),
            name: "Claude Code".into(),
            short: "CLD".into(),
            auth_kind: crate::model::AuthKind::Oauth,
            env_key: None,
            hint: String::new(),
            needs_key: false,
            enabled: true,
            detected: true,
            has_credential: true,
            credential_source: None,
            links: VendorId::Anthropic.links(),
            strategies: vec!["oauth".into()],
            source_preference: None,
        }];
        let snapshot = crate::snapshot_v1::build(&snaps, &catalog, "now".into(), None);
        build_payload("deadbeef".into(), "now".into(), snapshot)
    }

    #[test]
    fn roundtrip_cifrado_descifrado() {
        let payload = sample_payload();
        let blob = encrypt_payload(&payload, "correct horse battery staple").unwrap();
        assert_eq!(blob.v, SYNC_BLOB_VERSION);
        assert_eq!(blob.alg, SYNC_BLOB_ALG);
        // El blob es JSON serializable para el cable/archivo.
        let wire = serde_json::to_string(&blob).unwrap();
        let parsed: EncryptedBlob = serde_json::from_str(&wire).unwrap();
        let back = decrypt_blob(&parsed, "correct horse battery staple").unwrap();
        assert_eq!(back.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(back.device_id, "deadbeef");
        assert_eq!(back.snapshot.providers.len(), 1);
    }

    #[test]
    fn passphrase_incorrecta_falla() {
        let blob = encrypt_payload(&sample_payload(), "una-clave").unwrap();
        assert!(decrypt_blob(&blob, "otra-clave").is_err());
        assert!(decrypt_blob(&blob, "").is_err());
    }

    #[test]
    fn ciphertext_manipulado_falla() {
        let mut blob = encrypt_payload(&sample_payload(), "clave").unwrap();
        let mut raw = B64.decode(&blob.ciphertext).unwrap();
        raw[0] ^= 0x01;
        blob.ciphertext = B64.encode(&raw);
        assert!(decrypt_blob(&blob, "clave").is_err());
    }

    #[test]
    fn misma_sal_nonce_mismo_cifrado() {
        let payload = sample_payload();
        let salt = [7u8; SALT_LEN];
        let nonce = [9u8; NONCE_LEN];
        let a = encrypt_payload_with(&payload, "clave", &salt, &nonce).unwrap();
        let b = encrypt_payload_with(&payload, "clave", &salt, &nonce).unwrap();
        assert_eq!(a.ciphertext, b.ciphertext);
        let back = decrypt_blob(&a, "clave").unwrap();
        assert_eq!(back.device_id, "deadbeef");
    }

    #[test]
    fn sales_aleatorias_no_repiten_cifrado() {
        let payload = sample_payload();
        let a = encrypt_payload(&payload, "clave").unwrap();
        let b = encrypt_payload(&payload, "clave").unwrap();
        assert_ne!(a.salt, b.salt);
        assert_ne!(a.ciphertext, b.ciphertext);
    }

    #[test]
    fn version_futura_se_rechaza() {
        let mut payload = sample_payload();
        payload.schema_version = SNAPSHOT_SCHEMA_VERSION + 1;
        let bytes = serde_json::to_vec(&payload).unwrap();
        let err = decode_payload(&bytes).unwrap_err();
        assert!(err.contains("no soportado"), "{err}");

        let mut blob = encrypt_payload(&sample_payload(), "clave").unwrap();
        blob.v = SYNC_BLOB_VERSION + 1;
        assert!(decrypt_blob(&blob, "clave").is_err());
        blob.v = SYNC_BLOB_VERSION;
        blob.alg = "rot13".into();
        assert!(decrypt_blob(&blob, "clave").is_err());
    }

    // Gatekeeper M5: el payload serializado no puede llevar secretos ni
    // rutas de la máquina, aunque el fixture intente colarlos.
    #[test]
    fn payload_sin_secretos_ni_rutas() {
        let payload = sample_payload();
        let wire = serde_json::to_string(&payload).unwrap();
        // Simula el peor caso: si algún campo futuro arrastrara un secreto,
        // la serialización lo mostraría. Hoy el modelo no los tiene, así que
        // el cable debe estar limpio de estas marcas.
        for marker in [
            "token",
            "cookie",
            "passwd",
            "password",
            "secret",
            "credential",
            "authorization",
            "bearer",
            "api_key",
            "apikey",
            "private_key",
            ".claude",
            ".codex",
        ] {
            assert!(
                !wire.to_lowercase().contains(marker),
                "el cable contiene {marker}"
            );
        }
        // Sin rutas absolutas Windows (en JSON aparecen como C:\\...) ni
        // home Unix. Los esquemas URL (https://...) no cuentan.
        let chars: Vec<char> = wire.chars().collect();
        for (i, w) in chars.windows(3).enumerate() {
            if !w[0].is_ascii_alphabetic() || w[1] != ':' {
                continue;
            }
            let is_drive = w[2] == '\\' || (w[2] == '/' && chars.get(i + 3) != Some(&'/'));
            if is_drive {
                panic!("el cable contiene una ruta absoluta: {}", &wire);
            }
        }
    }

    #[test]
    fn fingerprint_estable_y_legible() {
        let a = pairing_fingerprint("device-1");
        let b = pairing_fingerprint("device-1");
        let c = pairing_fingerprint("device-2");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 9);
        assert_eq!(&a[4..5], "-");
        assert!(a
            .chars()
            .all(|ch| ch == '-' || CROCKFORD.contains(&(ch as u8))));
    }

    /// Vector dorado: fija el formato del cable. Si cambia el algoritmo,
    /// los parámetros Argon2, el JSON canónico o el envelope, este test
    /// rompe a propósito (compatibilidad con el teléfono en juego).
    #[test]
    fn vector_conocido_fija_formato() {
        use crate::snapshot_v1::DashboardSnapshotV1;
        let snapshot = DashboardSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            generated_at: "2026-09-12T00:00:00Z".into(),
            app_version: Some("vector".into()),
            providers: vec![],
        };
        let payload = build_payload(
            "vector-device".into(),
            "2026-09-12T00:00:00Z".into(),
            snapshot,
        );
        let salt: [u8; SALT_LEN] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let nonce: [u8; NONCE_LEN] = [17u8; NONCE_LEN];
        let blob = encrypt_payload_with(&payload, "vector-test-passphrase", &salt, &nonce).unwrap();
        let wire = serde_json::to_string(&blob).unwrap();
        assert_eq!(
            wire,
            "{\"v\":1,\"alg\":\"xchacha20poly1305+argon2id\",\
             \"salt\":\"AQIDBAUGBwgJCgsMDQ4PEA==\",\
             \"nonce\":\"ERERERERERERERERERERERERERERERER\",\
             \"ciphertext\":\"CnwT2OjdYeHwxCsDnU1POJiWQLHsJ4zdZ8IRClCR+DHuldB2zyvD+5NZ5xoWTDEbdo3pCmTFNihgXsdS+ylwdiuC90VqSJ+3yRmjkY8w5KA9cXkgVDATeD5iC0+Plz0UQ4WR8LySM6AT0nC+cxjff1m6hZwVWdoWPsuZ63sX9BIf0RwL9IqlxVk0YeRPgueCpm2/UjUvpe5ZcMcCTiDQxKv/J/BnIIlJtBnvB8o/AQ7TjkF7UG57ChfJzswWBiCJYvwTiVb4yJRcQc8M\"}"
        );
        // Y el vector abre con su passphrase.
        let parsed: EncryptedBlob = serde_json::from_str(&wire).unwrap();
        let back = decrypt_blob(&parsed, "vector-test-passphrase").unwrap();
        assert_eq!(back.device_id, "vector-device");
    }

    #[test]
    fn device_id_persiste_en_disco() {
        let dir = std::env::temp_dir().join(format!("sync-dev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("device-id");
        let first = load_or_create_device_id_in(&path).unwrap();
        assert_eq!(first.len(), 32);
        let second = load_or_create_device_id_in(&path).unwrap();
        assert_eq!(first, second);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_escribe_blob_legible() {
        let dir = std::env::temp_dir().join(format!("sync-exp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let payload = sample_payload();
        let path = export_blob_to_dir(&dir, "abc123", &payload, "clave").unwrap();
        assert_eq!(path, dir.join("abc123.json"));
        let blob = read_blob_file(&path).unwrap();
        assert_eq!(blob.v, SYNC_BLOB_VERSION);
        let back = decrypt_blob(&blob, "clave").unwrap();
        assert_eq!(back.snapshot.providers.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_rechaza_blob_malo() {
        let dir = std::env::temp_dir().join(format!("sync-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roto.json");
        std::fs::write(&path, b"no es json").unwrap();
        assert!(read_blob_file(&path).is_err());
        assert!(read_blob_file(&dir.join("ausente.json")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_current_respeta_desactivado() {
        let cfg = AppConfig::default();
        assert!(!cfg.sync_enabled);
        let snaps = std::collections::HashMap::new();
        let out = export_current_snapshot(&cfg, &snaps, None).unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn export_dir_por_defecto_y_personalizado() {
        let mut cfg = AppConfig::default();
        assert!(resolve_export_dir(&cfg).ends_with("ia-sync"));
        cfg.sync_export_dir = Some("  ".into());
        assert!(resolve_export_dir(&cfg).ends_with("ia-sync"));
        cfg.sync_export_dir = Some("D:/sync-mio".into());
        assert_eq!(
            resolve_export_dir(&cfg),
            std::path::PathBuf::from("D:/sync-mio")
        );
    }

    #[test]
    fn encrypt_decrypt_with_key_round_trips() {
        let payload = sample_payload_for_key_tests();
        let key = [9u8; 32];
        let blob = encrypt_payload_with_key(&payload, &key).unwrap();
        assert_eq!(blob.alg, SYNC_BLOB_ALG_V2);
        let back = decrypt_payload_with_key(&blob, &key).unwrap();
        assert_eq!(back.device_id, payload.device_id);
    }

    #[test]
    fn decrypt_with_key_rejects_a_different_devices_key() {
        let payload = sample_payload_for_key_tests();
        let blob = encrypt_payload_with_key(&payload, &[1u8; 32]).unwrap();
        assert!(decrypt_payload_with_key(&blob, &[2u8; 32]).is_err());
    }

    #[test]
    fn decrypt_with_key_rejects_a_v1_blob() {
        let payload = sample_payload_for_key_tests();
        let v1_blob = encrypt_payload(&payload, "some-passphrase").unwrap();
        assert!(decrypt_payload_with_key(&v1_blob, &[1u8; 32]).is_err());
    }

    fn sample_payload_for_key_tests() -> SyncPayload {
        let snapshot =
            crate::snapshot_v1::build(&std::collections::HashMap::new(), &[], "now".into(), None);
        build_payload("dev-key-test".into(), "now".into(), snapshot)
    }
}
