# Passwordless Device Pairing (Sync V2)

Date: 2026-09-13
Status: Approved by user (in-chat design brief, written up here)
Owner: Alberth Salazar
Relates to: `2026-09-13` sync/keyring/LAN-selection fixes earlier this session
(this spec builds the pairing UX on top of the now-correct keyring backend,
`lan_ip()` selection, and `ensure_sync_server` bind/rollback lifecycle — none
of those are touched again here except where this spec changes what drives
`ensure_sync_server`'s on/off decision).

## What this covers

Today, pairing a phone requires the user to invent a passphrase, type it on
the PC, save it, retype the exact same passphrase on Android, and only then
scan a QR. That passphrase is also the encryption key (Argon2id → the
XChaCha20-Poly1305 key), so it cannot simply be deleted without breaking
encryption — the ask is to remove it from the *user's* experience while
keeping (in fact strengthening) the cryptography underneath.

This spec replaces manual-passphrase pairing with a one-tap flow: **Vincular
teléfono → scan QR → confirm the code → done.** The key material becomes an
auto-generated 256-bit secret, generated per phone, that the user never sees
or types. It also introduces a real (if minimal) multi-device model: each
paired phone gets its own secret and its own row in a device list, and
revoking one phone never affects another.

**Explicitly NOT in scope:**
- Deleting the V1 (manual passphrase) code path. It stays, untouched, as a
  legacy fallback. A future spec can decide when to remove it.
- The "Exportar ahora" folder-export feature (`ia-sync/<device>.json` for
  Syncthing/OneDrive-style external sync). It keeps using the existing global
  passphrase exactly as it does today. Nothing in this spec changes it.
- Any change to `lan_ip()` selection, the bind/rollback lifecycle work, or
  the keyring backend fix — all already shipped in v0.3.1. This spec only
  changes *what decides* whether the sync server should be running.
- Request-level authentication on `/v2/snapshot` beyond what `/v1/snapshot`
  already has today (i.e., none — security is "only the paired device holds
  the decryption key," not "the endpoint checks a credential"). A future spec
  can add HMAC/bearer auth if LAN-visible enumeration becomes a real concern.
- X25519/ECDH key exchange. Considered and rejected: no forward-secrecy
  requirement justifies the added Android dependency and protocol states for
  syncing a handful of usage percentages on a LAN.

## Threat model / why this is safe

The channel is plain HTTP on the LAN — there is no TLS anywhere in this
system, before or after this spec. That means **nothing generated during
pairing may cross that HTTP channel in a form useful to an eavesdropper**:

- The 256-bit secret travels **only inside the QR code** (an optical, not a
  network, channel). It is never sent in an HTTP request or response body.
- `POST /v2/pair` exists only to let the PC learn the phone's identity/name
  and confirm the token was consumed — its response never contains the
  secret.
- The one-time pairing token is transmitted over HTTP (inside the QR *and*
  echoed back in `POST /v2/pair`), so it is treated as a bearer credential:
  short-lived (2 minutes), single-use, and the PC never stores it in the
  clear — only `SHA-256(token)`, compared in constant time.

## Wire protocol

### Pairing URI (v2)

```
iausage://pair?v=2&host=192.168.50.116&port=28741&pc=<pcDeviceId>&fp=PYYZ-JMJJ&token=<hex32>&secret=<base64url-32B>
```

- `pc` replaces today's `device` param name for the v1 URI — renamed to make
  the PC/phone distinction explicit in code and on the wire (`pcDeviceId` vs
  `clientDeviceId` below). `PairingUri` (Android) and `PairingInfo` (Rust)
  both gain this rename for v2; the v1 parser/struct are untouched.
- `token`: 16 random bytes (128 bits), hex-encoded. CSPRNG (`OsRng`), never
  `Math.random`/`kotlin.random.Random.Default` (not a CSPRNG on all JVMs).
- `secret`: 32 random bytes (256 bits), base64url, no padding.
- `fp` (fingerprint) is computed exactly as today, over `pcDeviceId`.

### `POST /v2/pair`

Request:
```json
{ "token": "<hex32>", "clientDeviceId": "<hex32>", "name": "Galaxy S26" }
```

- `clientDeviceId`: Android generates this once (same shape/derivation
  pattern as the PC's existing `load_or_create_device_id`) and persists it
  for the life of the install. Sent on every `/v2/pair` and `/v2/snapshot`
  call.
- `name`: taken from `android.os.Build.MODEL`, never typed by the user.

Response `200`:
```json
{ "paired": true, "deviceId": "<clientDeviceId echoed back>", "pcDeviceId": "<pc device id>", "fingerprint": "PYYZ-JMJJ" }
```
The `secret` is never present in this response, on success or failure.

Response `410 Gone`: token expired or already consumed.
Response `404 Not Found`: no pairing is currently pending.

Server-side, in order:
1. Reject with `404` if no `PendingPairing` exists at all (nothing to try
   against — this is the "no one pressed Vincular teléfono" case, distinct
   from a pairing that existed and died).
2. Reject with `410` if a `PendingPairing` exists but `expires_at` has
   passed.
3. `SHA-256(request.token)` compared in constant time against
   `pending.token_hash`; mismatch → `410` (same status as expired — a wrong
   token gives no more signal than an expired one).
4. On match: build a `PairedDevice` from `clientDeviceId`/`name`, write
   `pending.secret` to Credential Manager under
   `account = device-secret-<clientDeviceId>`, append the device to the
   config's device list, clear `PendingPairing`, respond `200`.

### `GET /v2/snapshot?device=<clientDeviceId>`

- Device not found in the list, or `revoked == true` → `404` (identical to
  "not found" — do not confirm to an unauthenticated LAN client that a given
  device id was ever paired and later revoked).
- Otherwise: load that device's secret from Credential Manager,
  `encrypt_payload_with_key(current_payload, &secret)`, return the blob,
  update `last_seen_at`.
- `clientDeviceId` is not treated as secret (any LAN client can guess/observe
  it); the actual access control is "do you hold the 256-bit key," same trust
  model `/v1/snapshot` already relies on today.

`/v1/meta`, `/v1/snapshot` (v1/legacy passphrase flow) are unchanged.

## Data model

```rust
// crates/iausage-core — new module, e.g. src/pairing.rs

pub struct PendingPairing {
    token_hash: [u8; 32],   // SHA-256(token); the plaintext token is never stored
    secret: [u8; 32],
    expires_at: Instant,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PairedDevice {
    pub client_device_id: String,
    pub name: String,
    pub created_at: String,   // now_iso()
    pub last_seen_at: Option<String>,
    pub revoked: bool,
}
```

- `PairedDevice` list lives in `AppConfig` (new field, `paired_devices:
  Vec<PairedDevice>`), persisted to `config.toml` exactly like today's
  `providers` map — no secrets in that file.
- Each device's 256-bit secret lives in Credential Manager (same
  `keyring::Entry` abstraction already fixed this session — `service =
  com.alberth.iausagebar`, `account = device-secret-<client_device_id>`).
  Revoking a device deletes that Credential Manager entry immediately (not
  just the `revoked` flag) — the row stays in `config.toml` with
  `revoked = true` so a stale/replayed `/v2/pair` for the same
  `client_device_id` (e.g. a phone that still has the old QR cached) is
  rejected explicitly rather than silently re-created.
- `PendingPairing` is **RAM-only**, lives in `AppState` next to
  `SyncServerState` (never serialized, never logged — the secret must not
  reach `config.toml`, log files, or diagnostics exports).

## Pairing lifecycle

```
"Vincular teléfono" pressed
  → generate token (16B), secret (32B), pc_device_id (existing)
  → PendingPairing { token_hash: SHA256(token), secret, expires_at: now()+2min }
  → ensure server running (see "Server lifecycle" below)
  → build v2 URI + QR, show it with a live "expira en 1:47" countdown

Android scans QR
  → extracts secret locally, never sends it anywhere
  → POST /v2/pair { token, clientDeviceId, name }

PC: token valid + not expired + not already consumed
  → register PairedDevice, store secret in Credential Manager
  → clear PendingPairing (single-use enforced by clearing, not by a separate
    "used" flag)
  → 200

PC: token wrong or expired while a PendingPairing still exists
  → 410, PendingPairing untouched (still waiting for the real phone)
PC: token replayed after that PendingPairing was already cleared
  (successful pair, or superseded by a newer "Vincular teléfono" press)
  → 404, same as "nothing pending" — there's nothing left to compare against

QR expires with 0 devices paired since it was shown
  → PendingPairing cleared by the same expiry check that drives "Server
    lifecycle" below (no separate timer needed)
```

Only one `PendingPairing` at a time: pressing "Vincular teléfono" again while
one is already pending replaces it (old token becomes permanently invalid —
same effect as expiry).

## Server lifecycle (replaces today's `cfg.sync_enabled` gate)

`ensure_sync_server`'s decision to bind/stay-bound changes from "is sync
enabled in config" to:

```
serverNeeded = pending_pairing.is_some() || paired_devices.any(|d| !d.revoked)
```

- Pressing "Vincular teléfono" creates the `PendingPairing` and immediately
  calls `ensure_sync_server` — this is what starts the server the very first
  time, with zero devices paired yet.
- A tick is needed to notice "the pending pairing expired and nothing came of
  it" even with no user action — this reuses the existing background
  refresh-loop thread (already ticking every refresh interval) rather than
  spawning a new timer thread: each tick calls `ensure_sync_server` again,
  which is already idempotent when nothing changed, and additionally now
  checks `pending_pairing.expires_at` and clears it if elapsed before
  recomputing `serverNeeded`.
- Revoking the last non-revoked device flips `serverNeeded` to `false` on
  the next `ensure_sync_server` call (called directly from the revoke
  command, not just on the next tick) → server stops, exactly like today's
  `stop_locked` path.
- `AppConfig.sync_enabled` and `sync_set_enabled`/the "Sincronizar con el
  teléfono" toggle are removed from the desktop UI. `AppConfig.sync_lan`
  (the LAN-vs-loopback exposure choice) stays — pairing only makes sense
  over LAN, but which *interface* to expose is unrelated to this spec and
  unchanged (already fixed this session).

## Cryptography

New, alongside (not replacing) today's passphrase-based functions:

```rust
pub fn encrypt_payload_with_key(payload: &SyncPayload, key: &[u8; 32]) -> Result<EncryptedBlob, String>
pub fn decrypt_payload_with_key(blob: &EncryptedBlob, key: &[u8; 32]) -> Result<SyncPayload, String>
```

Same XChaCha20-Poly1305 as today, same `EncryptedBlob` shape (salt field
becomes unused/omitted for v2 blobs — the key is already high-entropy, no KDF
needed; Argon2id stays reserved for the V1/legacy passphrase path only).
`decode_keyring_secret`/`read_keyring_entry` (this session's keyring fix) are
reused as-is for reading each device's secret — no new credential-storage
code needed on the Rust side.

Android: `ProtocolCrypto` gains `decrypt(blob, key: ByteArray)` (raw AEAD open
via lazysodium, no Argon2/BouncyCastle call) alongside the existing
passphrase `decrypt`. `SecureStateStore` gains `saveDeviceSecret`/
`deviceSecret()` (raw bytes, same AES-GCM-over-Keystore wrapping already
used for `passphrase`/`pairing`) without touching the existing passphrase
methods.

## Desktop UI

Replace (in `syncBody()`/`settings.ts`) the current passphrase input, "Guardar
frase"/"Primero guarda una frase secreta" copy, and the "Sincronizar con el
teléfono" toggle with:

```
Teléfono y sync

Red
Wi-Fi · 192.168.50.116

Los datos se cifran antes de salir de este PC.

[ Vincular teléfono ]

  (while a PendingPairing is live, in place of the button:)
  [QR]
  Código de verificación: PYYZ-JMJJ
  Este código caduca en 1:47   (live countdown, client-side timer against
                                 a server-provided expires_at timestamp)

Dispositivos vinculados
  Galaxy S26          ● Conectado · hace 12 s        [Desvincular]
  (empty state: "Ningún teléfono vinculado todavía.")

[ + Vincular otro dispositivo ]
```

`sync_get_status` gains `pairedDevices: PairedDeviceDto[]` and
`pendingPairing: { fingerprint, expiresAt } | null`; loses `hasPassphrase`
from the UI's decision points (the field can stay on the DTO for the legacy
screen path if one remains — see Migration below — but nothing in the new UI
reads it). New commands: `sync_start_pairing() -> SyncPairing` (creates the
`PendingPairing`, ensures the server, returns the same QR/fingerprint shape
`sync_get_pairing` returns today), `sync_revoke_device(clientDeviceId)`.

## Android

- `PairingUri.parse` gains v2 handling (`v=2`, `pc`/`secret`/`token` fields);
  v1 parsing (`v=1`, `device`) is untouched, same file, a second branch.
- `UsageSyncRepository.pair()` gets a v2 counterpart: extract `secret` from
  the parsed URI locally, `POST /v2/pair` with
  `{token, clientDeviceId, name}`, on `200` store `(pairing, secret)` via the
  new `SecureStateStore` methods, then `refresh()` as today.
- `PairingScreen` loses the passphrase `SecretTextField` for the v2 path:
  shows PC name/host, fingerprint, "Comprueba que coincide con el PC," and a
  single `[Vincular]` button. No text input at all.
- `clientDeviceId` generation/persistence: a new small helper (mirrors the
  PC's `load_or_create_device_id`), stored via `SecureStateStore` (new
  `clientDeviceId()`/`ensureClientDeviceId()`), generated once, reused for
  every future pairing attempt from that install (re-pairing the same phone
  updates `PairedDevice.name`/`last_seen_at` server-side rather than creating
  a second row — `client_device_id` is the natural key).

## Migration (V1 stays, V2 is default for new pairings)

- No data migration needed: V1-paired phones keep working against
  `/v1/meta`+`/v1/snapshot` with their existing passphrase, indefinitely,
  until a future spec removes that path.
- The desktop UI shown above is what *every* user sees from this release
  on — there is no user-facing "V1 mode" toggle. A phone paired under V1
  before this ships keeps syncing (the server still answers `/v1/*`); the
  user cannot go back to the old UI to pair a *second* phone with a manual
  passphrase after upgrading. If they want another phone, they use "Vincular
  teléfono" (v2). This is acceptable because the person running this app
  today is its own single real user/tester — there is no installed base to
  protect beyond that.
- `AppConfig.sync_enabled` becomes unused by the new lifecycle logic but is
  not deleted from the struct this spec — removing a `#[serde]` field is a
  separate, independent cleanup that risks nothing here if left alone, and
  YAGNI says don't do it just because it's now unread.

## Testing

Rust (`iausage-core`):
- Pairing token: 16 random bytes, hashed with SHA-256, never equal to the
  plaintext token stored anywhere reachable from a `Debug`/log format.
- Constant-time hash comparison (a wrong-length or wrong-content token is
  rejected; timing is not asserted in the test, just correctness — this repo
  has no timing-attack test harness and inventing one is out of scope).
- Expired token rejected (410) while `PendingPairing` still exists; a second
  attempt with an already-consumed token rejected (404, since pairing was
  cleared on first success) even if attempted before the original expiry.
- Successful pair: `PairedDevice` appears in config, secret readable back
  from Credential Manager under the right account name, `PendingPairing`
  cleared.
- `encrypt_payload_with_key`/`decrypt_payload_with_key` round-trip; two
  different 256-bit keys cannot decrypt each other's ciphertext.
- `/v2/snapshot` for an unknown or revoked device → 404; for an active
  device → decryptable with that device's own key only.
- `serverNeeded` truth table: 0 devices + no pending → false; pending only →
  true; ≥1 non-revoked device → true; all devices revoked + no pending →
  false.
- V1 tests already in the suite (`meta_expone_identidad_sin_secretos`,
  `snapshot_sirve_blob_cifrado_descifrable`, passphrase round-trip in
  `sync.rs`) keep passing unmodified — proof V1 wasn't touched.

Frontend (`tests/`):
- `sync_start_pairing`/`sync_revoke_device` wiring, QR countdown display,
  device list render (empty state + populated), no passphrase input anywhere
  in the new markup.

Android:
- v2 URI parse (valid, missing fields, wrong version).
- Pairing screen has no text-input node for the v2 flow.
- `clientDeviceId` persists across a simulated process/store recreation.
- No secret/token appears in any `Log.*` call touched by this change (a grep
  test over the diff's files, same spirit as the Rust "no secret in logs"
  check).

## Open implementation details left to the plan

- Exact HTTP status/body for "device tries `/v2/snapshot` while its own pair
  request raced a revoke" — treat as the normal revoked-404 case, no special
  handling needed, but call it out explicitly in the plan so it isn't missed.
- Whether `sync_get_pairing`'s existing QR-PNG generation code
  (`pairing_qr_png`) is reused as-is for v2 URIs — it should be, since it
  just PNG-encodes whatever URI string it's given; confirm during
  implementation rather than assume.
