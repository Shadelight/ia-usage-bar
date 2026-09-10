# IA Usage Bar → AI Usage Hub — North-Star Architecture

Date: 2026-09-10
Status: Direction agreed — a roadmap, not an implementation spec
Owner: Alberth Salazar
First slice: `2026-09-10-ia-usage-bar-stabilization.md` (Milestone 0)

This document is the target the project evolves toward. It is **not** a plan to
execute in one pass. Each milestone below gets its own design → plan →
implementation cycle. Milestone 0 (stabilization) is already specced separately
and is the foundation everything here builds on.

## Vision

A **local-first** desktop tool that answers, from one place, without opening ten
dashboards:

- how much have I used, how much is left, when does it reset;
- how much am I spending;
- which model / provider am I on;
- which limits are close;
- which provider has the most capacity right now;
- what is my consumption pace, and how much can I still use before the reset;
- which providers are working or erroring;
- which accounts are connected.

One core, many surfaces, each rendering the **same** normalized snapshot.
Provider logic exists once and is never duplicated per surface.

## Out of scope, permanently

- **Stream Deck** — not a surface, client, or integration.
- **Mandatory cloud services** — no account, no server dependency, no telemetry
  by default.
- Storing passwords. Showing full tokens. Writing secrets to logs.

## Differentiators (what makes this worth building)

1. Cross-platform, local-first, open source.
2. Desktop + tray + CLI + local HTTP API from a **single core**.
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
                       ┌──────────────────┐
                       │  Provider Engine  │
                       └────────┬─────────┘
              ┌─────────────────┼──────────────────┐
           Claude             Codex              Cursor        (per-provider modules)
              │                 │                  │
        AuthSources[]      AuthSources[]      AuthSources[]     (ordered by priority)
        FetchStrategies[]  FetchStrategies[]  FetchStrategies[] (ordered, with fallback)
        Parser + Mapper    Parser + Mapper    Parser + Mapper
              └─────────────────┼──────────────────┘
                                │
                        ProviderSnapshot            (normalized; carries source,
                                │                    freshness, confidence, errors)
                       ┌────────┴─────────┐
                       │  Shared Usage Core │
                       │  Cache (SWR)       │
                       │  Request dedup     │
                       │  Adaptive refresh  │
                       │  Pace engine       │
                       │  Headroom          │
                       │  Spend + pricing   │
                       │  Diagnostics       │
                       │  Provider status   │
                       │  Alerts engine     │
                       └────────┬─────────┘
              ┌────────┬────────┼────────┬───────────┐
            Tray    Desktop    CLI    HTTP API      TUI          (surfaces — render only)
                    Dashboard          (loopback)  (optional)
```

A surface never knows how Claude authenticates or how Codex computes weekly
usage. It receives snapshots.

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

- **Tray / menu bar** — Windows now; macOS/Linux later. Modes: multi-icon
  (`Claude 72%  Codex 42%`) or merged (`AI 72%`). Click → compact dashboard.
  User picks what shows: icon only / percentage / remaining / reset countdown /
  mini bar / provider+percentage. **Pinned metrics** can surface directly in
  the bar.
- **Desktop dashboard** — operational first: header (last updated, refresh),
  then providers, each with icon / name / plan / optional account, primary
  metrics as bars with remaining + reset, optional today's spend, then pace /
  source / freshness. No wall of technical data by default.
- **CLI** — `aiusage status`, `aiusage provider claude`, `aiusage --json`,
  `aiusage refresh`, `aiusage providers`, `aiusage diagnose claude`. JSON
  output is a **versioned contract** for scripts, agents, CI.
- **Local HTTP API** — loopback only (`127.0.0.1`). `GET /v1/providers`,
  `GET /v1/usage`, `GET /v1/providers/:id`, `POST /v1/refresh`,
  `GET /v1/health`. Never exposes credentials. Restrictive CORS. Documented
  risk that local processes/sites may probe the port. Fully disableable.
- **TUI** — optional, later.
- **Widgets** — where the OS allows, later.
- **IDE / editor extension** — exploratory, far future.

Customization across surfaces: enable/disable and reorder providers and
metrics, hide / always-visible / on-demand / pin, compact vs comfortable
density, theme system/dark/light, relative vs absolute reset times.

## Host capability interfaces

Providers do not implement filesystem / keychain / http from scratch. The core
supplies: `HTTPClient`, `CredentialStore` (`KeychainStore`,
`FileCredentialStore`, `BrowserSessionStore`), `ProcessRunner`, `CLIProbe`,
`LocalHistoryScanner`, `Logger`, `StatusClient`, `PricingService`. Provider
implementations receive these as dependencies. Minimal globals.

## Security model

- Local-first, no telemetry by default, usage never leaves the machine.
- Secrets stay local, redacted in logs, files with restrictive permissions.
- **Per-provider domain allowlist** — a provider cannot request an undeclared
  domain. Mandatory timeout on every request.
- No provider accesses another provider's secrets.
- No shell command is ever built from a remote response.
- All external data validated at the boundary.

## Architecture gatekeeper

Tests that fail the build if a future contributor or agent starts hardcoding
provider knowledge:

- No `if provider == "claude"` / `match VendorId` outside `providers/` and
  `model/vendor.rs`.
- Generic UI depends on descriptor + capabilities only.
- Every registered provider: has a descriptor, unique id, valid metrics, an
  icon, a docs page, tests, no credential leak, a timeout, a domain allowlist.
- A contract-conformance suite every provider must pass.

## ADR — desktop framework

**Decision: keep Tauri 2 + Rust backend + vanilla TypeScript/Vite frontend.**

- Reuse: the entire provider engine, quota/cost/pace logic, config, and tray
  are already Rust + Tauri 2 and build clean. A framework switch reuses none of
  it.
- Targets: Windows + macOS (Tauri 2 covers both; Linux is reasonable).
- Footprint: Tauri's system-webview keeps RAM/CPU well below Electron for an
  app that is idle almost always.
- Secure filesystem / keychain access: native in Rust.
- Auto-update, simple installers (MSI/NSIS/dmg), signing/notarization: Tauri 2
  supports all.
- A core rewrite to Rust-only-native or Swift needs a concrete, measurable
  advantage. There is none here — the core is already Rust.

Electron rejected (footprint, no reuse). Native per-OS shells rejected
(duplicate work, no reuse). Revisit only if a Tauri 2 limitation blocks a
required capability.

## Milestone roadmap

Each milestone ships working. No mega-branch.

| # | Milestone | Ends with |
|---|---|---|
| **M0** | Stabilization (separate doc) | Publishable v0.1.0, provider contract + normalized snapshot formalized |
| M1 | Fetch strategies + fallback records + host-capability interfaces | Providers try ordered strategies; snapshot records source/attempts/confidence |
| M2 | Shared cache (SWR) + request dedup + adaptive refresh | One refresh feeds all consumers; adaptive polling |
| M3 | Pace + Headroom | Pace states + capacity panel in the dashboard |
| M4 | Spend / history / pricing engine | Normalized periods, standalone pricing module, labeled basis |
| M5 | CLI | Versioned JSON contract, `status` / `provider` / `diagnose` / `refresh` |
| M6 | Local HTTP API | Loopback, disableable, documented risk, restrictive CORS |
| M7 | Deep diagnostics + provider status pages | Strategy trace, redacted report export, incident surfacing |
| M8 | Full alerts engine | Budget, unusual spend, reset, unavailable; dedupe + cooldown |
| M9 | Additional providers | Gemini CLI, Z.ai/GLM, MiniMax, DeepSeek, OpenRouter consolidation, … (quality gated) |
| M10 | Multi-account | Account arrays surfaced in UI + CLI + API |
| M11 | TUI (optional) | Terminal surface on the same core |
| M12 | macOS / Linux | Tray + build + signing for the other platforms |
| M13 | Auto-update + release channels | stable / beta, code signing + notarization |
| M14 | Widgets / IDE extension (exploratory) | — |

Order may change only if analysis of the repo shows a strong reason.

## Documentation to produce (across milestones)

`ARCHITECTURE.md`, `PROVIDER_CONTRACT.md`, `ADDING_PROVIDER.md`, `SECURITY.md`,
`PRIVACY.md`, `DIAGNOSTICS.md`, `RELEASING.md`, and `docs/providers/<provider>.md`
per provider (what is tracked, authentication, data source, endpoints, known
limitations, error meanings).

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
CodexProvider   → ProviderSnapshot
GeminiProvider  → ProviderSnapshot
        ↓
Tray(snapshot)  Desktop(snapshot)  CLI(snapshot)  HTTP(snapshot)
```

No surface knows how any provider authenticates or computes usage. A user opens
one app and understands in under five seconds: which AI they can keep using,
which limit is close, when quota returns, and how much they are spending.
