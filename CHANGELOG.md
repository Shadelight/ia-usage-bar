# Changelog

All notable changes to this project are documented here. This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- M5 sync con el teléfono (V1 local, sin backend): `SyncPayload` cifrado
  (Argon2id → XChaCha20-Poly1305), blob en carpeta tras cada refresh,
  HTTP local (`/v1/meta`, `/v1/snapshot`), pareo por QR con fingerprint
  de verificación, y sección Sync en Ajustes. Ver `SYNC.md`.
- CLI `iausage sync`: `export`, `verify`, `status`, `set-passphrase`,
  `enable`/`disable`, `serve`, `qr`.

## [0.2.0] - 2026-09-12

First release under the IA Usage Bar name. `v0.1.0` was the original Claude
Bar fork this project started from; this is the reconstructed multi-provider
monitor.

### Added

- Multi-provider catalog (Claude Code, Codex/ChatGPT, Cursor, Antigravity,
  GitHub Copilot, OpenAI API, OpenRouter, Z.AI, DeepSeek, Grok, Kimi, and more)
  with a shared identity registry, local official icons, and explicit
  connection states instead of a generic connected/error boolean.
- Canonical quota, reset, credit, product-breakdown, and cost models shared
  by every provider adapter.
- Configurable notification thresholds and launch-at-login setting.
- Pin (always-on-top) and compact window modes, both persisted and mirrored
  in the tray menu.
- Header quick-actions menu (compact mode, notifications, spend, provider
  usage/status links, logs folder, settings).
- Categorized Settings screen (General, Providers, Notifications,
  Appearance, Data & logs, About).
- Minimal capped log file and a diagnostics export for bug reports.
- Windows taskbar, minimize, close-to-tray, and single-instance behavior.

- Workspace `crates/iausage-core`: todo el pipeline de providers (modelo,
  config, caché, fetchers, coste) sale de `src-tauri`, que queda como thin
  wrapper de ventana/tray. La GUI y el CLI consumen el mismo pipeline.
- `ProviderDescriptor`: fuente única de metadata (estrategias, capacidades)
  para los 23 vendors. Añadir un provider es descriptor + parser + fixtures
  + `docs/providers/<slug>.md`.
- CLI `iausage`: `usage`, `providers`, `best`, `doctor`, `refresh`, `guard`
  (exit codes 0/1/64/69), `enable`/`disable`, `config validate`, `version`,
  todo con `--json` sobre el contrato estable `DashboardSnapshotV1`
  (`schemaVersion: 1`).
- Selección de fuente por provider (`Automática/OAuth/CLI/API/Web/Local`)
  con fuente activa visible ("Usando ahora").
- Salud del servicio separada de la conexión (dos filas en Detalles).
- `DataConfidence`: los costes de logs locales se etiquetan como
  `Estimado`, nunca como factura.
- Modelo de pace formal (% real vs % esperado, proyección de agotamiento).
- Refresh adaptativo (2/5/15/30 min por actividad + ahorro de batería) con
  política pura y testeable; intervalos manuales 1/2/5/15/30.
- Comando Tauri `get_snapshot_v1` + `set_source_preference`.
- Docs por provider (`docs/providers/`) y `docs/llms.txt`.
### Changed

- Transient provider failures preserve the last valid metrics as stale data.
- API keys entered in Settings are stored in Windows Credential Manager.
- Product identity is now IA Usage Bar; see [NOTICE](NOTICE) for the
  third-party attribution this carries forward from Claude Bar and the other
  MIT-licensed projects it draws on.

- `refresh_minutes` ahora admite 1/2/5/15/30 (el antiguo 10 migra a 15).
- La caché 0.2.0 sigue siendo legible (campos nuevos con default).

[Unreleased]: https://github.com/Shadelight/ia-usage-bar/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.0
