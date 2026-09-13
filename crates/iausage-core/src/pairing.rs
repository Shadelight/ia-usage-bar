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
        assert!(!new_pairing
            .pending
            .matches_token("00000000000000000000000000000000"));
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
}
