# IA Usage Bar — Overhaul Design

Date: 2026-09-10
Status: Approved (pending user spec review)
Owner: Alberth Salazar

## Goal

Take the half-finished fork of "Claude Bar" (a Windows tray app now sitting in
an uncommitted 2100-line working tree that converts it to a multi-provider AI
usage monitor) and land it as a clean, self-consistent, community-shareable
project called **IA Usage Bar**.

Four outcomes:

1. Finish the rename across repo, folder, code, and GitHub.
2. Reorganize files so no single file is a god file.
3. Give the dark UI a real visual identity and a fresh icon set.
4. Make the repo presentable to outside contributors (docs, templates, release CI).

Plus two small config features and a clean squashed git history.

## Constraints and decisions

- **Sequence: phased with checkpoints (option A).** Each phase ends in one
  verified commit. The user runs the app in the Windows tray between phases and
  confirms before the next phase starts.
- **Design: polish the existing dark theme.** No light mode, no blue-gradient
  concept. Dark-only is correct for a tray popover.
- **Git: squash to a single clean initial commit** at the end, force-pushed to
  `main`. Attribution to Claude Bar / ai-usagebar / the other MIT sources lives
  in `NOTICE` and `README`.
- **GitHub repo name: `ia-usage-bar`** (rename from `claude-bar-windows`).
- **Testing:** Rust unit tests for money/quota logic (provider parsing, pace
  projection). No frontend test framework — the user does a manual smoke test
  per phase. CI gains `cargo test`.
- Windows 10/11 only. iOS/Android icon assets get deleted.
- No new runtime dependencies. `tauri-plugin-autostart` is already a dependency.

## Current state (baseline facts)

- Tauri 2, Rust backend + vanilla TypeScript frontend (Vite, no framework).
- `cargo check` and `npm run build` both pass on the current working tree.
- Rename already done in the working tree: `README.md`, `Cargo.toml`
  (`iausagebar`), `tauri.conf.json` (`IA Usage Bar`, `com.alberth.iausagebar`),
  UI footer, i18n strings.
- Providers already split cleanly under `src-tauri/src/providers/`
  (antigravity, apikey, claude, codex, copilot, cursor, kiro, local,
  openai_admin, mod).
- God files: `src-tauri/src/lib.rs` (~25 KB), `src-tauri/src/model.rs`
  (~17 KB), `src/main.ts` (~900 lines after the diff).
- Untracked new backend modules: `config.rs`, `http.rs`, `jwt.rs`, `paths.rs`,
  `providers/`. Deleted-but-still-tracked: `claude_api.rs`, `credentials.rs`.
- Stray file: `Claude Bar.html` (73 KB design-canvas export) at repo root.
- Untracked: `NOTICE` (complete and correct), `cspell.json`, `src/vite-env.d.ts`.
- Icons are the pre-fork Claude Bar art (dated Jun 25). `src-tauri/icons/`
  carries full android/ and ios/ trees (~45 files) that a Windows app never uses.
- Docs screenshots (`docs/compact.png`, `docs/panel.png`, `docs/tray.png`) show
  the old Claude Bar UI.
- CI (`.github/workflows/build.yml`) only runs `npm run build` + `cargo check`.
- Remote: `https://github.com/Shadelight/claude-bar-windows.git`.

## Phase 0 — Baseline review

**Purpose:** the uncommitted working tree is the single largest risk. Nothing
proceeds until it is understood and committed as a known-good baseline.

**Work:**

- Read the full diff, module by module: `lib.rs` (+784), `model.rs` (+588),
  `main.ts` (+891), `styles.css` (rewritten), `cost.rs`, `pricing.rs`,
  `tray_icon.rs`, and every new `providers/*` file.
- Look specifically for:
  - Regressions vs. the original Claude Bar behavior (Claude Code session /
    weekly / cost still correct).
  - Swallowed errors — `unwrap()` / `let _ =` / silent `Err` on network and
    file-read paths.
  - Credential handling — keys reaching logs, plaintext fallbacks, keyring
    misuse.
  - Fragile Windows paths — hardcoded separators, missing `%USERPROFILE%`
    fallbacks, assumptions about which tools are installed.
  - Panic paths in the tray refresh loop (a panic there kills the icon).
- Produce a findings list (severity-tagged). Fix confirmed correctness bugs in
  this phase; defer style-only items to their natural phase.

**Checkpoint:** user runs the app, confirms each enabled provider shows real
data and the tray icon paints. Then commit as `Initial commit: IA Usage Bar`
(this becomes the squash base — see Git strategy).

**Testing:** add `cargo test` modules for provider response parsing (fixture
JSON per provider) and for pace projection in `model.rs`.

## Phase 1 — Rename completion + file structure

**Rename cleanup:**

- Delete tracked-but-removed `src-tauri/src/claude_api.rs`,
  `src-tauri/src/credentials.rs`.
- Move `Claude Bar.html` → `docs/design/canvas-2026-09.html`.
- `git mv docs/*.png docs/screenshots/` (replaced in Phase 2).
- Delete `src-tauri/icons/android/` and `src-tauri/icons/ios/`.
- Rename local folder `D:\Users\Escritorio\Claude Bar` → `IA Usage Bar`
  (user does this; Claude adjusts working directory).
- Rename GitHub repo `claude-bar-windows` → `ia-usage-bar`; update `origin`.
- Grep for any remaining `Claude Bar` / `claudebar` / `daybi` / `com.daybi`
  string outside `NOTICE`, `README`, `LICENSE`, and `docs/design/`.
- Commit `NOTICE`, `cspell.json`, `src/vite-env.d.ts`.

**Backend target structure** (`src-tauri/src/`):

| File | Responsibility |
|---|---|
| `lib.rs` | Tauri builder, plugin registration, window/tray setup wiring only |
| `commands.rs` | `#[tauri::command]` functions + their request/response payload types |
| `state.rs` | `AppState`, shared `AppConfig` handle, mutex/refresh guards |
| `dashboard.rs` | Assemble the `Dashboard` snapshot; the background refresh loop |
| `tray.rs` | (was `tray_icon.rs`) icon painting + tray event handling |
| `config.rs` `paths.rs` `http.rs` `jwt.rs` `cost.rs` `pricing.rs` `model.rs` | unchanged |
| `providers/` | unchanged |

`model.rs` split: keep snapshot/metric types in `model.rs`; move the `VendorId`
enum, display names, slugs, auth-kind metadata, and catalog wiring into a new
`vendor.rs`.

**Frontend target structure** (`src/`):

| File | Responsibility |
|---|---|
| `main.ts` | Bootstrap, event listeners, view routing |
| `api.ts` | `invoke` wrappers + all TS interfaces (`Dashboard`, `ProviderSnapshot`, `VendorInfo`, `SpendRow`) |
| `i18n.ts` | The `I18N` string tables + `t()` helper |
| `views/dash.ts` | Render provider tabs + detail |
| `views/settings.ts` | Render settings body |
| `views/spend.ts` | Render spend view |
| `styles.css` | unchanged location |

**Checkpoint:** `cargo check`, `npm run build`, user smoke test. Commit:
`Restructure backend and frontend into focused modules`.

## Phase 2 — Dark visual identity + icons

**Design tokens** (top of `styles.css`, replacing ad-hoc values):

- Spacing scale: `--s1: 4px; --s2: 8px; --s3: 12px; --s4: 16px; --s5: 24px`.
- Type scale (fixed steps): `11 / 12 / 13 / 16 / 20 px`; weights 400/600/700 only.
- One accent (`--accent`), kept. Semantic: `--ok --warn --hot` kept.
- Per-provider brand color: a `--brand` custom property set per card
  (Anthropic clay, OpenAI green, Cursor, etc.), used for the active tab glyph
  and the quota bar fill on that provider's card only.

**States** — make all four real, styled, and wired:

- Empty (no providers) — already exists, restyle.
- Loading / first fetch — skeleton bars, not a blank card.
- Error (auth failed / network) — inline on the card with the vendor's login hint.
- Stale (`nextUpdateInSecs` overdue / last fetch old) — the existing `#stall`
  banner, restyled.

**Icons:**

- One 1024×1024 source PNG for the new IA Usage Bar mark → `npm run tauri icon`
  regenerates `128/64/32`, `.ico`, `.icns`, and the Square*Logo set.
- Tray icon: keep the painted usage-percent text; add a 1px contrasting outline
  or rounded background plate so it stays legible on both light and dark
  Windows taskbars. Verify at 16px.

**Checkpoint:** user runs the app, checks the panel and the tray icon on a
light and a dark taskbar. New screenshots captured here for `docs/screenshots/`.
Commit: `Dark theme design system, real states, new icon set`.

## Phase 3 — Configuration

Two features only.

**Launch at Windows startup:**

- Toggle in Settings ("General" section).
- Backend command `set_autostart(bool)` calling the already-present
  `tauri-plugin-autostart` manager; `get_autostart()` for initial state.
- Reflect actual OS state on load, not a stored guess.

**Configurable notification thresholds:**

- Currently hardcoded at 75 / 90 / 95 %. Move to `AppConfig` as
  `notify_thresholds: Vec<u8>` (default `[75, 90, 95]`).
- Settings UI: a small comma-separated input or three number fields, validated
  (1–99, sorted, deduped) before save.
- The notification-firing logic in `dashboard.rs` reads the config vector.

**Checkpoint:** user toggles autostart (verify via Task Manager / reboot),
edits thresholds, confirms a notification fires at the new value. Commit:
`Add launch-at-login and configurable notification thresholds`.

## Phase 4 — Community docs + release

**Files:**

- `CONTRIBUTING.md` — build prereqs (Rust stable, Node 20, `npm i`,
  `npm run tauri dev`), project layout, how providers are added, PR expectations.
- `CHANGELOG.md` — Keep a Changelog format; `0.1.0` entry.
- `SECURITY.md` — credential handling summary (keyring, no telemetry, vendor
  endpoints only), how to report a vulnerability privately.
- `.github/ISSUE_TEMPLATE/bug_report.yml`, `feature_request.yml`,
  `.github/pull_request_template.md`.
- README: refresh screenshots, verify all links, add a build-from-source
  section, add a one-line install path.

**Release workflow** (`.github/workflows/release.yml`):

- Trigger on tag `v*`.
- `windows-latest`; Node 20; Rust stable; `npm ci`; `npm run tauri build`.
- Upload the MSI and NSIS installers to a GitHub Release for that tag.
- Existing `build.yml` stays as the PR/push check; add `cargo test` to it.

**Repo metadata:** description ("Windows tray monitor for your AI plan
usage — quota, reset, and spend across Claude, Codex, Cursor, and more"),
topics (`tauri`, `rust`, `windows`, `tray`, `claude`, `openai`, `usage-monitor`).

**Checkpoint:** push a `v0.1.0` tag on a test branch or use `workflow_dispatch`
to confirm the release build produces installers. Commit: `Add contributor
docs, issue templates, and release workflow`.

## Git strategy (squash, end of project)

After Phase 4 is verified:

1. `git checkout --orphan release-clean`
2. `git add -A && git commit -m "Initial commit: IA Usage Bar"`
   (commit body: one paragraph on what it is + "Derived from Claude Bar by
   Daybi; see NOTICE for full attribution.")
3. `git branch -M release-clean main`
4. `git push --force origin main`

Confirm with the user immediately before step 4. The `docs/superpowers/` specs
are kept in the clean tree.

## Risks

- **Force-push destroys the published history.** Personal repo, user-requested,
  confirmed at execution time. No forks/PRs exist to break.
- **File moves + a large uncommitted diff at once (Phase 1).** Mitigated by
  Phase 0 committing the baseline first, so Phase 1's moves are a reviewable
  diff on their own.
- **Tray icon legibility** can only be truly judged by the user on real
  taskbars — built into the Phase 2 checkpoint.
- **`npm run tauri icon` overwrites the whole icon set** including the Square
  logos; that is the intent, but the old `.ico` should be kept in
  `docs/design/` for reference.

## Out of scope

- Light mode / OS theme following.
- The blue-gradient design concept.
- Global hotkey, tray-click-to-open rework, status export/clipboard.
- Auto-primary (highest-usage) selection.
- macOS / Linux support.
- New providers beyond what the working tree already implements.
