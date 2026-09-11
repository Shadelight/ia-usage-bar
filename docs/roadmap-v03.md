# Roadmap v0.3 — De medidor a plataforma

> Estado: en ejecución. Este documento es el contrato de v0.3.
> Filosofía (CodexBar): copiar las **fronteras arquitectónicas**, no las features.

```text
                    ┌──────────────┐
                    │ IA Usage Core │
                    └──────┬───────┘
                           │
       ┌───────────────────┼─────────────────┐
       │                   │                 │
       ▼                   ▼                 ▼
 Windows GUI          iausage CLI      Dashboard API (0.5)
       │                   │                 │
       └───────────────────┼─────────────────┘
                           │
                           ▼
                   ProviderDescriptor
                           │
                  ┌────────┼─────────┐
                  ▼        ▼         ▼
                OAuth     CLI       Web/API
```

## v0.2.0 — Cerrar (no mezclar con 0.3)

- [ ] Provider actions desde `links` (5 campos end-to-end: usage/billing/status/docs/app)
- [ ] Smoke test, instalador/desinstalador NSIS/MSI, release GitHub

## v0.3.0 — Core + CLI (P0)

### 1. `crates/iausage-core` (extracción real, no módulo interno)

Mueve sin cambiar lógica: `model, config, cache, cost, pricing, http, jwt,
paths, logfile, providers`. `src-tauri` queda como thin wrapper Tauri
(`pub use iausage_core::...`, cero cambios en consumidores).
Añade módulos nuevos, todos sin dependencias Tauri y testeables:

| Módulo | Contenido |
|---|---|
| `descriptor` | `ProviderDescriptor`, `ProviderCapabilities`, `FetchStrategyKind`, `UsageSource::Auto`, tabla estática por vendor |
| `health` | `ServiceHealth { Operational, Degraded, Outage, Unknown }` separado de `ConnectionStatus` |
| `model` (+) | `DataConfidence { Exact, Estimated, PercentOnly, Unknown }` + procedencia `source` en quotas/costes (campos `#[serde(default)]` para no romper caché vieja) |
| `pace` | `UsagePace { expected, actual, delta, will_last, exhausted_at }` — % usado vs % esperado por tiempo de ventana |
| `refresh_policy` | `next_interval_secs(actividad, battery_saver, manual)` — 2/5/15/30 min, pura y testeable |
| `guard` | `evaluate(remaining, min) -> GuardVerdict`, exit codes `0/1/64/69` |
| `snapshot_v1` | `DashboardSnapshotV1 { schemaVersion: 1, generatedAt, providers[] }` — contrato estable para GUI/CLI/API |
| `doctor` | checks por provider: credencial, fuentes, red, parse, config |

### 2. Sources con fallback

Cada provider declara `strategies: &[OAuth|Cli|Api|Web|Local]` + `default`.
`Auto` prueba en orden y guarda `active_source`. Ajustes muestra
`OAuth ✓ / CLI ✓ / Web —`. Ej: Claude `OAuth→CLI→Web`, Codex `OAuth→app-server`,
Kimi `API→Web`, Windsurf `Web→Local`.

### 3. `crates/iausage-cli` — `iausage`

```text
iausage usage [id] [--json] [--refresh]
iausage providers [--json]
iausage best [--json]
iausage doctor [--json]
iausage refresh [--provider X]
iausage guard --provider claude --window session --min-remaining 15
iausage config validate
iausage version
```

`--json` emite `DashboardSnapshotV1`. `guard` para orquestación de agentes:

```powershell
iausage guard --provider claude --min-remaining 15
if ($LASTEXITCODE -ne 0) { # mandar el trabajo a Codex }
```

### 4. GUI v0.3

- Conexión vs Servicio en dos filas (`Conexión ● / Servicio ●|!`), link de estado.
- Refresh: `Manual/1/2/5/15/30/Adaptativo` (adaptativo = política por visibilidad + battery saver; sin scan de procesos).
- Pace formal en vez de extrapolación opaca.
- Acciones generadas desde `capabilities+links` (lo no soportado no se renderiza).
- Tipos frontend corregidos: `VendorLinks` 5 campos, `VendorInfo` con `hasCredential/docs/app`.

## v0.3.x — Pulido

Adaptive live, `ServiceHealth` con feeds Statuspage, confidence visible
(`Estimado` vs `Exacto · fuente`), `docs/providers/<slug>.md` + `llms.txt`.

## v0.4 — Historia y costes reales

`Usage history` ≠ `Cost history` ≠ `snapshot`. Cada valor con
`source/currency/confidence/period/observedFrom/observedUntil`. Sin sumar
monedas. `models.dev → caché 24h → fallback` solo para logs locales.
Charts + pace/burn-down.

## v0.5 — Multi-account, hooks, API

`Provider → Account[] → Snapshot` (desde ya `accounts.length = 1` compatible).
Hooks locales (`quota_low/reached/reset/unavailable/recovered/refresh_failed`,
JSON por stdin, sin shell por defecto). `iausage serve` loopback
(`/health /v1/snapshot|providers|best`, `no-store`, token solo `Authorization`).

## Después / 1.0

Plugins sandbox: NO antes de 0.5 (frontera de seguridad seria).
Agent sessions y widgets Windows: baja prioridad.
1.0: CI → tests → NSIS/MSI → Authenticode → SHA256 → Release →
`latest.json`/updater → descarga real → instalación real → update desde anterior.
Firma Windows obligatoria antes de 1.0 (0.2.0 vive sin certificado declarando SmartScreen).

## Reglas de contribución (nuevo provider)

`descriptor + strategies + parser + fixtures + docs/providers/<slug>.md`.
Sin lógica especial desperdigada: si tocas >5 archivos fuera de tu provider,
el diseño está mal.
