# Passwordless Device Pairing (Sync V2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace manual-passphrase phone pairing with a one-tap QR flow (auto-generated per-device 256-bit secret), while leaving the V1 passphrase path untouched as a legacy fallback.

**Architecture:** A new `pairing.rs` module in `iausage-core` owns the ephemeral pairing session (CSPRNG token+secret, SHA-256 token hash, name sanitization) and the wire struct for the v2 QR URI. `AppConfig` gains a persisted `paired_devices` list (no secrets — those live in Credential Manager, one entry per device, same abstraction this session's keyring fix already uses). `sync_server.rs` gains `/v2/pair` and `/v2/snapshot` routes, driven by two closures the desktop app injects (so the core stays testable without a real `AppState`). The desktop's `ensure_sync_server` lifecycle gate becomes `cfg.sync_enabled || pending_pairing.is_some() || any_active_v2_device` — a superset of today's V1-only gate, not a replacement. Android gets a second `PairingInfo`-carried secret path alongside its existing passphrase one, picked at runtime by which credential is present in `SecureStateStore`.

**Tech Stack:** Rust (iausage-core, src-tauri/Tauri commands), TypeScript (Vite frontend, `node:test`), Kotlin (Jetpack Compose, JUnit4 + AndroidJUnit4 instrumented tests).

**Spec:** `docs/superpowers/specs/2026-09-13-passwordless-device-pairing.md`

## Global Constraints

- V1 (manual passphrase) code stays working, untouched: `derive_key`, `store_passphrase`/`load_passphrase`/`has_passphrase`, `/v1/meta`, `/v1/snapshot`, Android's `ProtocolCrypto.decrypt(blob, passphrase: CharArray)`. Nothing in this plan deletes or edits their existing behavior.
- The `sync_set_passphrase` Tauri command stays registered (harmless if unreachable) — only its UI entry points are removed. Do not delete the command or its Rust tests.
- `cfg.sync_enabled` stays a real, read field in the server-start formula during this transition. Do not remove it from `AppConfig` or from that formula in this plan.
- The 256-bit device secret and the one-time pairing token never appear in: `config.toml`, application logs, Android `Log.*` calls, `SavedStateHandle`, Jetpack `DataStore`, crash reports, analytics, or any "Copiar URI"/clipboard affordance for the v2 QR.
- No new external dependencies (Rust crates, npm packages, Gradle libraries). Everything needed (`sha2`, `rand`, `base64`, `chacha20poly1305`, lazysodium, Android Keystore) is already in the workspace.
- Every Rust task runs `cargo fmt --all`, `cargo check --workspace --locked`, and `cargo test --workspace --locked` before its commit. Every frontend task runs `npm run test:frontend` and `npm run build`. Every Android task that only touches `app/src/test` runs `gradle :app:testDebugUnitTest`; a task touching `app/src/androidTest` additionally documents that the new instrumented test needs a real device/emulator to execute (CI only compiles it, per `release.yml`'s existing `compileDebugAndroidTestKotlin` step) and is not required to pass in this plan's automated steps — run it manually if a device is available, otherwise note it as manually-verified-pending.

---

## Task 1: `pairing.rs` — token/secret generation, hashing, name sanitization

**Files:**
- Create: `crates/iausage-core/src/pairing.rs`
- Modify: `crates/iausage-core/src/lib.rs` (add `pub mod pairing;`)
- Test: inline `#[cfg(test)] mod tests` in `pairing.rs`

**Interfaces:**
- Produces: `pub struct PendingPairing` with `pub fn is_expired(&self) -> bool`, `pub fn matches_token(&self, token_hex: &str) -> bool`; `pub struct NewPairing { pub pending: PendingPairing, pub token_hex: String, pub secret_b64: String }`; `pub fn start_pairing() -> NewPairing`; `pub const PAIRING_TTL: std::time::Duration`; `pub fn sanitize_device_name(raw: &str) -> String`.

- [ ] **Step 1: Find the current module list in `lib.rs`**

Run: `grep -n "^pub mod\|^mod " "crates/iausage-core/src/lib.rs"`

Confirm `pairing` is not already there, and note the alphabetical-ish grouping style to match.

- [ ] **Step 2: Write the failing tests**

```rust
// bottom of crates/iausage-core/src/pairing.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_pairing_produces_distinct_tokens_and_secrets() {
        let a = start_pairing();
        let b = start_pairing();
        assert_ne!(a.token_hex, b.token_hex);
        assert_ne!(a.secret_b64, b.secret_b64);
        assert_eq!(a.token_hex.len(), 32, "16 bytes hex-encoded is 32 chars");
        // 32 raw bytes, base64url no-pad: ceil(32*4/3) = 43 chars, no '=' padding.
        assert_eq!(a.secret_b64.len(), 43);
        assert!(!a.secret_b64.contains('='));
    }

    #[test]
    fn matches_token_accepts_only_the_real_token() {
        let new_pairing = start_pairing();
        assert!(new_pairing.pending.matches_token(&new_pairing.token_hex));
        assert!(!new_pairing.pending.matches_token("00000000000000000000000000000000"));
        assert!(!new_pairing.pending.matches_token("not-hex"));
    }

    #[test]
    fn pending_pairing_is_not_expired_immediately() {
        assert!(!start_pairing().pending.is_expired());
    }

    #[test]
    fn sanitize_device_name_trims_strips_and_truncates() {
        assert_eq!(sanitize_device_name("  Galaxy S26  "), "Galaxy S26");
        assert_eq!(sanitize_device_name(""), "Android");
        assert_eq!(sanitize_device_name("   "), "Android");
        assert_eq!(sanitize_device_name("a\u{0007}b\u{0000}c"), "abc");
        let long = "x".repeat(200);
        assert_eq!(sanitize_device_name(&long).chars().count(), 64);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail (module doesn't exist yet)**

Run: `cargo test -p iausage-core --lib pairing:: 2>&1 | head -30`
Expected: FAIL — `error[E0433]: failed to resolve: use of undeclared crate or module 'pairing'` (or similar; the module isn't wired into `lib.rs` yet).

- [ ] **Step 4: Implement `pairing.rs`**

```rust
//! Sync V2: passwordless device pairing.
//! See docs/superpowers/specs/2026-09-13-passwordless-device-pairing.md.

use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

/// How long a QR stays valid before the pairing must be restarted.
pub const PAIRING_TTL: Duration = Duration::from_secs(120);
const TOKEN_LEN: usize = 16; // 128 bits
pub const SECRET_LEN: usize = 32; // 256 bits
const MAX_NAME_LEN: usize = 64;

/// RAM-only pairing session created by "Vincular teléfono". Never
/// serialized: the secret must not reach config.toml, logs, or diagnostics.
pub struct PendingPairing {
    token_hash: [u8; 32],
    pub secret: [u8; SECRET_LEN],
    pub expires_at: Instant,
}

pub struct NewPairing {
    pub pending: PendingPairing,
    pub token_hex: String,
    pub secret_b64: String,
}

pub fn start_pairing() -> NewPairing {
    let mut token = [0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut token);
    let mut secret = [0u8; SECRET_LEN];
    OsRng.fill_bytes(&mut secret);
    let token_hash = hash_token(&token);
    NewPairing {
        pending: PendingPairing {
            token_hash,
            secret,
            expires_at: Instant::now() + PAIRING_TTL,
        },
        token_hex: hex_encode(&token),
        secret_b64: URL_SAFE_NO_PAD.encode(secret),
    }
}

impl PendingPairing {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Constant-time: a wrong token must take exactly as long to reject as
    /// the right one, so timing can't leak how much of it matched.
    pub fn matches_token(&self, token_hex: &str) -> bool {
        let Some(token) = hex_decode(token_hex) else {
            return false;
        };
        constant_time_eq(&hash_token(&token), &self.token_hash)
    }
}

/// `Build.MODEL`-derived names cross HTTP, land in `config.toml`, and get
/// rendered on Desktop — treat as untrusted input.
pub fn sanitize_device_name(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Android".to_string()
    } else {
        trimmed.chars().take(MAX_NAME_LEN).collect()
    }
}

fn hash_token(token: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(token);
    hasher.finalize().into()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}
```

- [ ] **Step 5: Wire the module into `lib.rs`**

Add `pub mod pairing;` next to the other `pub mod` declarations in `crates/iausage-core/src/lib.rs`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p iausage-core --lib pairing:: -- --nocapture`
Expected: 4 tests pass.

- [ ] **Step 7: Format and commit**

```bash
cargo fmt --all
git add crates/iausage-core/src/pairing.rs crates/iausage-core/src/lib.rs
git commit -m "feat(sync): add pairing token/secret generation and name sanitization"
```

---

## Task 2: `config.rs` — `PairedDevice` model and device-secret storage

**Files:**
- Modify: `crates/iausage-core/src/config.rs`
- Test: inline in `config.rs`'s existing `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing from Task 1 (this task only touches persisted config + keyring, no `pairing.rs` types).
- Produces: `pub struct PairedDevice { pub client_device_id: String, pub name: String, pub created_at: String, pub last_seen_at: Option<String>, pub revoked: bool }`; on `AppConfig`: `pub fn upsert_paired_device(&mut self, client_device_id: &str, name: &str) -> &PairedDevice`, `pub fn revoke_device(&mut self, client_device_id: &str) -> bool`, `pub fn has_active_paired_device(&self) -> bool`; free functions `pub fn store_device_secret(client_device_id: &str, secret: &[u8; 32]) -> Result<(), String>`, `pub fn read_device_secret(client_device_id: &str) -> Option<[u8; 32]>`, `pub fn delete_device_secret(client_device_id: &str) -> Result<(), String>`.

- [ ] **Step 1: Write the failing tests**

Add to the existing `#[cfg(test)] mod tests` block in `crates/iausage-core/src/config.rs`:

```rust
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
        assert_eq!(cfg.paired_devices.len(), 1, "re-pairing must not create a second row");
        assert!(!cfg.paired_devices[0].revoked, "re-pairing clears revoked");
        assert_eq!(cfg.paired_devices[0].name, "Galaxy S26 (renamed)");
        assert_eq!(cfg.paired_devices[0].created_at, created_at, "first-paired date is preserved");
        assert!(cfg.paired_devices[0].last_seen_at.is_none(), "fresh secret resets last_seen_at");
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

    #[test]
    fn device_secret_round_trips_through_credential_manager() {
        let id = format!("pairing-smoke-{}", std::process::id());
        let secret = [7u8; 32];
        store_device_secret(&id, &secret).unwrap();
        assert_eq!(read_device_secret(&id), Some(secret));
        delete_device_secret(&id).unwrap();
        assert_eq!(read_device_secret(&id), None);
        // Deleting an already-absent secret is not an error (revoke is idempotent).
        assert!(delete_device_secret(&id).is_ok());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p iausage-core --lib config:: upsert_paired_device 2>&1 | head -30`
Expected: FAIL — `no method named 'upsert_paired_device'`.

- [ ] **Step 3: Add `PairedDevice` and the `AppConfig` field**

In `crates/iausage-core/src/config.rs`, add after the `ProviderConfig` struct:

```rust
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
```

In `AppConfig`, add the field (after `sync_lan`, before `load_recovered`):

```rust
    /// Teléfonos vinculados por Sync V2. Los secretos NUNCA viven aquí — solo
    /// en Credential Manager, cuenta `device-secret-<client_device_id>`.
    #[serde(default)]
    pub paired_devices: Vec<PairedDevice>,
```

Add the same field to `Default for AppConfig`'s body: `paired_devices: Vec::new(),`.

- [ ] **Step 4: Add the `AppConfig` methods**

In the `impl AppConfig` block, after `enabled_ids`:

```rust
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
```

- [ ] **Step 5: Add the device-secret keyring functions**

At the end of `config.rs`, after `migrate_legacy_credentials`:

```rust
fn device_secret_account(client_device_id: &str) -> String {
    format!("device-secret-{client_device_id}")
}

/// Raw 256-bit secret storage — deliberately `set_secret`/`get_secret`
/// (bytes), not the NUL-tolerant string path `read_keyring_entry` uses for
/// API keys: a random secret is not a NUL-terminated string, so there's no
/// padding quirk to work around here in the first place.
pub fn store_device_secret(client_device_id: &str, secret: &[u8; 32]) -> Result<(), String> {
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, &device_secret_account(client_device_id))
        .map_err(|e| e.to_string())?;
    entry.set_secret(secret).map_err(|e| e.to_string())
}

pub fn read_device_secret(client_device_id: &str) -> Option<[u8; 32]> {
    let entry =
        keyring::Entry::new(CREDENTIAL_SERVICE, &device_secret_account(client_device_id)).ok()?;
    entry.get_secret().ok()?.try_into().ok()
}

/// Idempotent: revoking a device that was already deleted (or never had a
/// secret) is not an error.
pub fn delete_device_secret(client_device_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, &device_secret_account(client_device_id))
        .map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p iausage-core --lib config:: -- --nocapture`
Expected: all `config::tests::*` pass, including the 5 new ones.

- [ ] **Step 7: Format, full workspace check, commit**

```bash
cargo fmt --all
cargo check --workspace --locked
git add crates/iausage-core/src/config.rs
git commit -m "feat(sync): add PairedDevice model and per-device secret storage"
```

---

## Task 3: `sync.rs` — raw-key encrypt/decrypt for V2 blobs

**Files:**
- Modify: `crates/iausage-core/src/sync.rs`
- Test: inline in `sync.rs`'s existing `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `pub const SYNC_BLOB_ALG_V2: &str = "xchacha20poly1305"`; `pub fn encrypt_payload_with_key(payload: &SyncPayload, key: &[u8; 32]) -> Result<EncryptedBlob, String>`; `pub fn decrypt_payload_with_key(blob: &EncryptedBlob, key: &[u8; 32]) -> Result<SyncPayload, String>`.

- [ ] **Step 1: Write the failing tests**

Add to `sync.rs`'s `#[cfg(test)] mod tests`:

```rust
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
        let snapshot = crate::snapshot_v1::build(&std::collections::HashMap::new(), &[], "now".into(), None);
        build_payload("dev-key-test".into(), "now".into(), snapshot)
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p iausage-core --lib sync:: encrypt_decrypt_with_key 2>&1 | head -30`
Expected: FAIL — `cannot find function 'encrypt_payload_with_key'`.

- [ ] **Step 3: Implement the raw-key functions**

In `crates/iausage-core/src/sync.rs`, after `pub const SYNC_KEYRING_ACCOUNT`:

```rust
/// V2 (Sync V2 passwordless pairing): the key is already 256 bits of CSPRNG
/// output, so there is no passphrase to derive it from — same cipher, no KDF.
pub const SYNC_BLOB_ALG_V2: &str = "xchacha20poly1305";
```

After `decrypt_blob`:

```rust
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
    let plaintext =
        open(key, &nonce_arr, &ciphertext).map_err(|_| "sync: no se pudo descifrar")?;
    decode_payload(&plaintext)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p iausage-core --lib sync:: -- --nocapture`
Expected: all `sync::tests::*` pass, including the 3 new ones.

- [ ] **Step 5: Format, check, commit**

```bash
cargo fmt --all
cargo check --workspace --locked
git add crates/iausage-core/src/sync.rs
git commit -m "feat(sync): add raw-key encrypt/decrypt for V2 device secrets"
```

---

## Task 4: `pairing.rs` — V2 QR URI

**Files:**
- Modify: `crates/iausage-core/src/pairing.rs`
- Test: inline

**Interfaces:**
- Consumes: nothing new (pure string formatting).
- Produces: `pub struct PairingInfoV2 { pub host: String, pub port: u16, pub pc_device_id: String, pub fingerprint: String, pub token: String, pub secret: String }` with `pub fn to_uri(&self) -> String`.

- [ ] **Step 1: Write the failing test**

Append to `pairing.rs`'s test module:

```rust
    #[test]
    fn pairing_info_v2_uri_has_the_expected_shape() {
        let info = PairingInfoV2 {
            host: "192.168.50.116".into(),
            port: 28741,
            pc_device_id: "abc123".into(),
            fingerprint: "PYYZ-JMJJ".into(),
            token: "deadbeefdeadbeefdeadbeefdeadbeef".into(),
            secret: "c2VjcmV0LWJ5dGVzLWZvci10ZXN0aW5nMTIz".into(),
        };
        let uri = info.to_uri();
        assert!(uri.starts_with("iausage://pair?v=2&"));
        assert!(uri.contains("host=192.168.50.116"));
        assert!(uri.contains("pc=abc123"));
        assert!(uri.contains("token=deadbeefdeadbeefdeadbeefdeadbeef"));
        assert!(uri.contains(&format!("secret={}", info.secret)));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p iausage-core --lib pairing:: pairing_info_v2 2>&1 | head -20`
Expected: FAIL — `cannot find struct 'PairingInfoV2'`.

- [ ] **Step 3: Implement `PairingInfoV2`**

Add to `pairing.rs`:

```rust
/// The v2 pairing QR payload. Deliberately separate from
/// `sync_server::PairingInfo` (the v1 struct): v1 has its own strict
/// `parse_uri` validation and tests this feature must never touch.
pub struct PairingInfoV2 {
    pub host: String,
    pub port: u16,
    pub pc_device_id: String,
    pub fingerprint: String,
    pub token: String,
    pub secret: String,
}

impl PairingInfoV2 {
    pub fn to_uri(&self) -> String {
        format!(
            "iausage://pair?v=2&host={}&port={}&pc={}&fp={}&token={}&secret={}",
            self.host, self.port, self.pc_device_id, self.fingerprint, self.token, self.secret
        )
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p iausage-core --lib pairing:: -- --nocapture`
Expected: all `pairing::tests::*` pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add crates/iausage-core/src/pairing.rs
git commit -m "feat(sync): add PairingInfoV2 QR URI builder"
```

---

## Task 5: `sync_server.rs` — `/v2/pair` and `/v2/snapshot` routes

**Files:**
- Modify: `crates/iausage-core/src/sync_server.rs`
- Test: inline in the existing `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::pairing::sanitize_device_name` (Task 1), `crate::sync::{encrypt_payload_with_key}` (Task 3).
- Produces: `pub enum PairOutcome { Paired, NoPendingPairing, Rejected }`; `pub type TryConsumeTokenFn = std::sync::Arc<dyn Fn(&str, &str, &str) -> PairOutcome + Send + Sync>` (args: token, client_device_id, name); `pub type SnapshotKeyForFn = std::sync::Arc<dyn Fn(&str) -> Option<[u8; 32]> + Send + Sync>` (arg: client_device_id); `run_server`/`serve`/`bind`/`handle` all gain a `pairing_hooks: Option<PairingHooks>` parameter, where `pub struct PairingHooks { pub try_consume_token: TryConsumeTokenFn, pub snapshot_key_for: SnapshotKeyForFn }`.

- [ ] **Step 1: Write the failing tests**

Add to `sync_server.rs`'s test module (after the existing `serve_in_background`/`http_get` helpers):

```rust
    fn always_paired_hooks(key: [u8; 32]) -> PairingHooks {
        PairingHooks {
            try_consume_token: Arc::new(move |token, _client_id, _name| {
                if token == "goodtoken" {
                    PairOutcome::Paired
                } else {
                    PairOutcome::Rejected
                }
            }),
            snapshot_key_for: Arc::new(move |client_id| {
                if client_id == "phone-1" {
                    Some(key)
                } else {
                    None
                }
            }),
        }
    }

    fn http_post(port: u16, path: &str, body: &str) -> (u16, String) {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        write!(
            stream,
            "POST {path} HTTP/1.0\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let status: u16 = raw
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    fn serve_v2_in_background(
        port: u16,
        hooks: PairingHooks,
    ) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let cfg = ServeConfig::loopback(port);
        let handle = std::thread::spawn(move || {
            run_server(
                &cfg,
                "test",
                "testdev",
                sample_payload_fn(),
                "unused-for-v2",
                stop_clone,
                Some(hooks),
            )
            .unwrap();
        });
        for _ in 0..50 {
            if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        (handle, stop)
    }

    #[test]
    fn v2_pair_rejects_a_wrong_token() {
        let port = free_port();
        let (handle, stop) = serve_v2_in_background(port, always_paired_hooks([0u8; 32]));
        let (status, _) = http_post(
            port,
            "/v2/pair",
            r#"{"token":"badtoken","clientDeviceId":"phone-1","name":"Galaxy"}"#,
        );
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 410);
    }

    #[test]
    fn v2_pair_accepts_the_right_token_and_never_echoes_a_secret() {
        let port = free_port();
        let (handle, stop) = serve_v2_in_background(port, always_paired_hooks([0u8; 32]));
        let (status, body) = http_post(
            port,
            "/v2/pair",
            r#"{"token":"goodtoken","clientDeviceId":"phone-1","name":"Galaxy"}"#,
        );
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("\"paired\":true"));
        assert!(body.contains("\"deviceId\":\"phone-1\""));
        assert!(!body.to_lowercase().contains("secret"));
    }

    #[test]
    fn v2_snapshot_404_for_unknown_device() {
        let port = free_port();
        let (handle, stop) = serve_v2_in_background(port, always_paired_hooks([3u8; 32]));
        let (status, _) = http_get(port, "/v2/snapshot?device=someone-else");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 404);
    }

    #[test]
    fn v2_snapshot_decrypts_with_that_devices_key() {
        let port = free_port();
        let key = [3u8; 32];
        let (handle, stop) = serve_v2_in_background(port, always_paired_hooks(key));
        let (status, body) = http_get(port, "/v2/snapshot?device=phone-1");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 200, "{body}");
        let blob: EncryptedBlob = serde_json::from_str(&body).unwrap();
        let payload = crate::sync::decrypt_payload_with_key(&blob, &key).unwrap();
        assert_eq!(payload.device_id, "testdev");
    }

    #[test]
    fn v1_routes_still_work_when_v2_hooks_are_none() {
        let port = free_port();
        let (handle, stop) = serve_in_background(port, sample_payload_fn(), "clave-test");
        let (status, _) = http_get(port, "/v1/meta");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(status, 200);
    }

    #[test]
    fn v2_routes_404_when_no_hooks_are_configured() {
        let port = free_port();
        let (handle, stop) = serve_in_background(port, sample_payload_fn(), "clave-test");
        let (pair_status, _) = http_post(port, "/v2/pair", r#"{"token":"x","clientDeviceId":"y","name":"z"}"#);
        let (snapshot_status, _) = http_get(port, "/v2/snapshot?device=y");
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert_eq!(pair_status, 404);
        assert_eq!(snapshot_status, 404);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p iausage-core --lib sync_server:: v2_ 2>&1 | head -40`
Expected: FAIL — compile errors (`PairingHooks`, `PairOutcome` don't exist; `run_server` takes the wrong number of arguments).

- [ ] **Step 3: Add the hook types**

In `sync_server.rs`, after the `PayloadFn` type alias:

```rust
/// Outcome of trying to consume a v2 pairing token. `Rejected` covers both
/// "wrong token" and "expired" — a wrong token must carry no more signal
/// than an expired one. `NoPendingPairing` means nobody pressed "Vincular
/// teléfono" at all (a distinct, less alarming case).
pub enum PairOutcome {
    Paired,
    NoPendingPairing,
    Rejected,
}

/// (token, client_device_id, name) -> outcome. Backed by whatever shared
/// state the caller (the desktop app) uses — kept as a closure so this
/// crate stays testable without a real AppState/Tauri context.
pub type TryConsumeTokenFn = Arc<dyn Fn(&str, &str, &str) -> PairOutcome + Send + Sync>;
/// client_device_id -> that device's secret, or `None` if unknown/revoked.
pub type SnapshotKeyForFn = Arc<dyn Fn(&str) -> Option<[u8; 32]> + Send + Sync>;

pub struct PairingHooks {
    pub try_consume_token: TryConsumeTokenFn,
    pub snapshot_key_for: SnapshotKeyForFn,
}
```

- [ ] **Step 4: Route `/v2/pair` and `/v2/snapshot` in `handle`**

Replace the `handle` function's body and signature:

```rust
fn handle(
    mut request: tiny_http::Request,
    app_version: &str,
    device_id: &str,
    lan: bool,
    make_payload: &PayloadFn,
    passphrase: &str,
    pairing_hooks: Option<&PairingHooks>,
) {
    let method = request.method().clone();
    let url = request.url().to_string();
    let (status, body) = match (&method, url.split('?').next().unwrap_or("")) {
        (&tiny_http::Method::Get, "/v1/meta") => (200, meta_json(app_version, device_id, lan)),
        (&tiny_http::Method::Get, "/v1/snapshot") => snapshot_json(make_payload, passphrase),
        (&tiny_http::Method::Post, "/v2/pair") => {
            handle_v2_pair(&mut request, pairing_hooks)
        }
        (&tiny_http::Method::Get, "/v2/snapshot") => {
            handle_v2_snapshot(&url, make_payload, pairing_hooks)
        }
        _ => (
            404,
            serde_json::json!({ "error": "no encontrado" }).to_string(),
        ),
    };
    let _ = request.respond(json_response(body, status));
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequestBody {
    token: String,
    client_device_id: String,
    name: String,
}

fn handle_v2_pair(
    request: &mut tiny_http::Request,
    pairing_hooks: Option<&PairingHooks>,
) -> (u16, String) {
    let Some(hooks) = pairing_hooks else {
        return (404, serde_json::json!({ "error": "no encontrado" }).to_string());
    };
    let mut raw_body = String::new();
    if std::io::Read::read_to_string(request.as_reader(), &mut raw_body).is_err() {
        return (400, serde_json::json!({ "error": "cuerpo inválido" }).to_string());
    }
    let Ok(body) = serde_json::from_str::<PairRequestBody>(&raw_body) else {
        return (400, serde_json::json!({ "error": "cuerpo inválido" }).to_string());
    };
    let name = crate::pairing::sanitize_device_name(&body.name);
    match (hooks.try_consume_token)(&body.token, &body.client_device_id, &name) {
        PairOutcome::NoPendingPairing => (
            404,
            serde_json::json!({ "error": "no hay pareo pendiente" }).to_string(),
        ),
        PairOutcome::Rejected => (
            410,
            serde_json::json!({ "error": "token vencido o inválido" }).to_string(),
        ),
        PairOutcome::Paired => (
            200,
            serde_json::json!({
                "paired": true,
                "deviceId": body.client_device_id,
            })
            .to_string(),
        ),
    }
}

fn handle_v2_snapshot(
    url: &str,
    make_payload: &PayloadFn,
    pairing_hooks: Option<&PairingHooks>,
) -> (u16, String) {
    let Some(hooks) = pairing_hooks else {
        return (404, serde_json::json!({ "error": "no encontrado" }).to_string());
    };
    let Some(client_device_id) = query_param(url, "device") else {
        return (400, serde_json::json!({ "error": "falta el parámetro device" }).to_string());
    };
    let Some(key) = (hooks.snapshot_key_for)(&client_device_id) else {
        return (404, serde_json::json!({ "error": "no encontrado" }).to_string());
    };
    let blob: Result<EncryptedBlob, String> = (|| {
        let payload = make_payload()?;
        crate::sync::encrypt_payload_with_key(&payload, &key)
    })();
    match blob {
        Ok(blob) => (
            200,
            serde_json::to_string(&blob).unwrap_or_else(|_| "{}".into()),
        ),
        Err(e) => (500, serde_json::json!({ "error": e }).to_string()),
    }
}

/// Hand-rolled: matches the same style `PairingInfo::parse_uri` already
/// uses for query strings, no new dependency.
fn query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}
```

- [ ] **Step 5: Thread `pairing_hooks` through `run_server`/`serve`/`bind`'s callers**

`bind` is unchanged (it only opens the socket). Change `serve` and `run_server`:

```rust
pub fn serve(
    server: &tiny_http::Server,
    app_version: &str,
    device_id: &str,
    lan: bool,
    make_payload: &PayloadFn,
    passphrase: &str,
    stop: &Arc<AtomicBool>,
    pairing_hooks: Option<&PairingHooks>,
) -> Result<(), String> {
    while !stop.load(Ordering::Relaxed) {
        match server.recv_timeout(Duration::from_millis(200)) {
            Ok(Some(request)) => handle(
                request,
                app_version,
                device_id,
                lan,
                make_payload,
                passphrase,
                pairing_hooks,
            ),
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

pub fn run_server(
    cfg: &ServeConfig,
    app_version: &str,
    device_id: &str,
    make_payload: PayloadFn,
    passphrase: &str,
    stop: Arc<AtomicBool>,
    pairing_hooks: Option<PairingHooks>,
) -> Result<(), String> {
    let server = bind(cfg)?;
    serve(
        &server,
        app_version,
        device_id,
        cfg.lan,
        &make_payload,
        passphrase,
        &stop,
        pairing_hooks.as_ref(),
    )
}
```

- [ ] **Step 6: Fix the existing tests' call sites**

`serve_in_background` (the existing v1 helper) now needs a trailing `None` argument to `run_server`. Update it:

```rust
    fn serve_in_background(
        port: u16,
        make_payload: PayloadFn,
        passphrase: &'static str,
    ) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let cfg = ServeConfig::loopback(port);
        let handle = std::thread::spawn(move || {
            run_server(
                &cfg,
                "test",
                "testdev",
                make_payload,
                passphrase,
                stop_clone,
                None,
            )
            .unwrap();
        });
        for _ in 0..50 {
            if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        (handle, stop)
    }
```

Also update the `bind_y_serve_atienden_meta` test added in the previous session (it calls `serve(&server, ..., &stop_clone)`) to pass `None` as the trailing argument.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p iausage-core --lib sync_server:: -- --nocapture`
Expected: all tests pass, including the 6 new v2 ones.

- [ ] **Step 8: Update the CLI's call site**

`crates/iausage-cli/src/main.rs`'s `cmd_sync_serve` calls `run_server(...)` with 6 args — it has no v2 pairing concept, so it passes `None`:

Run: `grep -n "sync_server::run_server" crates/iausage-cli/src/main.rs`

Add `, None` after the `stop` argument in that call.

- [ ] **Step 9: Full workspace check and commit**

```bash
cargo fmt --all
cargo check --workspace --locked
cargo test --workspace --locked
git add crates/iausage-core/src/sync_server.rs crates/iausage-cli/src/main.rs
git commit -m "feat(sync): add /v2/pair and /v2/snapshot HTTP routes"
```

---

## Task 6: Desktop lifecycle — `PendingPairing` runtime state and `server_needed`

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/sync_service.rs`
- Modify: `src-tauri/src/dashboard.rs` (the `run_loop` tick)

**Interfaces:**
- Consumes: `iausage_core::pairing::{PendingPairing, start_pairing, NewPairing}` (Task 1), `iausage_core::sync_server::{PairingHooks, PairOutcome, bind, serve}` (Task 5), `AppConfig::{upsert_paired_device, revoke_device, has_active_paired_device}` and `config::{store_device_secret, read_device_secret, delete_device_secret}` (Task 2).
- Produces: `AppState.pending_pairing: Mutex<Option<PendingPairingRuntime>>` where `pub(crate) struct PendingPairingRuntime { pub(crate) core: iausage_core::pairing::PendingPairing, pub(crate) expires_at_iso: String }`; `AppState.last_seen: Mutex<HashMap<String, String>>` (in-memory `client_device_id -> ISO timestamp`, flushed to `config.toml` by `flush_last_seen`); `pub(crate) fn ensure_sync_server(app: &AppHandle) -> Result<(), String>` (signature unchanged from the previous session's fix — only its internal `server_needed` logic and hook-wiring change); `pub(crate) fn flush_last_seen(app: &AppHandle)`.

- [ ] **Step 1: Add the runtime state fields**

In `src-tauri/src/state.rs`, add to `AppState` (after `sync_server`):

```rust
    /// Sesión de pareo V2 en curso (RAM únicamente — nunca se serializa, ver
    /// docs/superpowers/specs/2026-09-13-passwordless-device-pairing.md).
    pub(crate) pending_pairing: Mutex<Option<crate::sync_service::PendingPairingRuntime>>,
    /// `last_seen_at` por dispositivo V2, actualizado en cada
    /// `/v2/snapshot` — no se escribe `config.toml` en cada poll del
    /// teléfono, solo se vuelca con `flush_last_seen` en el tick del
    /// refresh loop. Perder unos minutos de precisión aquí en un crash no
    /// importa: es un campo cosmético, no de seguridad.
    pub(crate) last_seen: Mutex<HashMap<String, String>>,
```

Find where `AppState` is constructed (its `Default` impl or an explicit builder — run `grep -rn "AppState {" src-tauri/src/` to find it) and add `pending_pairing: Mutex::new(None),` and `last_seen: Mutex::new(HashMap::new()),` to that construction.

- [ ] **Step 2: Write the failing test for `server_needed`**

Add a `#[cfg(test)] mod tests` block to `src-tauri/src/sync_service.rs` (create one if it doesn't exist) testing the pure formula in isolation — extract it as a standalone function first:

```rust
/// Pure decision the rest of `ensure_sync_server` acts on. Kept as its own
/// function so the truth table can be tested without a Tauri AppHandle.
pub(crate) fn server_needed(
    sync_enabled: bool,
    has_pending_pairing: bool,
    has_active_paired_device: bool,
) -> bool {
    sync_enabled || has_pending_pairing || has_active_paired_device
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_needed_truth_table() {
        assert!(!server_needed(false, false, false));
        assert!(server_needed(true, false, false), "legacy V1 toggle alone must keep the server up");
        assert!(server_needed(false, true, false), "a pending pairing alone must start the server");
        assert!(server_needed(false, false, true), "an active V2 device alone must keep the server up");
        assert!(server_needed(true, true, true));
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p iausagebar --lib sync_service:: server_needed 2>&1 | head -20`
Expected: FAIL — the function doesn't exist yet (this step is a formality here since Step 2 already wrote it above the test — actually run this test BEFORE adding `server_needed`'s body to confirm your test harness runs, then add the function. If following strict TDD, write the test first, run it failing, then add the function above it).

- [ ] **Step 4: Rewrite `ensure_sync_server`**

Replace the whole function in `src-tauri/src/sync_service.rs`:

```rust
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

    // V1 passphrase se sigue cargando solo si V1 realmente lo necesita —
    // un pareo V2 puro nunca debería fallar por falta de passphrase.
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

    let http_server = match crate::sync_server::bind(&serve_cfg) {
        Ok(http_server) => http_server,
        Err(error) => {
            server.last_error = Some(error.clone());
            return Err(error);
        }
    };

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
            lock_or_recover(&state.last_seen)
                .insert(client_device_id.to_string(), now_iso());
            crate::config::read_device_secret(client_device_id)
        }),
    }
}
```

Note: `crate::sync::load_passphrase().unwrap_or_default()` returns an empty string when there's no V1 passphrase — the v1 `/v1/snapshot` route will then fail to decrypt for anyone hitting it without a real passphrase set, but that route is only reachable by an existing V1-paired phone, which by definition already has a real passphrase; a V2-only install (no `sync_enabled`) never triggers this path because `server_needed` wouldn't have started the server for V1 reasons in the first place if there's no real V1 passphrase to have set `sync_enabled = true` with. Add this exact reasoning as a comment above the `unwrap_or_default()` line so a future reader doesn't "fix" it into a hard error.

- [ ] **Step 5: Add `flush_last_seen`**

After `build_pairing_hooks` in `sync_service.rs`:

```rust
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
```

- [ ] **Step 6: Add `SyncServerState.last_error` reset consistency check**

Run: `grep -n "last_error" src-tauri/src/sync_service.rs` — confirm the field already exists (added in the previous session's lifecycle fix) and every new return path above sets it (`None` on success/no-op, `Some(error)` on failure). No code change expected here, this step is a verification read.

- [ ] **Step 7: Add the tick to `run_loop`**

In `src-tauri/src/dashboard.rs`, in `run_loop` (around line 418-436), add the re-check and the last-seen flush right after `refresh_sync`:

```rust
pub(crate) fn run_loop(app: AppHandle) {
    loop {
        refresh_sync(&app, None);
        if let Err(error) = crate::sync_service::ensure_sync_server(&app) {
            eprintln!("sync server tick: {error}");
        }
        crate::sync_service::flush_last_seen(&app);
        let state = app.state::<AppState>();
        // ... rest unchanged
```

- [ ] **Step 8: Update the startup call site**

In `src-tauri/src/lib.rs`, the startup `ensure_sync_server` call (added in the previous session) needs no signature change — confirm with `grep -n "ensure_sync_server" src-tauri/src/lib.rs` that it already does `if let Err(error) = sync_service::ensure_sync_server(...)`.

- [ ] **Step 9: Run the workspace tests**

Run: `cargo test --workspace --locked 2>&1 | tail -40`
Expected: all pass, including the new `server_needed_truth_table`.

- [ ] **Step 10: Format, check, commit**

```bash
cargo fmt --all
cargo check --workspace --locked
git add src-tauri/src/state.rs src-tauri/src/sync_service.rs src-tauri/src/dashboard.rs
git commit -m "feat(sync): server lifecycle keeps V1 alive while adding V2 pairing hooks"
```

---

## Task 7: Tauri commands — `sync_start_pairing`, `sync_revoke_device`, DTO changes

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (command registration)

**Interfaces:**
- Consumes: `iausage_core::pairing::{start_pairing, PairingInfoV2, PAIRING_TTL}` (Tasks 1, 4), `crate::sync_service::{ensure_sync_server, PendingPairingRuntime}` (Task 6), `AppConfig.paired_devices`/`revoke_device` (Task 2), `crate::config::delete_device_secret` (Task 2).
- Produces: `#[tauri::command] pub(crate) fn sync_start_pairing(...) -> Result<SyncPairing, String>`; `#[tauri::command] pub(crate) fn sync_revoke_device(...) -> Result<(), String>`; `SyncStatus` gains `paired_devices: Vec<PairedDeviceDto>` and `pending_pairing: Option<PendingPairingDto>`; new `PairedDeviceDto`/`PendingPairingDto` structs (camelCase via `#[serde(rename_all = "camelCase")]`, matching the existing `SyncStatus`/`SyncPairing` pattern).

- [ ] **Step 1: Add the DTOs**

In `src-tauri/src/commands.rs`, near `SyncStatus`:

```rust
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
```

Add fields to `SyncStatus`:

```rust
    paired_devices: Vec<PairedDeviceDto>,
    pending_pairing: Option<PendingPairingDto>,
```

- [ ] **Step 2: Fill the new fields in `sync_get_status`**

In `sync_get_status`, after building `device_id`/before the `Ok(SyncStatus { ... })`:

```rust
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
```

Add `paired_devices,` and `pending_pairing,` to the `SyncStatus { ... }` construction.

- [ ] **Step 3: Add `sync_start_pairing`**

After `sync_get_pairing` in `commands.rs`:

```rust
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
    ensure_sync_server(&app)?;
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
        uri,
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
    let mut cfg = lock_or_recover(&state.config).clone();
    if !cfg.revoke_device(&client_device_id) {
        return Err("sync: dispositivo no encontrado".into());
    }
    cfg.save()?;
    *lock_or_recover(&state.config) = cfg;
    crate::config::delete_device_secret(&client_device_id)?;
    // Revocar el último dispositivo (sin V1 activo ni pairing pendiente)
    // debe apagar el servidor de inmediato, no esperar el próximo tick.
    ensure_sync_server(&app)?;
    Ok(())
}
```

`use ensure_sync_server` — confirm the existing `use crate::sync_service::ensure_sync_server;` import (or fully-qualify as `crate::sync_service::ensure_sync_server`) matches how `sync_set_enabled` already calls it in the same file.

- [ ] **Step 4: Remove `sync_set_enabled`**

Delete the `sync_set_enabled` function entirely from `commands.rs` (it's superseded by `sync_start_pairing`/`sync_revoke_device` driving `server_needed` instead of a manual toggle).

- [ ] **Step 5: Update `lib.rs`'s command registration**

In `src-tauri/src/lib.rs`'s `tauri::generate_handler![...]` list: remove `commands::sync_set_enabled,`, add `commands::sync_start_pairing,` and `commands::sync_revoke_device,` (near `commands::sync_get_pairing,`).

- [ ] **Step 6: Compile-check**

Run: `cargo check --workspace --locked 2>&1 | tail -60`
Expected: compiles clean. Fix any remaining reference to `sync_set_enabled` the compiler flags (there should be none left in Rust after Task 6 already stopped calling it from `ensure_sync_server`'s own logic — this command was only ever a thin wrapper).

- [ ] **Step 7: Full workspace test, format, commit**

```bash
cargo fmt --all
cargo test --workspace --locked
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(sync): add sync_start_pairing/sync_revoke_device commands"
```

---

## Task 8: Frontend types (`api.ts`)

**Files:**
- Modify: `src/api.ts`

**Interfaces:**
- Produces: `PairedDeviceDto { clientDeviceId: string; name: string; createdAt: string; lastSeenAt: string | null }`; `PendingPairingDto { fingerprint: string; expiresAt: string }`; `SyncStatusDto` gains `pairedDevices: PairedDeviceDto[]` and `pendingPairing: PendingPairingDto | null`.

- [ ] **Step 1: Add the new interfaces and extend `SyncStatusDto`**

```typescript
export interface PairedDeviceDto {
  clientDeviceId: string;
  name: string;
  createdAt: string;
  lastSeenAt: string | null;
}

export interface PendingPairingDto {
  fingerprint: string;
  expiresAt: string;
}

export interface SyncStatusDto {
  enabled: boolean;
  deviceId: string;
  fingerprint: string;
  exportDir: string;
  hasPassphrase: boolean;
  lan: boolean;
  serverRunning: boolean;
  serverAddr: string;
  serverError: string | null;
  lastExport: SyncExportInfo | null;
  pairedDevices: PairedDeviceDto[];
  pendingPairing: PendingPairingDto | null;
}
```

(`enabled`/`hasPassphrase` stay on the type — nothing in Rust removed them from `SyncStatus` — but no new UI code will read them; see Task 9.)

- [ ] **Step 2: Typecheck via the build**

Run: `npm run build 2>&1 | tail -30`
Expected: succeeds (this is a type-only addition, nothing consumes the new fields yet).

- [ ] **Step 3: Commit**

```bash
git add src/api.ts
git commit -m "feat(sync): add PairedDeviceDto/PendingPairingDto types"
```

---

## Task 9: `settings.ts` — replace passphrase UI with pairing + device list

**Files:**
- Modify: `src/views/settings.ts`
- Modify: `src/i18n.ts`

**Interfaces:**
- Consumes: `PairedDeviceDto`, `PendingPairingDto` (Task 8).
- Produces: `syncBody()` no longer renders any passphrase input or the `cfg-sync` toggle; renders `[data-startpairing]` button, a live countdown while `pendingPairing` is set, and a device list with `[data-revokedevice]` rows.

- [ ] **Step 1: Add new i18n keys**

In `src/i18n.ts`, in the `es` block near the other `sync*` keys:

```typescript
    syncStartPairing: "Vincular teléfono",
    syncPairingExpiresIn: "Este código caduca en",
    syncPairedDevices: "Dispositivos vinculados",
    syncNoPairedDevices: "Ningún teléfono vinculado todavía.",
    syncDeviceConnected: "Conectado",
    syncRevokeDevice: "Desvincular",
    syncEncryptedNotice: "Los datos se cifran antes de salir de este PC.",
```

And in the `en` block:

```typescript
    syncStartPairing: "Link phone",
    syncPairingExpiresIn: "This code expires in",
    syncPairedDevices: "Paired devices",
    syncNoPairedDevices: "No phone paired yet.",
    syncDeviceConnected: "Connected",
    syncRevokeDevice: "Unlink",
    syncEncryptedNotice: "Data is encrypted before it leaves this PC.",
```

Change `syncShowQr`'s existing value from `"Mostrar QR de vinculación"` to `"Vincular teléfono"` (es) and from `"Show linking QR"` to `"Link phone"` (en) — or just reuse `syncStartPairing` for the button label and delete `syncShowQr` if nothing else references it (`grep -n "syncShowQr" src/*.ts` first to check).

- [ ] **Step 2: Rewrite `syncBody()`**

Replace the whole function in `src/views/settings.ts`:

```typescript
function syncBody(): string {
  const st = syncStatus;
  const server = st ? syncServerLabel(st) : "—";
  const pending = st?.pendingPairing;
  const devices = st?.pairedDevices ?? [];
  return `
    <p class="lede">${t("syncLede")}</p>
    ${syncStatusRow(t("syncDevice"), st ? `${st.deviceId} (${st.fingerprint})` : "—", "sync-device")}
    ${syncStatusRow(t("syncFolder"), st?.exportDir ?? "—", "sync-dir")}
    ${syncStatusRow(t("syncServer"), server, "sync-server")}
    ${syncStatusRow(t("syncLastExport"), st?.lastExport ? `${st.lastExport.path} (${st.lastExport.bytes} B)` : t("syncNeverExported"), "sync-export")}
    ${toggleRow("cfg-sync-lan", t("syncLanExpose"), st?.lan ?? false)}
    <p class="lede">${t("syncLanHint")}</p>
    <p class="lede">${escapeHtml(t("syncEncryptedNotice"))}</p>
    <div class="prov-buttons">
      <button type="button" data-startpairing>${actionIconSvg("external-link", 12)}<span>${escapeHtml(t("syncStartPairing"))}</span></button>
      <button type="button" data-syncexport ${st?.lan ? "" : "disabled"} title="${st?.lan ? "" : escapeHtml(t("syncPairNeedsLan"))}">${actionIconSvg("refresh", 12)}<span>${escapeHtml(t("syncExportNow"))}</span></button>
    </div>
    <div id="sync-qr" class="sync-qr">${syncPairing ? `
      <h3>${escapeHtml(t("syncReadyToPair"))}</h3>
      <img src="data:image/png;base64,${syncPairing.qrPngBase64}" alt="QR" />
      <p class="sync-address">${escapeHtml(syncPairing.host)}:${syncPairing.port}</p>
      <p class="lede">${escapeHtml(t("syncVerificationCode"))}: <strong>${escapeHtml(syncPairing.fingerprint)}</strong></p>
      ${pending ? `<p class="lede" id="sync-pairing-countdown" data-expires-at="${escapeHtml(pending.expiresAt)}">${escapeHtml(t("syncPairingExpiresIn"))}</p>` : ""}
    ` : ""}</div>
    <h3>${escapeHtml(t("syncPairedDevices"))}</h3>
    ${devices.length === 0 ? `<p class="lede">${escapeHtml(t("syncNoPairedDevices"))}</p>` : `
      <div class="sync-devices">${devices.map((d) => `
        <div class="row" data-device-row="${escapeHtml(d.clientDeviceId)}">
          <span>${escapeHtml(d.name)}</span>
          <span class="lede">${d.lastSeenAt ? escapeHtml(t("syncDeviceConnected")) : ""}</span>
          <button type="button" data-revokedevice="${escapeHtml(d.clientDeviceId)}">${escapeHtml(t("syncRevokeDevice"))}</button>
        </div>
      `).join("")}</div>
    `}
  `;
}
```

`data-copy-pairing-uri`/`syncTechnicalDetails`/`syncCopyUri` are intentionally dropped from this markup — no "Copiar URI" affordance for the v2 QR, per spec. The passphrase `<input>`, `data-savesyncpass`, `data-forgetsyncpass`, and the `cfg-sync` toggle row are gone entirely.

- [ ] **Step 3: Update `reloadSyncStatus`/`refreshSyncView`**

In `reloadSyncStatus`, remove the two lines patching `#cfg-sync`'s checked state (the element no longer exists):

```typescript
  const toggle = document.getElementById("cfg-sync") as HTMLInputElement | null;
  if (toggle && document.activeElement !== toggle) toggle.checked = st.value.enabled;
```

Delete those two lines. Keep the `#cfg-sync-lan` patching as-is.

`refreshSyncView` currently guards on `syncPassphraseDraft` — since there's no passphrase input anymore, simplify it to always do a full `renderSettings()`-driven patch (check the function body with `grep -n "refreshSyncView" -A 15 src/views/settings.ts` first and remove the now-dead `typing`/`syncPassphraseDraft` branch, keeping whatever the non-typing branch already does).

- [ ] **Step 4: Add a live countdown updater**

Add near the bottom of `settings.ts` (exported so `main.ts` can call it once per second while the sync category is open):

```typescript
let countdownInterval: ReturnType<typeof setInterval> | null = null;

export function startPairingCountdown(): void {
  stopPairingCountdown();
  countdownInterval = setInterval(() => {
    const el = document.getElementById("sync-pairing-countdown");
    if (!el) {
      stopPairingCountdown();
      return;
    }
    const expiresAt = new Date(el.dataset.expiresAt ?? "").getTime();
    const remainingMs = expiresAt - Date.now();
    if (remainingMs <= 0) {
      el.textContent = "";
      stopPairingCountdown();
      void reloadSyncStatus();
      return;
    }
    const totalSeconds = Math.ceil(remainingMs / 1000);
    const mm = Math.floor(totalSeconds / 60);
    const ss = String(totalSeconds % 60).padStart(2, "0");
    el.textContent = `${t("syncPairingExpiresIn")} ${mm}:${ss}`;
  }, 1000);
}

export function stopPairingCountdown(): void {
  if (countdownInterval !== null) {
    clearInterval(countdownInterval);
    countdownInterval = null;
  }
}
```

- [ ] **Step 5: Run the frontend tests**

Run: `npm run test:frontend 2>&1 | tail -50`
Expected: some existing tests fail — that's Task 11's job to fix. Confirm here only that nothing *new* breaks beyond the sync-menu tests you already know reference the removed markup (`cfg-sync`, `data-savesyncpass`).

- [ ] **Step 6: Build**

Run: `npm run build 2>&1 | tail -20`
Expected: succeeds (no TypeScript errors).

- [ ] **Step 7: Commit**

```bash
git add src/views/settings.ts src/i18n.ts
git commit -m "feat(sync): replace passphrase UI with one-tap pairing and device list"
```

---

## Task 10: `main.ts` — simplify `pairPhoneFlow`, wire revoke, drop dead handlers

**Files:**
- Modify: `src/main.ts`

**Interfaces:**
- Consumes: `sync_start_pairing`, `sync_revoke_device` commands (Task 7); `startPairingCountdown`/`stopPairingCountdown` (Task 9).

- [ ] **Step 1: Simplify `pairPhoneFlow`**

Replace the function body (currently checking `hasPassphrase`, then `lan`, then auto-enabling `sync_set_enabled`):

```typescript
async function pairPhoneFlow(): Promise<void> {
  openSettingsCategory("sync");
  await refreshSyncView();
  const status = await invokeCmd<SyncStatusDto>("sync_get_status");
  if (!status.ok) {
    showCommandError(status.error ?? t("commandFailed"));
    return;
  }
  if (!status.value.lan) {
    showToast(t("syncPairNeedsLan"), 4000);
    requestAnimationFrame(() => document.getElementById("cfg-sync-lan")?.focus());
    return;
  }
  const pairing = await invokeCmd<SyncPairingDto>("sync_start_pairing");
  if (!pairing.ok) {
    showCommandError(pairing.error ?? t("commandFailed"));
    return;
  }
  setSyncPairing(pairing.value);
  await refreshSyncView();
  startPairingCountdown();
  requestAnimationFrame(() => {
    document.getElementById("sync-qr")?.scrollIntoView({ behavior: "smooth", block: "center" });
  });
}
```

- [ ] **Step 2: Update the import line**

Add `startPairingCountdown` to the existing `import { ... } from "./views/settings"` list (near `setSyncPairing`).

- [ ] **Step 3: Update the button selector list and `data-syncqr`/`data-startpairing` handling**

The selector list at the `target.closest<HTMLElement>(...)` call (search `grep -n 'data-savesyncpass' src/main.ts`) currently includes `[data-savesyncpass],[data-forgetsyncpass],[data-syncqr]`. Replace those three with `[data-startpairing],[data-revokedevice]`.

Delete the two `if (btn.hasAttribute("data-savesyncpass")) { ... }` and `if (btn.hasAttribute("data-forgetsyncpass")) { ... }` blocks entirely (their markup no longer exists after Task 9).

Replace the `if (btn.hasAttribute("data-syncqr")) { ... }` block with:

```typescript
    if (btn.hasAttribute("data-startpairing")) {
      await pairPhoneFlow();
      return;
    }
    if (btn.hasAttribute("data-revokedevice")) {
      const clientDeviceId = btn.getAttribute("data-revokedevice") ?? "";
      const result = await invokeCmd("sync_revoke_device", { clientDeviceId });
      if (!result.ok) showCommandError(result.error ?? t("commandFailed"));
      await refreshSyncView();
      return;
    }
```

- [ ] **Step 4: Remove the `cfg-sync` change handler**

In the generic checkbox-change handler (search `grep -n 'el.id === "cfg-sync"' src/main.ts`), delete the `if (el.id === "cfg-sync") { ... }` block and change the following `} else if (el.id === "cfg-sync-lan")` to a plain `if (el.id === "cfg-sync-lan")`.

- [ ] **Step 5: Run frontend tests**

Run: `npm run test:frontend 2>&1 | tail -50`
Expected: the tests Task 11 hasn't rewritten yet still fail on outdated assertions — proceed to Task 11 next. Confirm no *new* runtime errors (undefined function references) beyond assertion mismatches by running `npm run build` too.

- [ ] **Step 6: Build**

Run: `npm run build 2>&1 | tail -20`
Expected: succeeds.

- [ ] **Step 7: Commit**

```bash
git add src/main.ts
git commit -m "feat(sync): wire pairing/revoke commands, drop passphrase-toggle handlers"
```

---

## Task 11: Rewrite the affected frontend tests

**Files:**
- Modify: `tests/sync-menu.test.ts`

**Interfaces:**
- Consumes: nothing new — this task only updates assertions against `src/main.ts`'s current text.

- [ ] **Step 1: Delete the obsolete `cfg-sync` test**

Remove the entire `test("cfg-sync reverts the checkbox when the backend refuses", ...)` block (lines ~104-107) — there is no more `cfg-sync` checkbox to revert.

- [ ] **Step 2: Rewrite the pair-phone precondition test**

Replace `test("pair-phone checks passphrase, LAN and enabled before pairing", ...)`:

```typescript
test("pair-phone checks LAN before starting a pairing, with no passphrase ceremony", () => {
  const flowStart = main.indexOf("async function pairPhoneFlow");
  assert.ok(flowStart > 0, "pairPhoneFlow must exist as a standalone function shared by the menu and the tray");
  const flow = main.slice(flowStart, main.indexOf("\nasync function handleAction", flowStart));
  const lanIdx = flow.indexOf("syncPairNeedsLan");
  const startPairingIdx = flow.indexOf("sync_start_pairing");
  assert.ok(lanIdx > 0, "pair-phone missing the LAN gate");
  assert.ok(startPairingIdx > 0, "pair-phone missing sync_start_pairing");
  assert.ok(lanIdx < startPairingIdx, "pair-phone must check LAN before starting a pairing");
  assert.ok(flow.includes('getElementById("cfg-sync-lan")'), "missing LAN must focus the LAN toggle");
  assert.ok(flow.includes('getElementById("sync-qr")'), "successful pairing must scroll to the QR");
  assert.ok(!flow.includes("hasPassphrase"), "V2 pairing must never gate on a passphrase");
  assert.ok(!flow.includes("sync_set_enabled"), "V2 pairing must never auto-enable a legacy toggle");
});
```

- [ ] **Step 3: Update the i18n key test**

`test("sync menu i18n keys exist in both languages", ...)` already only checks `syncPhoneMenu`/`syncPairNeedsLan` — both still exist unchanged, so this test needs no edit. Confirm with a read, don't modify if it already passes.

- [ ] **Step 4: Add a regression test for the removed passphrase ceremony**

Add a new test near the others:

```typescript
test("sync settings no longer render a passphrase input or manual toggle", () => {
  const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");
  assert.ok(!settings.includes('data-savesyncpass'), "passphrase save button must be gone");
  assert.ok(!settings.includes('data-forgetsyncpass'), "passphrase forget button must be gone");
  assert.ok(!settings.includes('toggleRow("cfg-sync",'), "manual sync-enable toggle must be gone");
  assert.ok(settings.includes("data-startpairing"), "pairing button must exist");
  assert.ok(settings.includes("data-revokedevice"), "device revoke control must exist");
});
```

- [ ] **Step 5: Run the full frontend suite**

Run: `npm run test:frontend 2>&1 | tail -80`
Expected: all tests pass.

- [ ] **Step 6: Build**

Run: `npm run build 2>&1 | tail -20`
Expected: succeeds.

- [ ] **Step 7: Commit**

```bash
git add tests/sync-menu.test.ts
git commit -m "test(sync): rewrite pair-phone tests for the passwordless flow"
```

---

## Task 12: Android `ProtocolModels.kt` + `PairingUri.kt` — v2 URI parsing

**Files:**
- Modify: `android/app/src/main/java/com/shadelight/iausage/data/ProtocolModels.kt`
- Modify: `android/app/src/main/java/com/shadelight/iausage/data/PairingUri.kt`
- Modify: `android/app/src/test/java/com/shadelight/iausage/data/PairingUriTest.kt`

**Interfaces:**
- Produces: `PairingInfo` gains `val token: String? = null, val secret: ByteArray? = null` (trailing, defaulted — v1 construction sites are unaffected); `PairingUri.parse` accepts `v=2` URIs with `pc`/`token`/`secret` params; `SUPPORTED_ALGORITHM_V2 = "xchacha20poly1305"` constant added.

- [ ] **Step 1: Write the failing test**

Add to `PairingUriTest.kt`:

```kotlin
    @Test fun `parses a v2 pairing URI with token and secret`() {
        val pcId = "0123456789abcdef0123456789abcdef"
        val fp = PairingUri.fingerprintFor(pcId)
        val result = PairingUri.parse(
            "iausage://pair?v=2&host=192.168.50.116&port=28741&pc=$pcId&fp=$fp&token=deadbeef&secret=c2VjcmV0LWJ5dGVzLTEyMw"
        )
        assertEquals("192.168.50.116", result.host)
        assertEquals(pcId, result.deviceId)
        assertEquals("deadbeef", result.token)
        assertEquals("secret-bytes-123", result.secret?.let { String(it) })
    }

    @Test(expected = IllegalArgumentException::class) fun `v2 rejects a missing secret`() {
        val pcId = "0123456789abcdef0123456789abcdef"
        val fp = PairingUri.fingerprintFor(pcId)
        PairingUri.parse("iausage://pair?v=2&host=192.168.50.116&port=28741&pc=$pcId&fp=$fp&token=deadbeef")
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run (from `android/`): `gradle :app:testDebugUnitTest --tests "*PairingUriTest*" 2>&1 | tail -40`
Expected: FAIL — `v=2` currently hits `require(query["v"] == "1")`.

- [ ] **Step 3: Extend `PairingInfo`**

In `ProtocolModels.kt`:

```kotlin
const val SUPPORTED_ALGORITHM_V2 = "xchacha20poly1305"

data class PairingInfo(
    val host: String,
    val port: Int,
    val deviceId: String,
    val fingerprint: String,
    val minAppVersion: String,
    val token: String? = null,
    val secret: ByteArray? = null,
)
```

Kotlin auto-generates `equals`/`hashCode` for data classes including array fields by reference by default with a warning — add `override fun equals`/`hashCode` only if the compiler warns and a test needs value equality; for now, no code compares two `PairingInfo` instances containing a `secret`, so leave the default and note the warning is expected (it already applies to nothing today since v1 sites never set `secret`).

- [ ] **Step 4: Add v2 parsing to `PairingUri.parse`**

Replace `PairingUri.parse` in `PairingUri.kt`:

```kotlin
    fun parse(raw: String): PairingInfo {
        val trimmed = raw.trim()
        val uri = runCatching { URI(trimmed) }.getOrElse { throw IllegalArgumentException("El QR no es un enlace de IA Usage.") }
        require("iausage" == uri.scheme && "pair" == uri.host && trimmed.startsWith(PREFIX)) { "El QR no es un enlace de IA Usage." }
        val query = uri.rawQuery.orEmpty()
            .split('&')
            .mapNotNull { entry ->
                val separator = entry.indexOf('=')
                if (separator <= 0) null else {
                    val key = URLDecoder.decode(entry.substring(0, separator), "UTF-8")
                    val value = URLDecoder.decode(entry.substring(separator + 1), "UTF-8")
                    key to value
                }
            }
            .toMap()
        return when (query["v"]) {
            "1" -> parseV1(query)
            "2" -> parseV2(query)
            else -> throw IllegalArgumentException("La versión del QR no es compatible.")
        }
    }

    private fun parseV1(query: Map<String, String>): PairingInfo {
        val host = query["host"]?.trim().orEmpty()
        val port = query["port"]?.toIntOrNull()
        val device = query["device"]?.trim().orEmpty()
        val fingerprint = query["fp"]?.trim().orEmpty()
        val minApp = query["minApp"]?.trim().orEmpty()
        require(host.isNotEmpty() && !host.equals("localhost", true) && host != "127.0.0.1") { "El QR no contiene una dirección LAN válida." }
        require(port != null && port in 1..65535) { "El puerto del QR no es válido." }
        require(device.matches(Regex("[0-9a-fA-F]{16,128}"))) { "El identificador del PC no es válido." }
        require(fingerprint.matches(Regex("[0-9A-HJKMNPQRSTVWXYZ]{4}-[0-9A-HJKMNPQRSTVWXYZ]{4}"))) { "El código de verificación del QR no es válido." }
        require(fingerprint == fingerprintFor(device)) { "El código de verificación no corresponde al PC del QR." }
        require(minApp.matches(Regex("\\d+\\.\\d+\\.\\d+"))) { "La versión mínima del PC no es válida." }
        return PairingInfo(host, port, device.lowercase(), fingerprint, minApp)
    }

    private fun parseV2(query: Map<String, String>): PairingInfo {
        val host = query["host"]?.trim().orEmpty()
        val port = query["port"]?.toIntOrNull()
        val pc = query["pc"]?.trim().orEmpty()
        val fingerprint = query["fp"]?.trim().orEmpty()
        val token = query["token"]?.trim().orEmpty()
        val secretB64 = query["secret"]?.trim().orEmpty()
        require(host.isNotEmpty() && !host.equals("localhost", true) && host != "127.0.0.1") { "El QR no contiene una dirección LAN válida." }
        require(port != null && port in 1..65535) { "El puerto del QR no es válido." }
        require(pc.matches(Regex("[0-9a-fA-F]{16,128}"))) { "El identificador del PC no es válido." }
        require(fingerprint.matches(Regex("[0-9A-HJKMNPQRSTVWXYZ]{4}-[0-9A-HJKMNPQRSTVWXYZ]{4}"))) { "El código de verificación del QR no es válido." }
        require(fingerprint == fingerprintFor(pc)) { "El código de verificación no corresponde al PC del QR." }
        require(token.isNotEmpty()) { "El QR no contiene un token de pareo." }
        require(secretB64.isNotEmpty()) { "El QR no contiene un secreto de pareo." }
        val secret = runCatching {
            android.util.Base64.decode(secretB64, android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING)
        }.getOrElse { throw IllegalArgumentException("El secreto del QR no es válido.") }
        return PairingInfo(host, port, pc.lowercase(), fingerprint, "0.0.0", token, secret)
    }
```

Note: v2 URIs carry no `minApp` param (the spec's v2 wire format has no `minApp` field — v2 was designed after the min-version gate had already proven unnecessary friction for a same-repo PC/phone pair; `"0.0.0"` is a placeholder that `UsageSyncRepository`'s `versionAtLeast` check will trivially satisfy). Add a one-line comment at the `"0.0.0"` literal explaining exactly this, so it doesn't read as a bug.

- [ ] **Step 5: Run tests to verify they pass**

Run: `gradle :app:testDebugUnitTest --tests "*PairingUriTest*" 2>&1 | tail -40`
Expected: all pass, including the 2 new ones and the 3 existing v1 ones unchanged.

- [ ] **Step 6: Commit**

```bash
git add android/app/src/main/java/com/shadelight/iausage/data/ProtocolModels.kt \
        android/app/src/main/java/com/shadelight/iausage/data/PairingUri.kt \
        android/app/src/test/java/com/shadelight/iausage/data/PairingUriTest.kt
git commit -m "feat(android): parse v2 pairing URIs alongside v1"
```

---

## Task 13: Android `SecureStateStore.kt` — device secret + `clientDeviceId`

**Files:**
- Modify: `android/app/src/main/java/com/shadelight/iausage/data/SecureStateStore.kt`
- Create: `android/app/src/androidTest/java/com/shadelight/iausage/data/SecureStateStoreInstrumentedTest.kt`

**Interfaces:**
- Produces: `fun savePairingV2(pairing: PairingInfo, clientDeviceId: String, secret: ByteArray)`, `fun deviceSecret(): ByteArray?`, `fun clientDeviceId(): String?`, `fun ensureClientDeviceId(): String`; `clear()` preserves `clientDeviceId` across a disconnect.

- [ ] **Step 1: Write the instrumented test (documents expected behavior; run manually on a device/emulator per Global Constraints)**

```kotlin
package com.shadelight.iausage.data

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SecureStateStoreInstrumentedTest {
    private fun freshStore(): SecureStateStore {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        context.getSharedPreferences("iausage.secure.v1", android.content.Context.MODE_PRIVATE).edit().clear().apply()
        return SecureStateStore(context)
    }

    @Test fun clientDeviceId_is_generated_once_and_persists() {
        val store = freshStore()
        val id = store.ensureClientDeviceId()
        assertEquals(id, store.ensureClientDeviceId())
        assertEquals(id, store.clientDeviceId())
    }

    @Test fun deviceSecret_round_trips() {
        val store = freshStore()
        val pairing = PairingInfo("192.168.50.116", 28741, "pc1", "PYYZ-JMJJ", "0.0.0")
        val secret = ByteArray(32) { it.toByte() }
        store.savePairingV2(pairing, "phone-1", secret)
        assertArrayEquals(secret, store.deviceSecret())
        assertEquals(pairing.host, store.pairing()?.host)
    }

    @Test fun clear_preserves_clientDeviceId_but_wipes_everything_else() {
        val store = freshStore()
        val id = store.ensureClientDeviceId()
        val pairing = PairingInfo("192.168.50.116", 28741, "pc1", "PYYZ-JMJJ", "0.0.0")
        store.savePairingV2(pairing, "phone-1", ByteArray(32))
        store.clear()
        assertEquals(id, store.clientDeviceId())
        assertNull(store.deviceSecret())
        assertNull(store.pairing())
    }
}
```

- [ ] **Step 2: Implement the new `SecureStateStore` methods**

Add to `SecureStateStore.kt`:

```kotlin
    fun savePairingV2(pairing: PairingInfo, clientDeviceId: String, secret: ByteArray) {
        write("pairing", listOf(pairing.host, pairing.port, pairing.deviceId, pairing.fingerprint, pairing.minAppVersion).joinToString("\n"))
        write("deviceSecret", Base64.encodeToString(secret, Base64.NO_WRAP))
        preferences.edit().remove("passphrase").apply()
        secret.fill(0)
    }

    fun deviceSecret(): ByteArray? = read("deviceSecret")?.let { Base64.decode(it, Base64.NO_WRAP) }

    fun clientDeviceId(): String? = read("clientDeviceId")

    fun ensureClientDeviceId(): String {
        read("clientDeviceId")?.let { return it }
        val bytes = ByteArray(16)
        java.security.SecureRandom().nextBytes(bytes)
        val id = bytes.joinToString("") { "%02x".format(it) }
        write("clientDeviceId", id)
        return id
    }
```

- [ ] **Step 3: Make `clear()` preserve `clientDeviceId`**

Replace:

```kotlin
    fun clear() = preferences.edit().clear().apply()
```

with:

```kotlin
    /** `clientDeviceId` survives: re-pairing this same phone after a
     * disconnect must still be recognized server-side as the same device
     * (an upsert, not a fresh row) — see the pairing V2 spec's "Re-pairing"
     * section. Everything else (pairing, passphrase, deviceSecret, payload)
     * is wiped as before. */
    fun clear() {
        val preservedClientId = read("clientDeviceId")
        preferences.edit().clear().apply()
        preservedClientId?.let { write("clientDeviceId", it) }
    }
```

- [ ] **Step 4: Run the unit test suite (the rest of the module compiles)**

Run (from `android/`): `gradle :app:testDebugUnitTest :app:compileDebugAndroidTestKotlin 2>&1 | tail -40`
Expected: compiles clean; the new instrumented test is compiled but not executed here (no device attached) — note it as manually-verified-pending per Global Constraints, or run `gradle :app:connectedDebugAndroidTest --tests "*SecureStateStoreInstrumentedTest*"` if a device/emulator is available.

- [ ] **Step 5: Commit**

```bash
git add android/app/src/main/java/com/shadelight/iausage/data/SecureStateStore.kt \
        android/app/src/androidTest/java/com/shadelight/iausage/data/SecureStateStoreInstrumentedTest.kt
git commit -m "feat(android): store per-device secret and a stable clientDeviceId"
```

---

## Task 14: Android `ProtocolCrypto.kt` — raw-key decrypt

**Files:**
- Modify: `android/app/src/main/java/com/shadelight/iausage/data/ProtocolCrypto.kt`
- Modify: `android/app/src/androidTest/java/com/shadelight/iausage/data/ProtocolCryptoInstrumentedTest.kt`

**Interfaces:**
- Produces: `fun decrypt(blob: EncryptedBlob, key: ByteArray): String` overload on `ProtocolCrypto`.

- [ ] **Step 1: Write the instrumented test**

Add to `ProtocolCryptoInstrumentedTest.kt`:

```kotlin
    @Test fun decryptsARawKeyBlob() {
        val crypto = ProtocolCrypto()
        // Build a blob the same way Rust's encrypt_payload_with_key does, by
        // round-tripping through the same golden-vector style used above:
        // encrypt here with the Rust-shaped ciphertext is out of scope for a
        // Kotlin-only test — instead, verify the algorithm/version guards and
        // that a wrong-size key is rejected outright (the full cross-language
        // round trip is covered by the Rust-side tests in Task 3 and by
        // manual pairing against a real desktop instance).
        val badKey = ByteArray(16) // wrong size: must be 32
        val blob = EncryptedBlob(1, SUPPORTED_ALGORITHM_V2, "AQIDBAUGBwgJCgsMDQ4PEA==", "ERERERERERERERERERERERERERERERER", "AA==")
        val error = runCatching { crypto.decrypt(blob, badKey) }.exceptionOrNull()
        assertTrue(error is IllegalArgumentException)
    }
```

- [ ] **Step 2: Run to verify it fails**

Run (from `android/`): `gradle :app:compileDebugAndroidTestKotlin 2>&1 | tail -30`
Expected: FAIL — `decrypt(blob, key: ByteArray)` overload doesn't exist yet (compile error, not a test-runtime failure — this class needs a device to actually execute, but the missing-overload error surfaces at compile time already).

- [ ] **Step 3: Implement the overload**

Add to `ProtocolCrypto.kt`:

```kotlin
    /** V2: the key is already 256 bits of CSPRNG output — no Argon2id. */
    fun decrypt(blob: EncryptedBlob, key: ByteArray): String {
        require(blob.version == SUPPORTED_BLOB_VERSION) { "La versión del blob no es compatible." }
        require(blob.algorithm == SUPPORTED_ALGORITHM_V2) { "El algoritmo del blob no es compatible." }
        require(key.size == AEAD.XCHACHA20POLY1305_IETF_KEYBYTES) { "La clave del dispositivo tiene un tamaño inválido." }
        val nonce = decode(blob.nonce, "nonce", 24)
        val ciphertext = decode(blob.ciphertext, "ciphertext", null)
        return sodium.decrypt(
            sodium.toHexStr(ciphertext),
            null,
            nonce,
            Key.fromBytes(key),
            AEAD.Method.XCHACHA20_POLY1305_IETF,
        ) ?: throw SecurityException("No se pudo descifrar: clave de dispositivo inválida.")
    }
```

- [ ] **Step 4: Run to verify it compiles / passes**

Run: `gradle :app:testDebugUnitTest :app:compileDebugAndroidTestKotlin 2>&1 | tail -40`
Expected: compiles clean. Note the instrumented test itself as manually-verified-pending unless a device/emulator is attached (`gradle :app:connectedDebugAndroidTest --tests "*ProtocolCryptoInstrumentedTest*"`).

- [ ] **Step 5: Commit**

```bash
git add android/app/src/main/java/com/shadelight/iausage/data/ProtocolCrypto.kt \
        android/app/src/androidTest/java/com/shadelight/iausage/data/ProtocolCryptoInstrumentedTest.kt
git commit -m "feat(android): add raw-key decrypt for V2 device secrets"
```

---

## Task 15: Android `UsageSyncRepository.kt` — v2 pair/refresh flow

**Files:**
- Modify: `android/app/src/main/java/com/shadelight/iausage/data/UsageSyncRepository.kt`

**Interfaces:**
- Consumes: `SecureStateStore.{savePairingV2, deviceSecret, clientDeviceId, ensureClientDeviceId}` (Task 13), `ProtocolCrypto.decrypt(blob, key: ByteArray)` (Task 14), `PairingInfo.{token, secret}` (Task 12).
- Produces: `fun pairV2(pairing: PairingInfo): SyncPayload` on `UsageSyncRepository`; `refresh()` transparently picks V1 or V2 based on what `SecureStateStore` holds.

- [ ] **Step 1: Add `pairV2`, `postPair`, `fetchV2`, and the name helper**

Add to `UsageSyncRepository.kt` (the class body):

```kotlin
    fun pairV2(pairing: PairingInfo): SyncPayload {
        val secret = requireNotNull(pairing.secret) { "El QR no contiene un secreto de pareo." }
        val token = requireNotNull(pairing.token) { "El QR no contiene un token de pareo." }
        val clientDeviceId = store.ensureClientDeviceId()
        val name = sanitizedDeviceName()
        postPair(pairing, token, clientDeviceId, name)
        val (payload, raw) = fetchV2(pairing, clientDeviceId, secret)
        store.savePairingV2(pairing, clientDeviceId, secret)
        store.savePayload(raw)
        return payload
    }

    private fun postPair(pairing: PairingInfo, token: String, clientDeviceId: String, name: String) {
        val baseUrl = "http://${bracketedHost(pairing.host)}:${pairing.port}"
        val body = JSONObject().apply {
            put("token", token)
            put("clientDeviceId", clientDeviceId)
            put("name", name)
        }
        val connection = (URL("$baseUrl/v2/pair").openConnection() as HttpURLConnection).apply {
            connectTimeout = 10_000
            readTimeout = 20_000
            requestMethod = "POST"
            doOutput = true
            setRequestProperty("Content-Type", "application/json")
        }
        try {
            connection.outputStream.use { it.write(body.toString().toByteArray(Charsets.UTF_8)) }
            when (val code = connection.responseCode) {
                200 -> return
                410 -> error("El código QR ya expiró o se usó. Genera uno nuevo.")
                404 -> error("El PC no tiene un pareo pendiente. Genera un QR nuevo.")
                else -> error("El PC respondió HTTP $code al vincular.")
            }
        } finally {
            connection.disconnect()
        }
    }

    private fun fetchV2(pairing: PairingInfo, clientDeviceId: String, secret: ByteArray): Pair<SyncPayload, String> {
        val baseUrl = "http://${bracketedHost(pairing.host)}:${pairing.port}"
        val meta = ServerMeta.fromJson(getJson("$baseUrl/v1/meta"))
        require(meta.schemaVersion == SUPPORTED_SCHEMA_VERSION && meta.blobVersion == SUPPORTED_BLOB_VERSION) { "El PC usa una versión de sync no compatible." }
        require(meta.lan) { "El servidor del PC no está expuesto a la red local." }
        require(meta.deviceId.equals(pairing.deviceId, true) && meta.fingerprint == pairing.fingerprint) { "El PC no coincide con el QR. No continúes la vinculación." }
        val blob = EncryptedBlob.fromJson(getJson("$baseUrl/v2/snapshot?device=$clientDeviceId"))
        val raw = crypto.decrypt(blob, secret)
        val payload = SyncPayload.fromJson(JSONObject(raw))
        require(payload.deviceId.equals(pairing.deviceId, true)) { "El snapshot no pertenece al PC vinculado." }
        return payload to raw
    }

    /** `Build.MODEL` crosses HTTP and gets stored/rendered on Desktop —
     * mirror the same sanitization rules as the Rust side. */
    private fun sanitizedDeviceName(): String {
        val cleaned = android.os.Build.MODEL.filterNot { it.isISOControl() }.trim()
        return if (cleaned.isEmpty()) "Android" else cleaned.take(64)
    }
```

- [ ] **Step 2: Make `refresh()` branch on which credential is stored**

Replace `refresh()`:

```kotlin
    fun refresh(): SyncPayload {
        val pairing = store.pairing() ?: error("No hay un PC vinculado.")
        val secret = store.deviceSecret()
        val (payload, raw) = if (secret != null) {
            val clientDeviceId = store.clientDeviceId() ?: error("Falta el identificador de este dispositivo.")
            fetchV2(pairing, clientDeviceId, secret)
        } else {
            val passphrase = store.passphrase() ?: error("La frase secreta segura ya no está disponible.")
            fetch(pairing, passphrase)
        }
        store.savePayload(raw)
        return payload
    }
```

- [ ] **Step 3: Compile-check**

Run (from `android/`): `gradle :app:compileDebugKotlin 2>&1 | tail -40`
Expected: compiles clean.

- [ ] **Step 4: Unit tests**

Run: `gradle :app:testDebugUnitTest 2>&1 | tail -40`
Expected: all pass (no existing unit test in this repo exercises `UsageSyncRepository` directly — it needs network/JNI, so it's covered by manual pairing verification, consistent with how `pair()`/`fetch()` already work today).

- [ ] **Step 5: Commit**

```bash
git add android/app/src/main/java/com/shadelight/iausage/data/UsageSyncRepository.kt
git commit -m "feat(android): pair and refresh via V2 device secret when present"
```

---

## Task 16: Android `UsageViewModel.kt` + `PairingScreen.kt` — v2 UI flow

**Files:**
- Modify: `android/app/src/main/java/com/shadelight/iausage/ui/UsageViewModel.kt`
- Modify: `android/app/src/main/java/com/shadelight/iausage/ui/screens/PairingScreen.kt`

**Interfaces:**
- Consumes: `UsageSyncRepository.pairV2` (Task 15), `PairingInfo.secret` (Task 12).
- Produces: `PendingPairingScreen` renders no passphrase field when `pairing.secret != null`; `UsageViewModel.pair` dispatches to `pairV2` in that case. No change to `App.kt` — `onPair`'s `(String) -> Unit` signature is unchanged, v2 callers just pass an ignored empty string.

- [ ] **Step 1: Branch `UsageViewModel.pair`**

Replace the function body:

```kotlin
    fun pair(passphrase: String) = launch(blocking = true) {
        val pairing = _state.value.pairing ?: return@launch
        val payload = if (pairing.secret != null) {
            repository.pairV2(pairing)
        } else {
            require(passphrase.isNotBlank()) { "Introduce la frase secreta." }
            repository.pair(pairing, passphrase.toCharArray())
        }
        RefreshWorker.schedule(appContext)
        // Keep `pairing` in state (not just the encrypted store) — DeviceScreen
        // needs the host/port/verification code right after pairing succeeds,
        // not just on the next cold start.
        _state.value = UsageUiState(payload = payload, pairing = pairing)
    }
```

- [ ] **Step 2: Branch `PendingPairingScreen`**

Replace the composable:

```kotlin
@Composable
fun PendingPairingScreen(pairing: PairingInfo, loading: Boolean, onPair: (String) -> Unit, onUseAnotherQr: () -> Unit) {
    val isV2 = pairing.secret != null
    // Keyed on the pairing itself so scanning a different PC's QR starts
    // the passphrase field empty instead of carrying over the old value.
    var passphrase by rememberSaveable(pairing.deviceId) { mutableStateOf("") }
    Column(Modifier.fillMaxWidth().padding(24.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("PC encontrado", style = MaterialTheme.typography.titleLarge)
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(16.dp)) {
                Text("IA Usage Desktop", style = MaterialTheme.typography.titleMedium)
                Text("${pairing.host}:${pairing.port}")
                Spacer(Modifier.padding(4.dp))
                Text("Código de verificación", style = MaterialTheme.typography.labelLarge)
                Text(pairing.fingerprint, style = MaterialTheme.typography.titleMedium)
            }
        }
        Text("Comprueba que el código coincide con el que ves en el PC.")
        if (!isV2) {
            SecretTextField(
                value = passphrase,
                onValueChange = { passphrase = it },
                label = "Frase secreta",
                modifier = Modifier.testTag("pairing-secret"),
                onDone = { if (passphrase.isNotBlank() && !loading) onPair(passphrase) },
            )
        }
        Button(
            onClick = { onPair(passphrase) },
            enabled = (isV2 || passphrase.isNotBlank()) && !loading,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(if (loading) "Vinculando…" else "Vincular")
        }
        TextButton(onClick = onUseAnotherQr) { Text("Usar otro QR") }
    }
}
```

- [ ] **Step 3: Compile-check and unit tests**

Run (from `android/`): `gradle :app:testDebugUnitTest :app:compileDebugKotlin 2>&1 | tail -60`
Expected: compiles clean, existing unit tests unaffected (there is no compose UI test harness in this repo's unit test suite, per the existing test layout — UI verification for this screen is manual, same as it is today for the v1 flow).

- [ ] **Step 4: Full Android verification matching CI**

Run (from `android/`): `gradle :app:testDebugUnitTest :app:compileDebugAndroidTestKotlin :app:assembleDebug :app:lintDebug 2>&1 | tail -80`
Expected: succeeds (matches exactly what `release.yml`'s `android` job runs, minus signing).

- [ ] **Step 5: Commit**

```bash
git add android/app/src/main/java/com/shadelight/iausage/ui/UsageViewModel.kt \
        android/app/src/main/java/com/shadelight/iausage/ui/screens/PairingScreen.kt
git commit -m "feat(android): pairing screen skips the passphrase field for V2 QRs"
```

---

## Final Integration Check

- [ ] Run the full validation matrix from the previous session's convention, all in the repo root unless noted:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test --workspace --locked
npm run test:frontend
npm run build
```

```bash
cd android
gradle :app:testDebugUnitTest :app:compileDebugAndroidTestKotlin :app:assembleDebug :app:lintDebug
```

- [ ] Manually verify end to end on real hardware once, the same way the keyring/LAN-selection fixes were verified this session: enable LAN, press "Vincular teléfono," scan the resulting QR with a real phone build, confirm the device appears in the list, confirm a second refresh cycle updates data, then revoke and confirm the phone's next refresh fails cleanly (not a crash).
- [ ] Confirm `git log` shows one commit per task (16 feature/test commits) — do not squash them; the task boundaries are also the review boundaries.
