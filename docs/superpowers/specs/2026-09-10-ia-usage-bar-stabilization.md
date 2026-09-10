# IA Usage Bar — Milestone 0: Stabilization

Date: 2026-09-10
Status: Revised after user review — pending re-approval
Owner: Alberth Salazar
Supersedes: `2026-09-10-ia-usage-bar-overhaul-design.md` (renamed to this file)
North star: `2026-09-10-ia-usage-hub-architecture.md`

## What this milestone is

Take the half-finished fork of "Claude Bar" — a Windows tray app now sitting in
an uncommitted ~2100-line working tree that turns it into a multi-provider AI
usage monitor — and land it as a **healthy, self-consistent, publishable
project called IA Usage Bar (v0.1.0)**.

This milestone answers: *how do I turn this half-done fork into a sane project?*
It does **not** build the full "AI Usage Hub" (CLI, local HTTP API, TUI, pace
engine, adaptive refresh, multi-account, provider status). That is the north
star doc, and this milestone is deliberately its first slice — nothing here
blocks it, and two pieces (the provider contract and the normalized snapshot)
are pulled forward specifically so the Hub work does not have to unpick them.

### Explicitly out of scope

- Stream Deck. Not a surface, not a client, not an integration. Ignore it
  entirely.
- Other repos (CarteraGo, `lenadweb/stream-deck-ai-limits`, etc.). This repo is
  the only source of truth.
- macOS / Linux. Windows 10/11 only for v0.1.0; portability is a north-star
  concern.
- Porting any provider logic to TypeScript. The engine stays Rust.
- CLI, local HTTP API, TUI, adaptive refresh, pace/headroom engine, provider
  status pages, multi-account UI. North star, later milestones.

## Decisions (from user review)

- **Sequence: phased with checkpoints.** Each phase ends in one verified commit.
  The user runs the app in the Windows tray between phases and confirms before
  the next starts.
- **Framework: keep Tauri 2 + Rust backend + vanilla TS/Vite frontend.**
  `cargo check` and `npm run build` both pass today. No framework change — see
  the ADR in the north-star doc; there is no measurable problem a rewrite would
  solve.
- **Design: polish the existing dark theme.** No light mode, no blue-gradient
  concept.
- **Git: squash to a clean initial commit** at the end — but tag an archive
  branch first (see Git strategy). GitHub repo renamed `claude-bar-windows` →
  `ia-usage-bar`.
- **Testing:** Rust unit + fixture tests for money/quota logic and for the
  provider contract. No frontend test framework — manual smoke test per phase.
  CI gains `cargo test`.
- No new runtime dependencies.
- **Provider knowledge must not centralize.** See Phase 2 — this is the one
  substantive change from the original doc.

## Current state (baseline facts)

- Tauri 2, Rust backend + vanilla TypeScript frontend (Vite, no framework).
- `cargo check` + `npm run build` pass on the current working tree.
- Rename already done in the working tree: `README.md`, `Cargo.toml`
  (`iausagebar`), `tauri.conf.json` (`IA Usage Bar`, `com.alberth.iausagebar`),
  UI footer, i18n.
- Providers already split under `src-tauri/src/providers/` (antigravity,
  apikey, claude, codex, copilot, cursor, kiro, local, openai_admin, mod).
- **Provider metadata is already a god file.** `model.rs` carries `VendorId`
  with 23 variants and five parallel giant `match` arms (`slug`, `display_name`,
  `short`, `env_key`, `auth_kind`, `login_hint`). `providers/mod.rs::dispatch`
  is a sixth match. Adding a provider today means editing 6+ central arms.
- `model.rs` already has an embryonic normalized model: `MetricLine` enum
  (`Progress` / `Values` / `Badge`, `#[serde(tag = "kind")]`), `ProviderSnapshot`
  (id, name, short, plan, connected, stale, error, hint, updated_at, lines,
  primary_utilization), a `visible: "always" | "demand"` field per line, and a
  basic `most_headroom`.
- `Provider` trait: `id()`, `has_local_credentials(cfg)`, `refresh(cfg) ->
  ProviderSnapshot`. Each provider does **one** fetch path — no strategy
  ordering, no fallback record, no source/confidence tracking.
- `http.rs`: 20s timeout, redirect cap, `429 → RateLimited`. No domain
  allowlist, no `Retry-After`, no backoff, new `Client` per call.
- God files: `lib.rs` (~25 KB, `#[tauri::command]` + state + dashboard assembly
  + tray refresh loop), `model.rs` (~17 KB), `src/main.ts` (~900 lines after
  the diff).
- Untracked new backend modules: `config.rs`, `http.rs`, `jwt.rs`, `paths.rs`,
  `providers/`. Deleted-but-tracked: `claude_api.rs`, `credentials.rs`.
- Stray `Claude Bar.html` (73 KB design-canvas export) at repo root.
- Untracked: `NOTICE` (complete, correct), `cspell.json`, `src/vite-env.d.ts`.
- `src-tauri/icons/` carries android/ + ios/ trees (~45 files) unused by a
  Windows app; app icon art is pre-fork Claude Bar (Jun 25).
- `docs/*.png` screenshots show the old Claude Bar UI.
- CI (`.github/workflows/build.yml`) runs `npm run build` + `cargo check` only.
- Remote: `github.com/Shadelight/claude-bar-windows`.

## Phases

### Phase 0 — Audit the working tree

**Purpose:** the uncommitted ~2100-line diff is the single largest risk.
Nothing proceeds until it is understood and committed as a known-good baseline.

**Work:**

- Read the full diff module by module: `lib.rs` (+784), `model.rs` (+588),
  `main.ts` (+891), `styles.css` (rewritten), `cost.rs`, `pricing.rs`,
  `tray_icon.rs`, every new `providers/*`.
- Look for: regressions vs. original Claude Bar (Claude session / weekly / cost
  still correct); swallowed errors (`unwrap()`, `let _ =`, silent `Err` on
  network + file paths); credential leaks (tokens in logs, plaintext
  fallbacks, keyring misuse); fragile Windows paths; panic paths in the tray
  refresh loop.
- Produce a findings list, severity-tagged. Fix confirmed correctness bugs
  here; defer style-only items to their phase.

**Checkpoint:** user runs the app, confirms each enabled provider shows real
data and the tray icon paints. Commit: `Initial commit: IA Usage Bar` (squash
base).

**Tests:** add `cargo test` fixture modules for provider response parsing (one
captured JSON per provider) and for percentage / reset parsing in `model.rs`.

### Phase 1 — Rename completion + god-file split

**Rename cleanup:**

- Delete tracked-but-removed `src-tauri/src/claude_api.rs`, `credentials.rs`.
- Move `Claude Bar.html` → `docs/design/canvas-2026-09.html`.
- `git mv docs/*.png docs/screenshots/` (replaced in Phase 3).
- Delete `src-tauri/icons/android/`, `src-tauri/icons/ios/`.
- Keep a copy of the old `icon.ico` in `docs/design/` for reference.
- User renames the local folder `Claude Bar` → `IA Usage Bar`; Claude adjusts
  the working directory.
- Rename GitHub repo `claude-bar-windows` → `ia-usage-bar`; update `origin`.
- Grep for remaining `Claude Bar` / `claudebar` / `daybi` / `com.daybi` outside
  `NOTICE`, `README`, `LICENSE`, `docs/design/`.
- Commit `NOTICE`, `cspell.json`, `src/vite-env.d.ts`.

**Backend god-file split** (`src-tauri/src/`):

| File | Responsibility |
|---|---|
| `lib.rs` | Tauri builder, plugin registration, window/tray wiring only |
| `commands.rs` | `#[tauri::command]` fns + their payload types |
| `state.rs` | `AppState`, shared `AppConfig` handle, refresh guards |
| `dashboard.rs` | Assemble the `Dashboard` snapshot; background refresh loop |
| `tray.rs` | (was `tray_icon.rs`) icon painting + tray events |
| `config.rs` `paths.rs` `http.rs` `jwt.rs` `cost.rs` `pricing.rs` | unchanged |

`model.rs` is split in Phase 2, not here.

**Frontend split** (`src/`):

| File | Responsibility |
|---|---|
| `main.ts` | Bootstrap, event listeners, view routing |
| `api.ts` | `invoke` wrappers + all TS interfaces |
| `i18n.ts` | `I18N` string tables + `t()` |
| `views/dash.ts` `views/settings.ts` `views/spend.ts` | render per view |
| `styles.css` | unchanged location |

**Checkpoint:** `cargo check`, `npm run build`, user smoke test. Commit:
`Restructure backend and frontend into focused modules`.

### Phase 2 — Formalize the provider contract (the substantive change)

**Problem:** provider knowledge is centralized in `model.rs` (6 parallel match
arms) and `providers/mod.rs::dispatch`. At 23 vendors it is already unpleasant;
the north star wants many more plus per-provider capabilities, auth sources,
metrics, dashboard/status URLs. A single `vendor.rs` knowledge dump just moves
the god file — it does not fix it.

**Target: each provider owns its own metadata and logic.**

```
src-tauri/src/providers/
  mod.rs            registry + dispatch by iterating descriptors, not a match
  descriptor.rs     ProviderDescriptor type + Capabilities + AuthSource types
  claude/
    mod.rs          pub const DESCRIPTOR + impl Provider
    auth.rs         local credential reading
    fetch.rs        the HTTP call(s)
    mapper.rs       JSON -> Vec<Metric>
  codex/  cursor/  antigravity/  copilot/  kiro/  openai_admin/  apikey/  local/
```

`model.rs` splits into:

- `model.rs` — the normalized snapshot / metric types only.
- `model/vendor.rs` — a slim `VendorId` enum plus `VendorId::all()`. No
  `display_name` / `short` / `login_hint` / `env_key` matches — those move onto
  each provider's `DESCRIPTOR`.

**`ProviderDescriptor`** (each provider declares one `const`):

```rust
pub struct ProviderDescriptor {
    pub id: VendorId,              // stable
    pub slug: &'static str,
    pub display_name: &'static str,
    pub short: &'static str,
    pub website: &'static str,
    pub dashboard_url: Option<&'static str>,
    pub status_url: Option<&'static str>,
    pub auth: &'static [AuthSourceKind],   // ordered by priority
    pub env_key: Option<&'static str>,
    pub login_hint: &'static str,
    pub capabilities: Capabilities,
}

pub struct Capabilities {
    pub quota: bool,
    pub spend: bool,
    pub credits: bool,
    pub model_breakdown: bool,
    pub local_history: bool,
    pub multi_account: bool,   // false for all today; the field exists now
}
```

The catalog (`VendorInfo` sent to the UI) is built by iterating
`ProviderDescriptor`s, so the UI never learns a provider name from a `match`.

**Normalized snapshot / metric** — evolve, do not replace, the current
`MetricLine`. Keep serialization stable where the frontend already depends on
it; add fields rather than rename.

```rust
pub struct ProviderSnapshot {
    pub id: String,
    pub account: Option<String>,   // None today; multi-account ready
    pub plan: String,
    pub status: SnapshotStatus,
    pub metrics: Vec<Metric>,      // was `lines`
    pub source: Option<String>,    // which auth/fetch path produced this
    pub freshness: Freshness,      // fresh | stale
    pub confidence: Confidence,    // high | medium | low
    pub updated_at: String,
    pub errors: Vec<String>,
    pub primary_utilization: Option<f64>,
}

pub enum SnapshotStatus {
    Fresh, Stale, Loading, Unavailable, Unsupported,
    NotAuthenticated, RateLimited, NetworkError, ProviderError,
}

pub struct Metric {
    pub id: String,
    pub label: String,
    pub kind: MetricKind,
    pub visible: Visibility,       // always | demand | pinned
    pub resets_at: Option<String>,
    pub window_secs: Option<i64>,
}

pub enum MetricKind {
    Progress { used: f64, limit: f64, unit: Unit },
    Value    { text: String },
    Spend    { amount: f64, currency: String, period: String, basis: SpendBasis },
    Credits  { used: f64, remaining: f64, total: f64 },
    Count    { used: f64, limit: Option<f64>, unit: Unit },
    RateLimit{ remaining: f64, limit: f64 },
    Badge    { text: String },
    Text     { text: String },
    // Chart deferred to the Hub milestone that needs it.
}

pub enum SpendBasis { AccountWide, EstimatedLocal }
```

Rules baked in now:

- "No data" is never `0`. Absent metric or explicit status, never a zero bar.
- A snapshot with any usage window works — do not assume exactly
  session + weekly. Claude already emits up to 6 windows.
- Spend always carries its `basis`; the UI labels "Account usage" vs
  "Estimated from local activity".

**Contract tests** (new `cargo test` suite):

- Every registered provider has a descriptor with a unique `id` and `slug`.
- Every descriptor's `env_key` (if any) is unique.
- No provider module references another provider's module.
- Architecture guard: `grep` test failing on `if .*VendorId::` /
  `match vendor` outside `src-tauri/src/providers/` and `model/vendor.rs`.

**Checkpoint:** `cargo test`, `cargo check`, `npm run build`, user smoke test —
every provider still renders the same data. Commit: `Formalize ProviderDescriptor
and normalized snapshot contract`.

### Phase 3 — Dark visual identity + real states + icons

**Design tokens** in `styles.css` (replace ad-hoc values): spacing scale
`4/8/12/16/24`; type scale `11/12/13/16/20`, weights 400/600/700; one accent;
`--ok --warn --hot` kept; a per-card `--brand` custom property for the active
provider.

**States** — make all of `SnapshotStatus` render deliberately: loading
(skeleton bars, not blank), refreshing, fresh, stale, offline,
not-authenticated (inline with the provider's `login_hint`), rate-limited,
unsupported, no-data, provider-error. Keep the last good metrics visible when
status is stale/rate-limited.

**Icons:** one 1024×1024 source PNG for the new mark →
`npm run tauri icon` regenerates the set. Tray icon: keep the painted
usage-percent text, add a 1px contrasting outline / rounded plate for
legibility on light and dark taskbars; verify at 16px.

**Checkpoint:** user checks the panel and tray on a light and a dark taskbar.
Capture new `docs/screenshots/`. Commit: `Dark design system, real states,
new icon set`.

### Phase 4 — Settings + alerts

- **Launch at Windows startup:** Settings toggle (General), backed by the
  already-present `tauri-plugin-autostart`. Reflect real OS state on load.
- **Configurable notification thresholds:** move the hardcoded 75/90/95 into
  `AppConfig` as `notify_thresholds: Vec<u8>` (default `[75, 90, 95]`),
  validated 1–99 / sorted / deduped. `dashboard.rs` reads the vector.
- **Notification dedupe + cooldown:** do not re-fire the same
  provider+threshold within a window; fire once per crossing.
- Per-provider enable/disable and reorder already exist — keep.

**Checkpoint:** user toggles autostart (verify via Task Manager / reboot),
edits thresholds, confirms one notification per crossing. Commit: `Launch at
login, configurable + deduped notification thresholds`.

### Phase 5 — Community docs + release

- `CONTRIBUTING.md` (build prereqs: Rust stable, Node 20, `npm i`,
  `npm run tauri dev`; project layout; **how to add a provider** — point at the
  Phase 2 module layout; PR expectations).
- `CHANGELOG.md` (Keep a Changelog; `0.1.0`).
- `SECURITY.md` (credential handling: keyring, no telemetry, vendor endpoints
  only; private disclosure path).
- `.github/ISSUE_TEMPLATE/{bug_report,feature_request}.yml`,
  `pull_request_template.md`.
- README: refresh screenshots, verify links, add build-from-source.
- `release.yml`: on tag `v*`, `windows-latest`, `npm run tauri build`, upload
  MSI + NSIS to a GitHub Release. Add `cargo test` to `build.yml`.
- Repo description + topics (`tauri`, `rust`, `windows`, `tray`, `claude`,
  `openai`, `usage-monitor`).

**Checkpoint:** dispatch the release workflow on a test tag, confirm it
produces installers. Commit: `Contributor docs, issue templates, release
workflow`.

## Git strategy (squash, end of milestone)

After Phase 5 verifies:

1. `git tag archive/pre-ia-usage-bar-squash && git push origin archive/pre-ia-usage-bar-squash`
   — the emergency door. Do this **first**.
2. `git checkout --orphan release-clean`
3. `git add -A && git commit -m "Initial commit: IA Usage Bar"`
   (body: one paragraph on what it is + "Derived from Claude Bar by Daybi; see
   NOTICE for full attribution.")
4. `git branch -M release-clean main`
5. Confirm with the user, then `git push --force origin main`.

`docs/superpowers/` specs are kept in the clean tree.

## Testing strategy

- `model.rs` / `model/vendor.rs`: unit tests (percentage, reset parsing,
  `monthly_spend`, `most_headroom`, slug uniqueness — some already exist).
- `providers/*/mapper.rs`: fixture tests — one captured JSON response per
  provider → asserted `Vec<Metric>`.
- Contract + architecture-guard suite (Phase 2).
- No real API calls in CI.
- Frontend: manual smoke test at each checkpoint.
- CI: `build.yml` gains `cargo test`; `release.yml` added.

## Risks

- **Force-push destroys published history.** Personal repo, user-requested,
  archive tag pushed first (Git strategy step 1), confirm before step 5.
- **Phase 0: large uncommitted diff + Phase 1 file moves.** Phase 0 commits the
  baseline first so Phase 1's moves are a reviewable diff of their own.
- **Phase 2 touches every provider + the frontend's `lines` → `metrics`
  rename.** Mitigated by evolving serialization (add, don't rename where the
  frontend depends on it) and the per-provider fixture tests landing in the
  same phase.
- **Tray icon legibility** can only be judged on real taskbars — built into the
  Phase 3 checkpoint.
- **`npm run tauri icon` overwrites the whole icon set** — intended; old `.ico`
  kept in `docs/design/`.

## Deliverable order recap

Phase 0 audit → 1 rename + god-file split → 2 provider contract → 3 design +
icons → 4 settings + alerts → 5 docs + release → squash.

Each phase ends working and committed. After this milestone the north-star doc
governs.
