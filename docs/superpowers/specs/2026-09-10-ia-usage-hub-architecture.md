# IA Usage Bar — Rust Core, Windows Collector, Android Companion

Date: 2026-09-10
Status: Direction agreed — a roadmap, not an implementation spec
Owner: Alberth Salazar
First slice: `2026-09-10-ia-usage-bar-stabilization.md` (Milestone 0)

This document is the target the project evolves toward. It is **not** a plan to
execute in one pass. Each milestone below gets its own design → plan →
implementation cycle. Milestone 0 (stabilization) is already specced separately
and is the foundation everything here builds on.

## Vision

A **local-first** tool that answers, from one place, without opening ten
dashboards:

- how much have I used, how much is left, when does it reset;
- how much am I spending;
- which model / provider am I on;
- which limits are close;
- which provider has the most capacity right now;
- what is my consumption pace, and how much can I still use before the reset;
- which providers are working or erroring;
- which accounts are connected.

The immediate personal use case that drives every design choice: **work on the
Windows PC with Claude / Codex / Gemini / etc., then pull out the Android phone
and in two seconds see what's left, what's about to run out, and when each
limit comes back.**

One Rust core produces one normalized snapshot. Every surface — Windows now,
Android next, macOS/Linux/iOS later — renders that snapshot. Provider logic
exists once, in Rust, and is never reimplemented per platform.

## Platforms and roles

| Platform | Role | Status |
|---|---|---|
| **Windows** | Primary desktop + tray **and the collector** — most integrations depend on local credential files, CLIs, session files, and running processes that only exist on the PC | Priority 1 (Milestone 0) |
| **Android** | Companion app + home-screen widget. Consumes normalized snapshots only. Never runs provider integrations, never receives credentials | Priority 3 (after the sync protocol) |
| macOS / Linux | Desktop + tray, same Rust core. Also potential collectors | Architecture stays open; not built now (no hardware to test) |
| iOS | Companion, like Android | Architecture stays open; later |

Windows is the collector because that is where the credentials live. The phone
is a viewer. The Rust core is the single source of truth.

## Out of scope, permanently

- **Stream Deck** — not a surface, client, or integration. Do not design
  modules, clients, or architecture for it.
- **Mandatory cloud services** — no account required, no server the tool cannot
  run without, no telemetry by default. An **optional, end-to-end-encrypted
  relay** for phone sync is allowed (see Snapshot sync) — the server never sees
  plaintext.
- Reimplementing provider logic in Kotlin / Swift / TypeScript. Every non-Rust
  surface renders snapshots.
- Storing passwords. Showing full tokens. Writing secrets to logs. Sending any
  credential to any companion device.

## Differentiators (what makes this worth building)

1. Local-first, open source, no mandatory cloud.
2. **Windows desktop + tray + Android companion + home-screen widget from a
   single Rust core** — the phone shows real usage without reimplementing a
   single provider.
3. **Pace** — not "Claude is at 70%" but "you are burning 2.1× the sustainable
   rate; at this pace you run out 1h 17m before the reset."
4. **Headroom** — normalized "capacity available" across providers with
   different quota systems, clearly labeled as an operational aid.
5. Aggregate cost across providers.
6. Deep diagnostics per provider (strategies tried, source used, latency,
   confidence).
7. Extensible provider architecture — a new provider is a folder, not 15 edits.
8. Multi-account ready in the core from the start.
9. Efficient shared cache + request deduplication.

Not "we support more providers." Quality and the operational features above.

## Layered architecture

```
                         PROVIDER ENGINE (Rust)
                     per-provider modules: auth sources,
                     ordered fetch strategies, parser, mapper
                                   │
                            ProviderSnapshot
                    (normalized; source, freshness,
                     confidence, errors, schema_version)
                                   │
                         SHARED USAGE CORE (Rust)
                cache (SWR) · request dedup · adaptive refresh
                pace · headroom · spend + pricing · diagnostics
                       provider status · alerts engine
                                   │
                ┌──────────────────┴───────────────────┐
                │                                      │
            WINDOWS                              SYNC LAYER
        Tauri desktop                    encrypted ProviderSnapshot
        tray / dashboard                  (never credentials)
        the COLLECTOR                              │
                                        ┌──────────┴──────────┐
                                        │                     │
                                    ANDROID              FUTURE
                                 companion app       macOS / Linux
                                  + widget            iOS
                              (renders snapshots)  (same core / viewer)
```

A surface never knows how Claude authenticates or how Codex computes weekly
usage. It receives snapshots. A companion device receives only the snapshot,
never a credential.

## Provider engine

### Per-provider module layout

Established in Milestone 0:

```
providers/<name>/
  mod.rs         pub const DESCRIPTOR + impl Provider
  auth.rs        credential discovery (local files, keychain, env, OAuth)
  fetch.rs       one or more fetch strategies
  mapper.rs      raw response -> Vec<Metric>
  tests/         fixtures + assertions
```

Registration = add the module + its `DESCRIPTOR` to the registry list. No
central `match` on provider identity outside `providers/` and `model/vendor.rs`
(enforced by an architecture-guard test).

### Descriptor

Each provider declares stable metadata (see the Milestone 0 doc for the Rust
type): `id`, `slug`, `display_name`, `short`, `website`, `dashboard_url`,
`status_url`, ordered `auth`, `env_key`, `login_hint`, and a `Capabilities`
struct (`quota`, `spend`, `credits`, `model_breakdown`, `local_history`,
`multi_account`). The UI reads capabilities, never a hardcoded provider check.

### Auth sources — priority order

1. Reuse official credentials already on the machine (CLI credential files,
   local app state).
2. OS secret store — Windows Credential Manager / macOS Keychain.
3. CLI auth files.
4. OAuth.
5. Manual API key — only when nothing else is reasonable.

Never store passwords. Never render a full token. Never log a credential file.
Env vars allowed for advanced users.

### Fetch strategies

A provider may have several ways to get data (e.g. Claude: OAuth usage
endpoint, Claude Code credential file, keychain, CLI, local JSONL logs, admin
API). Each strategy declares:

`availability`, `priority`, `timeout`, `auth source`, `freshness`,
`confidence`, `fallback behavior`.

The engine tries the best available strategy and falls back only when safe. The
snapshot records: `sourceUsed`, `attempts`, `errors`, `confidence`,
`updatedAt` — so diagnostics can explain any failure.

### Auto-detection

On first run, detect locally which tools exist (Claude Code, Codex, Gemini CLI,
Antigravity, Cursor, Copilot, OpenRouter config, Kiro, Grok, …). **No network
requests to detect a provider.** Valid local credentials → mark available.
Never enable a provider the user does not use.

## Normalized data model

The seed types live in the Milestone 0 doc (`ProviderSnapshot`, `Metric`,
`MetricKind`, `SnapshotStatus`, `Freshness`, `Confidence`). North-star
extensions:

- **Usage windows:** any number per provider. Claude: 5h, weekly, per-model
  weekly, extra usage. Codex: session, weekly, credits. Z.ai: 5h, weekly, MCP,
  searches. Never presume two bars.
- **Multi-account:** `provider → accounts[] → metrics[]`. The core carries the
  arrays from the start; most providers have one account initially. One
  provider's identity never leaks into another.
- **`MetricKind::Chart`** (`(timestamp, value)[]`) added in the milestone that
  needs history visualization.
- **States** are never collapsed to loading/error. Full set: loading-initial,
  refreshing, fresh, stale, offline, rate-limited, auth-required, unsupported,
  no-data, provider-error. Last valid data is kept when appropriate.
- **Data confidence:** high (official API/CLI), medium (local state / dashboard
  scrape), low (heuristic). Normal UI need not show it; diagnostics always do.
- **"No data" is never `0`.**
- **Transport-ready:** the snapshot carries a `schema_version` and serializes to
  plain JSON with no host-specific types (no absolute paths, no `PathBuf`, no
  credential fields). It is both the frontend payload and the sync payload — see
  Snapshot sync. Changes are additive; a breaking change bumps `schema_version`
  and the companion degrades gracefully on an unknown version.

## Shared core services

### Cache — stale-while-revalidate

On start, show the last known snapshot immediately, then refresh in the
background. Never show an empty UI when prior data exists. The cache is shared:
one provider appearing in tray + dashboard + CLI produces **one** refresh, and
all consumers read the same snapshot.

### Request deduplication

Concurrent identical requests coalesce. N consumers of the same
provider+account = one network call.

### Adaptive refresh engine

Modes: adaptive, 1m, 2m, 5m, 15m, 30m, manual. Adaptive weighs: provider,
proximity to limit, proximity to reset, recent activity, errors, rate limits,
whether an agent is active. Never increases request volume without cause.

### Rate-limit protection

On 429: keep last known value, mark `RateLimited`, honor `Retry-After`, apply
exponential backoff + jitter, stop hammering.

### Pace engine (marquee feature)

Compute per window: `elapsedWindowPercent`, `usagePercent`,
`expectedUsageAtCurrentTime`, `paceDifference`, `projectedUsageAtReset`.
States: under / on / over / critical pace. Example: 70% consumed but only 35%
of the window elapsed → warn that the current rate exhausts quota before reset.

### Headroom

A normalized "useful margin before the next block" metric, shown as an
operational aid with an explanation of what it represents. Never claim
providers with different quota systems are mathematically equivalent.

### "What should I use now?"

An informational panel (not model automation): available capacity per
provider + reset time, sorted. No invented comparisons when data is missing.

### Spend + pricing engine

Periods: today, yesterday, 7d, 30d, billing cycle. Show cost / tokens /
requests when available. Local-logs-only spend is labeled "Estimated from local
activity"; account-wide is labeled "Account usage"; the two are never mixed
without a label. Model breakdown when the provider supports it. Never invent a
cost without valid pricing.

**Pricing is a standalone module**: `(provider, model, input, output,
cached_input, currency, validFrom, source)`, updatable without recompiling the
whole app. Computed history records which pricing table it used.

### Diagnostics

Per provider: authentication state, source used, last update, latency,
freshness, fallback. On failure: every strategy tried and why (`OAuth 401`,
`CLI timed out`, `cached snapshot used`). "Copy diagnostic report" button that
redacts tokens, cookies, API keys, emails (if identity redaction is on), and
private paths.

### Provider status

When an official status page exists, surface active incidents. Never conflate
"the user's request failed" with "the provider has a global outage" — separate
states.

### Alerts engine

Configurable, sane defaults (75/90/95%). Alert types: quota approaching, quota
exhausted, credits low, budget threshold, unusual spend, reset completed,
provider unavailable, rate limited. Dedupe, cooldown, per-provider /
per-metric disable. No spam.

### Offline mode

No connection → use known snapshots, show "Offline · last updated 21m ago",
never replace values with zero.

## Surfaces

### Primary

- **Windows tray / menu bar** — the collector's face. Modes: multi-icon
  (`Claude 72%  Codex 42%`) or merged (`AI 72%`). Click → compact dashboard.
  User picks what shows: icon only / percentage / remaining / reset countdown /
  mini bar / provider+percentage. **Pinned metrics** surface directly in the
  bar.
- **Windows desktop dashboard** — operational first: header (last updated,
  refresh), then providers, each with icon / name / plan / optional account,
  primary metrics as bars with remaining + reset, optional today's spend, then
  pace / source / freshness. No wall of technical data by default.
- **Android companion app** — see below. Priority 3.
- **Android home-screen widget** — see below. Priority feature, not a
  far-future experiment.

### Later / optional

- **CLI** — `aiusage status`, `aiusage provider claude`, `aiusage --json`,
  `aiusage refresh`, `aiusage diagnose claude`. JSON output is a versioned
  contract for scripts, agents, CI. Cheap to add once the core is a library;
  not on the critical path to the personal use case.
- **Local HTTP API** — loopback (`127.0.0.1`), and the transport for sync V1
  (see Snapshot sync). Never exposes credentials. Restrictive CORS. Documented
  probe risk. Fully disableable.
- **macOS / Linux desktop + tray** — same Rust core; needs test hardware.
- **iOS companion** — like Android; later.
- **TUI, IDE / editor extension** — exploratory, far future.

Customization across surfaces: enable/disable and reorder providers and
metrics, hide / always-visible / on-demand / pin, compact vs comfortable
density, theme system/dark/light, relative vs absolute reset times.

## Snapshot sync

The Windows collector produces snapshots; the phone needs them, and the PC may
be asleep or off. This is a distinct architectural layer.

### Hard rule: the phone never receives credentials

```
Claude credentials ──▶ Claude provider ──▶ ProviderSnapshot ──▶ sync ──▶ Android
        (stay on the PC, always)                              (snapshot only)
```

What crosses the sync boundary: `provider`, `plan`, `status`, `updatedAt`,
`metrics[]` (id, label, kind, used/limit/percent, resetsAt, pace, spend basis),
`schema_version`. What never crosses: OAuth/refresh tokens, cookies, API keys,
credential file contents, absolute paths. Enforced by the type that gets
serialized for sync being a distinct `SyncPayload`, not the internal state.

### V1 — local encrypted transport (no backend)

```
Windows collector ──▶ local encrypted API / file ──▶ Android
```

- The collector exposes the snapshot on the loopback HTTP API and/or writes an
  encrypted snapshot blob to a path the user already syncs (Syncthing, a
  cloud-drive folder — the tool does not care which).
- Reachability: same Wi-Fi, or the user's own Tailscale / VPN / WireGuard. No
  server operated by this project.
- Encryption: a symmetric key derived from a passphrase the user sets on both
  devices (Argon2id → key; AES-256-GCM or XChaCha20-Poly1305 for the blob).
- Pairing: show a QR on the desktop containing the endpoint + a pairing secret;
  the phone scans it once.

### V2 — optional end-to-end-encrypted relay

```
Windows ─▶ encrypt locally ─▶ relay (stores ciphertext) ─▶ Android ─▶ decrypt locally
```

- For "check from anywhere" without VPN setup. Opt-in, off by default.
- The relay stores only `{ ciphertext, deviceId, timestamp, schema_version }`.
  It cannot read anything. Keys never leave the devices.
- Keeps the honest framing: **local-first and end-to-end encrypted**, not
  "no byte ever leaves the PC" — those are different claims and the docs say
  which one applies.

### Staleness

The companion always keeps the last valid snapshot and renders it immediately.
If the last sync is old it shows the age and a `STALE` marker — never `0%`,
never a blank. Example: `Claude 22% · Updated 4h ago · STALE`.

## Android companion

Priority 3, right after the sync protocol. **Android runs no provider
integrations.** It consumes normalized snapshots.

### What it shows

- Provider list (All / Claude / Codex / Cursor / …), sorted by remaining
  capacity.
- Provider detail: session, weekly, credits, reset, pace, spend, status, last
  sync.
- The app caches the last valid snapshot locally and works while the PC is
  disconnected (shown as stale).

### Home-screen widget — three sizes

| Widget | Content |
|---|---|
| **2×1 compact** | one provider + % remaining + reset |
| **2×2 provider** | session + weekly + reset + status for one provider |
| **4×2 overview** | 3–5 providers sorted by remaining capacity, + "updated Nm ago" |

```
┌────────────────────────┐      ┌───────────────────┐
│ IA Usage               │      │ CLAUDE            │
│ Claude     18%   1h42m  │      │ Session      82%  │
│ Codex      63%   3d 4h  │      │ ████████░░        │
│ Gemini     87%   2h11m  │      │ 18% remaining     │
│ Updated 2m ago          │      │ Reset in 1h 42m   │
└────────────────────────┘      └───────────────────┘
```

Renders cached data instantly; marks stale data clearly.

### Technology

Native — Tauri is not reused here just for the sake of reuse.

```
Kotlin + Jetpack Compose        app UI
Glance                          home-screen widgets
Room / DataStore                snapshot cache + config
WorkManager                     periodic sync
```

Module shape — client concerns only, no provider names:

```
SnapshotRepository   holds + persists the latest SyncPayload
SyncClient           V1 local / V2 relay transport, decryption, pairing
ProviderScreen       Compose UI over SnapshotRepository
WidgetRepository     feeds Glance widgets from the cache
```

There is no `ClaudeProvider.kt`. The shared contract is the `SyncPayload` JSON
schema (Kotlin data classes generated from or mirrored against the Rust types).

## Host capability interfaces

Providers do not implement filesystem / keychain / http from scratch. The core
supplies: `HTTPClient`, `CredentialStore` (`KeychainStore`,
`FileCredentialStore`, `BrowserSessionStore`), `ProcessRunner`, `CLIProbe`,
`LocalHistoryScanner`, `Logger`, `StatusClient`, `PricingService`. Provider
implementations receive these as dependencies. Minimal globals.

## Security model

- Local-first, no telemetry by default. Credentials never leave the collector
  machine — not to a companion device, not to a relay, never.
- What may leave the machine: a normalized snapshot, and only encrypted. V1
  keeps it on the LAN / the user's own VPN. V2's optional relay stores
  ciphertext only and cannot decrypt it. "Local-first + end-to-end encrypted"
  is the claim; the docs never overstate it as "nothing ever leaves the PC".
- Secrets stay local, redacted in logs, files with restrictive permissions.
- **Per-provider domain allowlist** — a provider cannot request an undeclared
  domain. Mandatory timeout on every request.
- No provider accesses another provider's secrets.
- No shell command is ever built from a remote response.
- All external data validated at the boundary — including the decrypted sync
  payload on the companion side (treat it as untrusted input, validate against
  `schema_version`).
- The `SyncPayload` type is structurally incapable of carrying a credential —
  it has no such field, and the internal state type is never serialized to the
  wire.

## Architecture gatekeeper

Tests that fail the build if a future contributor or agent starts hardcoding
provider knowledge:

- No `if provider == "claude"` / `match VendorId` outside `providers/` and
  `model/vendor.rs`.
- Generic UI depends on descriptor + capabilities only.
- Every registered provider: has a descriptor, unique id, valid metrics, an
  icon, a docs page, tests, no credential leak, a timeout, a domain allowlist.
- A contract-conformance suite every provider must pass.

## ADR — framework choices

### Desktop: keep Tauri 2 + Rust backend + vanilla TypeScript/Vite frontend

- Reuse: the entire provider engine, quota/cost/pace logic, config, and tray
  are already Rust + Tauri 2 and build clean. A framework switch reuses none of
  it.
- Targets: Windows + macOS (Tauri 2 covers both; Linux is reasonable).
- Footprint: Tauri's system-webview keeps RAM/CPU well below Electron for an
  app that is idle almost always.
- Secure filesystem / keychain access: native in Rust.
- Auto-update, simple installers (MSI/NSIS/dmg), signing/notarization: Tauri 2
  supports all.

Electron rejected (footprint, no reuse). Native per-OS shells rejected
(duplicate work, no reuse). Revisit only if a Tauri 2 limitation blocks a
required capability.

### Android: native Kotlin, not Tauri mobile

- The companion is a thin viewer over a JSON snapshot. It needs no webview and
  no Rust on the device — shipping either would be weight for nothing.
- Home-screen widgets require the native widget frameworks (Glance / AppWidget)
  regardless; Tauri mobile does not provide them.
- Stack: Kotlin + Jetpack Compose (app), Glance (widgets), Room/DataStore
  (cache/config), WorkManager (sync). No provider logic — only
  `SnapshotRepository`, `SyncClient`, `ProviderScreen`, `WidgetRepository`.
- The shared contract is the `SyncPayload` schema, not shared code.

### Core: stays Rust

A rewrite of the core to another language needs a concrete, measurable
advantage. There is none — the core is already Rust and every surface is a
renderer of its output.

## Milestone roadmap

Each milestone ships working. No mega-branch. Ordered around the personal use
case (Windows + phone), not around an impressive feature list.

| # | Milestone | Ends with |
|---|---|---|
| **M0** | Windows stabilization (separate doc) | Publishable v0.1.0; `ProviderDescriptor` + normalized, transport-ready snapshot formalized |
| M1 | Fetch strategies + host-capability interfaces | Providers try ordered strategies; snapshot records source/attempts/confidence |
| M2 | Shared cache (SWR) + request dedup + adaptive refresh | One refresh feeds all consumers; adaptive polling |
| M3 | Pace + Headroom | Pace states + capacity panel in the dashboard |
| M4 | Spend / history / pricing engine | Normalized periods, standalone pricing module, labeled basis |
| **M5** | **Snapshot Sync Protocol** | `SyncPayload` type, encryption, pairing (QR), V1 local transport (LAN / user VPN), staleness handling |
| **M6** | **Android companion app** | Kotlin/Compose app renders provider list + detail from synced snapshots; local cache; stale states |
| **M7** | **Android home-screen widget** | Glance widgets in 2×1 / 2×2 / 4×2; instant cached render; stale marker |
| M8 | CLI | Versioned JSON contract: `status` / `provider` / `diagnose` / `refresh` |
| M9 | Local HTTP API | Loopback, disableable, restrictive CORS, documented probe risk |
| M10 | Deep diagnostics + provider status pages | Strategy trace, redacted report export, incident surfacing |
| M11 | Full alerts engine | Budget, unusual spend, reset, unavailable; dedupe + cooldown; optional push to Android |
| M12 | Additional providers | Gemini CLI, Z.ai/GLM, MiniMax, DeepSeek, OpenRouter consolidation, … (quality gated) |
| M13 | Multi-account | Account arrays surfaced in desktop + companion |
| M14 | macOS | Tray + build + signing |
| M15 | Linux | Tray + build + packaging |
| M16 | iOS companion | Same `SyncPayload`, native Swift viewer + widget |
| M17 | TUI / IDE integrations | Exploratory |
| — | Sync V2 (optional E2E relay) | Slotted whenever "check from anywhere without a VPN" becomes worth the infra |

Order may change only if analysis of the repo shows a strong reason.

## Documentation to produce (across milestones)

`ARCHITECTURE.md`, `PROVIDER_CONTRACT.md`, `ADDING_PROVIDER.md`, `SECURITY.md`,
`PRIVACY.md`, `DIAGNOSTICS.md`, `RELEASING.md`, `SYNC.md` (the `SyncPayload`
schema, encryption, pairing, what does and does not cross the boundary), and
`docs/providers/<provider>.md` per provider (what is tracked, authentication,
data source, endpoints, known limitations, error meanings).

**Privacy page** must state plainly: which files are read and why, which
endpoints are contacted, what stays local, where credentials live, whether
telemetry exists, how to disable integrations.

## Working method per milestone

State the problem → inspect related code → propose minimal changes → implement
→ add tests → run lint/typecheck/tests/build → fix → `git diff --check` →
summarize changed files → report remaining risk/debt. Nothing is "done" until
the available checks have run.

Before touching code: `git status`. Never `git reset --hard`, `git clean -fd`,
`git checkout .`; never delete the user's branches or stashes; work around
unrelated modifications.

## Success criterion

```
ClaudeProvider  → ProviderSnapshot
CodexProvider   → ProviderSnapshot   (Rust core, on the Windows PC)
GeminiProvider  → ProviderSnapshot
        │
        ├── Windows tray + dashboard (snapshot)
        └── encrypted SyncPayload ──▶ Android app + widget (snapshot)
```

No surface — desktop or phone — knows how any provider authenticates or
computes usage, and no credential ever reaches the phone. The user works on the
PC, pulls out the phone, and understands in under five seconds: which AI they
can keep using, which limit is close, when quota returns, and how much they are
spending.
