# Sync con el teléfono (M5)

El PC es el colector: ahí viven credenciales, sesiones y CLIs. El teléfono
es un visor: solo recibe el snapshot normalizado y cifrado. Nada de lógica
de providers sale del Rust core.

Alcance M5 (V1 local, sin backend nuestro): envelope cifrado, blob en
carpeta, HTTP local y pareo por QR. Fuera de M5: app Android (M6), widget
Glance (M7), relay con cifrado extremo a extremo (V2), push.

## Formato del cable

```json
{ "v": 1, "alg": "xchacha20poly1305+argon2id",
  "salt": "<b64>", "nonce": "<b64>", "ciphertext": "<b64>" }
```

El `ciphertext` descifra a JSON canónico de `SyncPayload`:

```json
{ "schemaVersion": 1, "deviceId": "<hex>",
  "generatedAt": "<iso8601>",
  "snapshot": { "schemaVersion": 1, "providers": [ ...DashboardSnapshotV1 ] } }
```

- Cifrado: XChaCha20-Poly1305 (nonce aleatorio 24 B). Clave: Argon2id
  (m = 64 MiB, t = 3, p = 4) sobre la passphrase + salt aleatoria 16 B.
- `v` es la versión del envelope; `schemaVersion` la del snapshot. Ambas se
  rechazan si no se conocen. El test `vector_conocido_fija_formato` fija el
  formato byte a byte: cambiar algoritmo, parámetros o JSON canónico rompe
  el build a propósito (hay un teléfono al otro lado).
- El `deviceId` (hex aleatorio, `device-id` en el config dir) identifica al
  colector. No es secreto; no autentica nada.

## Qué cruza y qué no

Cruza: estado, motivo, staleness, cuotas, créditos, coste, `service.url`,
`updatedAt`, `generatedAt`. Nunca cruza: tokens OAuth/refresh, cookies, API
keys, contenido de ficheros de credenciales, rutas absolutas, la passphrase.
Lo garantiza el tipo (`SyncPayload` ≠ estado interno) más el test
`payload_sin_secretos_ni_rutas`, que falla el build si el cable muestra
marcas de secretos o paths.

## Passphrase

Vive en el Credential Manager (cuenta `sync-passphrase`), nunca en
`config.toml`. Sin ella no hay sync: activar exige tenerla guardada.
Rotarla = guardarla de nuevo y reiniciar el servidor (el servidor la carga
al arrancar). Olvidarla apaga el sync: seguir cifrando con una clave que ya
no existe dejaría blobs ilegibles.

## Transportes V1

**V1a — blob en carpeta.** Tras cada refresh completo, si sync está activo,
se escribe `<carpeta>/<device>.json` (defecto `<config>/ia-sync`). Esa
carpeta es la que el usuario ya sincroniza (Syncthing, OneDrive…).

**V1b — HTTP local.** Solo mientras la app corre:

```text
GET /v1/meta      -> { schemaVersion, blobVersion, deviceId,
                       fingerprint, appVersion, lan } (público)
GET /v1/snapshot  -> EncryptedBlob JSON (opaco sin passphrase)
```

Bind `127.0.0.1:28741` por defecto. Exponer en LAN (`0.0.0.0`) es opt-in
explícito en Ajustes → Sync, solo para redes de confianza. El snapshot se
construye y cifra por petición: siempre fresco, ~0,5 s por Argon2id.

## Pareo

Ajustes → Sync → Mostrar QR, o `iausage sync qr [--lan] [--out f.png]`:

```text
iausage://pair?v=1&host=<ip>&port=28741&device=<hex>&fp=XXXX-XXXX&minApp=0.3.0
```

- El QR lleva secreto de alta entropía implícito: la seguridad es la
  passphrase, no el QR. Todo el formato es URL-safe a propósito.
- `fp` es el fingerprint (`XXXX-XXXX`, Crockford, derivado de
  `sha256("iausage-pairing-v1:" + device_id)`): se compara en ambas
  pantallas para detectar MITM en LAN. No es secreto ni clave.
- `minApp` es la versión mínima del colector que entiende ese pareo.
- Sin `--lan` el QR apunta a `127.0.0.1` (inútil para el teléfono; avisa).

## CLI

```powershell
iausage sync set-passphrase      # lee de stdin (vacía = olvidar + apagar)
iausage sync enable | disable
iausage sync status              # estado, device, fingerprint, último export
iausage sync export [--out DIR] [--refresh]
iausage sync verify <archivo>
iausage sync qr [--lan] [--out qr.png]
iausage sync serve [--lan] [--port N] [--refresh]  # foto fija al arrancar
```

Códigos: 0 ok, 64 uso incorrecto, 69 fallo operativo.

## Modelo de amenazas (honesto)

- Sin *forward secrecy*: si se filtran blob + passphrase, hay que rotar la
  passphrase (los blobs viejos quedan legibles). El relay V2 no cambia esto.
- LAN sin cifrado de transporte: el blob ya va cifrado; un atacante de red
  ve metadatos (tamaño, tiempos) pero no el contenido.
- El teléfono valida `schema_version` y trata el payload como entrada no
  confiable (M6).
- PC dormido/apagado = datos stale en el teléfono, siempre marcados con su
  edad, nunca ceros. El servidor muere con la app; el blob en carpeta queda.
