# Milestone 0 — Stabilization Implementation Plan (Part 1: Phases 0–1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the uncommitted ~2100-line working tree that converts "Claude Bar" into a multi-provider monitor into a committed, audited, reorganized, still-green baseline for "IA Usage Bar".

**Architecture:** Tauri 2 desktop app, Rust backend (`src-tauri/`), vanilla TypeScript + Vite frontend (`src/`). This part does two things: (1) read and commit the working tree as a known-good baseline after fixing any confirmed correctness bugs, (2) finish the rename and split the two god files (`src-tauri/src/lib.rs`, `src/main.ts`) into focused modules with no behaviour change.

**Tech Stack:** Rust 2021 / Tauri 2, `serde`, `reqwest` (blocking), `chrono`, `keyring`, `rusqlite`; TypeScript 5.6, Vite 5.4, `@tauri-apps/api` v2. Tests: `cargo test` only (no frontend test framework).

**Spec:** `docs/superpowers/specs/2026-09-10-ia-usage-bar-stabilization.md` (north star: `docs/superpowers/specs/2026-09-10-ia-usage-hub-architecture.md`)

## Scope of this document

Milestone 0 in the spec has six phases (0–5). This plan covers **Phase 0
(audit) and Phase 1 (rename + god-file split)** only, because:

- Phase 0 produces a findings list by reading a diff nobody has read yet. Its
  results materially shape Phases 2–5 (the provider-contract refactor, the
  design pass). Writing concrete TDD steps for those now would be guesswork.
- Phase 0 + 1 is a complete, shippable deliverable on its own: a committed,
  reorganized, building, test-green tree.

**Part 2** (`2026-09-10-milestone-0-stabilization-part2.md`) will cover Phases
2–5 (provider `ProviderDescriptor` + normalized `ProviderSnapshot` contract,
dark design system + icons, settings + alerts, community docs + release, the
squash) and is written after Task 2 of this plan lands the audit.

## Global Constraints

Every task's requirements implicitly include this section. Values copied verbatim from the spec.

- **Platform:** Windows 10/11 only. No macOS / Linux / Android / iOS code in Milestone 0. Deleting the Tauri-generated `src-tauri/icons/android/` and `src-tauri/icons/ios/` trees is Windows-desktop cleanup, not a statement about platform support.
- **No new runtime dependencies.** No new entries under `[dependencies]` in `src-tauri/Cargo.toml` or `dependencies` in `package.json`. Dev-only tooling is allowed only if a task explicitly calls for it (none here).
- **Framework is fixed:** Tauri 2 + Rust backend + vanilla TypeScript/Vite frontend. No framework, bundler, or language change.
- **Green at every checkpoint:** `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `npm run build` all pass. The user additionally runs the app in the Windows tray at each checkpoint and confirms before the next phase.
- **No behaviour change in Phase 1.** The rename and the god-file split move code; they do not alter what the app does. Verification is "the existing suite + build + manual smoke test are still green", not new behavioural tests.
- **Git safety:** run `git status` before touching anything. Never run `git reset --hard`, `git clean -fd`, `git checkout .`, or `git checkout -- <path>`. Never delete the user's branches or stashes. If unrelated modifications appear, work around them and report them.
- **Provider knowledge must not centralize further.** Do not add new `match VendorId { … }` arms or `if id == "claude"` checks anywhere. The existing central matches in `model.rs` stay untouched in this part — they are Phase 2's problem.
- **Naming:** product name `IA Usage Bar`; bundle identifier `com.alberth.iausagebar`; npm package `iausagebar`; Rust crate `iausagebar` / lib `iausagebar_lib`; target GitHub repo name `ia-usage-bar`. Attribution to the original Claude Bar (Daybi) and to `ai-usagebar` / the other MIT sources lives only in `NOTICE`, `README.md`, and `LICENSE`.
- **Commit message trailer:** every commit ends with:

  ```
  Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL
  ```

- **Line endings:** the repo has mixed LF/CRLF and git prints `LF will be replaced by CRLF` warnings. Ignore them; do not add a `.gitattributes` normalization pass in this part.

---

## File Structure

### Backend (`src-tauri/src/`) — after Phase 1

| File | Responsibility | Change |
|---|---|---|
| `lib.rs` | crate root: `mod` declarations, `pub fn run()` (Tauri builder, plugin registration, `.setup`, tray creation, `invoke_handler`, spawn of the refresh loop) | shrinks from ~760 to ~120 lines |
| `commands.rs` | **new.** All `#[tauri::command]` fns (`get_dashboard`, `refresh_now`, `refresh_provider`, `detect_providers`, `get_app_config`, `set_app_config`, `set_provider_enabled`, `save_api_key`, `quit`, `hide_panel`, `set_notifications`) + the `parse_id` helper | new |
| `state.rs` | **new.** `AppState`, `TrayMenuState`, `NotifyState`, `UsageAlert` struct/enum defs and their inherent impls | new |
| `dashboard.rs` | **new.** `build_dashboard`, `emit_dashboard`, `do_refresh`, `refresh_sync`, `run_loop`, `check_notifications`, `take_usage_alerts`, `reset_happened`, `parse_ts` | new |
| `tray.rs` | **renamed from `tray_icon.rs`** + absorbs the tray-behaviour fns from `lib.rs`: `render` (existing), `on_tray_left_click`, `position_window`, `show_window`, `tray_percent`, `tooltip`, `update_tray_from_dashboard` | rename + grow |
| `config.rs` `paths.rs` `http.rs` `jwt.rs` `cost.rs` `pricing.rs` `model.rs` | unchanged in this part | none |
| `providers/` | unchanged in this part | none |
| `claude_api.rs` `credentials.rs` | **deleted** (already removed in the working tree, still tracked) | delete |

### Frontend (`src/`) — after Phase 1

| File | Responsibility | Change |
|---|---|---|
| `main.ts` | bootstrap: `main()`, the global `document` click/change listeners, `handleAction`, view routing (`view` variable + the `renderX` dispatch), `fitWindow`, `applyDashboard` | shrinks from ~700 to ~200 lines |
| `api.ts` | **new.** `isTauri`, `invokeCmd<T>`, and every shared `interface` / `type`: `MetricLine`, `ProviderSnapshot`, `VendorInfo`, `SpendRow`, `Dashboard` | new |
| `i18n.ts` | **new.** `I18N` table, `lang` state + `localStorage` read, `t()` | new |
| `views/dash.ts` | **new.** `renderDash`, `detailHtml`, `progressBlock`, `addedProviders`, and the per-metric helpers only `renderDash` uses (`pctOf`, `windowLabel`, `resetCountdown`, `exactReset`, `paceNote`, `counts`, `glyph`, `accent`, `tabName`, `escapeHtml` — move `escapeHtml` here or to `api.ts` if `views/settings.ts` also needs it) | new |
| `views/settings.ts` | **new.** `renderSettings`, `persistConfig` | new |
| `views/spend.ts` | **new.** `renderSpend`, `previewDashboard` | new |
| `styles.css` | unchanged | none |
| `vite-env.d.ts` | unchanged (currently untracked → committed in Task 4) | commit |

`ACCENT` and `TAB_NAME` maps (currently top of `main.ts`) move to `views/dash.ts` unless Task 7 finds another consumer, in which case they go to `api.ts`.

### Docs / assets

| Path | Change |
|---|---|
| `docs/design/phase-0-audit.md` | **new** — the audit findings (Task 1) |
| `docs/design/canvas-2026-09.html` | **moved** from repo-root `Claude Bar.html` |
| `docs/design/legacy-icon.ico` | **copied** from `src-tauri/icons/icon.ico` before Phase 2 regenerates it (kept for reference) |
| `docs/screenshots/{compact,panel,tray}.png` | **moved** from `docs/{compact,panel,tray}.png` |
| `NOTICE`, `cspell.json`, `src/vite-env.d.ts` | **committed** (currently untracked) |
| `src-tauri/icons/android/`, `src-tauri/icons/ios/` | **deleted** |
| `.github/workflows/build.yml` | **modified** — add a `cargo test` step |

---

## Phase 0 — Audit the working tree

### Task 1: Audit the uncommitted diff and record findings

**Files:**
- Create: `docs/design/phase-0-audit.md`
- Read only: the full `git diff` plus every untracked file under `src-tauri/src/` and `src/`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: `docs/design/phase-0-audit.md` with a findings table whose columns are `id | area | severity (high/med/low) | status (confirmed/suspected) | file:line | description | fix approach`. Task 2 consumes the `confirmed` + `high` rows. Part 2 consumes everything else.

- [ ] **Step 1: Snapshot the starting state**

Run:
```bash
cd "D:/Users/Escritorio/Claude Bar"
git status
git stash list
git diff --stat
```
Expected: branch `main`, the modified/deleted/untracked set listed in the spec's "Current state" section, **no stashes that predate this work**. If there are unexpected stashes or unrelated modified files, stop and report — do not proceed.

- [ ] **Step 2: Read the backend diff, module by module**

Read in this order, taking notes against the checklist below:
```
src-tauri/src/lib.rs          (git diff — ~784 changed lines)
src-tauri/src/model.rs        (git diff — ~588 changed lines)
src-tauri/src/cost.rs         (git diff)
src-tauri/src/pricing.rs      (git diff)
src-tauri/src/tray_icon.rs    (git diff)
src-tauri/src/providers/*.rs  (untracked — read in full)
src-tauri/src/config.rs src-tauri/src/http.rs src-tauri/src/jwt.rs src-tauri/src/paths.rs  (untracked — read in full)
```
Checklist per file:
- **Regression vs. original Claude Bar:** does Claude Code session %, weekly %, and local cost still get computed and displayed? (Compare `providers/claude.rs` + `cost.rs` against the deleted `claude_api.rs` in `git show HEAD:src-tauri/src/claude_api.rs`.)
- **Swallowed errors:** every `unwrap()`, `expect()`, `let _ =`, and `.ok()` on a network call, file read, or JSON parse — is silent failure acceptable there, or does it hide a real problem the user needs to see?
- **Credential leakage:** any path where an OAuth token / API key / cookie / full credential-file body could reach `println!`, `eprintln!`, `log::`, a `format!` that goes into a `ProviderSnapshot.error`, or a panic message.
- **Fragile Windows paths:** hardcoded `/`, missing `%USERPROFILE%` / `dirs::home_dir()` fallback, assumption that a CLI or its files exist.
- **Panic in the tray loop:** `run_loop` / `refresh_sync` / `check_notifications` / `update_tray_from_dashboard` — any `unwrap`/indexing/`[..]` slice that can panic. A panic on that thread kills the tray icon silently.

- [ ] **Step 3: Read the frontend diff**

Read `git diff src/main.ts` (~891 changed lines) and `git diff src/styles.css`. Checklist:
- Does `invokeCmd` swallow errors in a way that leaves the UI blank instead of showing a state?
- Any `innerHTML` assembled from provider-supplied strings without `escapeHtml`? (Check `detailHtml`, `renderSettings`, `renderSpend`.)
- Does the `es`/`en` `I18N` table have the same key set on both sides? (A missing key falls back to the raw key string in the UI.)

- [ ] **Step 4: Build and run the current tree**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```
Expected: all pass (they did at spec time). Record any failure as a `high / confirmed` finding.

Then ask the user to run `npm run tauri dev`, enable their real providers, and confirm each shows data and the tray icon paints a percentage. Record what they report.

- [ ] **Step 5: Write `docs/design/phase-0-audit.md`**

Fill the findings table. For every `confirmed` + `high` row, the `fix approach` column must name the exact file, the exact change, and how it will be tested. `suspected` rows and `med`/`low` rows get a one-line note and a "→ Part 2, Phase N" pointer. End the file with a "Baseline verdict" paragraph: is the working tree safe to commit as-is (after Task 2's high fixes), or does it need more work first?

- [ ] **Step 6: Commit the audit doc**

```bash
git add docs/design/phase-0-audit.md
git commit -m "docs: Phase 0 working-tree audit findings

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 2: Fix confirmed high-severity findings

**Files:**
- Modify: whichever files Task 1's `confirmed` + `high` rows name (unknown until Task 1 runs)
- Test: add a `#[test]` next to each fix, in that file's existing `#[cfg(test)] mod tests` block (or a new one)

**Interfaces:**
- Consumes: the `confirmed` + `high` rows of `docs/design/phase-0-audit.md`
- Produces: a green tree with a regression test per fix. No public API changes expected; if a fix needs one, note it in the audit doc so Part 2's tasks see it.

> If Task 1's "Baseline verdict" is "safe to commit as-is" and there are **zero** confirmed-high rows, this task is a no-op: skip to Step 4, record "no high findings", and move on.

- [ ] **Step 1: For each confirmed-high finding, write the failing regression test**

Example shape (the real assertion comes from the finding). If a finding is
"`providers/codex.rs:88` panics on a missing `usage` key":
```rust
#[test]
fn codex_snapshot_survives_missing_usage_key() {
    let body = serde_json::json!({ "plan": "plus" }); // no "usage"
    let snap = super::snapshot_from_json("Plus", &body);
    assert!(!snap.connected || snap.error.is_some());
}
```

- [ ] **Step 2: Run it, verify it fails the way the finding predicts**

Run: `cargo test --manifest-path src-tauri/Cargo.toml <test_name> -- --nocapture`
Expected: FAIL (panic / wrong value) matching the audit description.

- [ ] **Step 3: Apply the minimal fix named in the audit's `fix approach` column**

Minimal = the smallest change that makes the test pass without altering
unrelated behaviour. No refactor here; refactors are Phase 1.

- [ ] **Step 4: Run the full suite**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run build
```
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "fix: resolve confirmed high-severity findings from Phase 0 audit

<one bullet per finding fixed, referencing its audit id>

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 3: Commit the working tree as the baseline

**Files:**
- No edits. This task only stages and commits what Phases 0 produced plus the pre-existing working-tree changes, **except** the still-untracked new modules and files handled in Task 4.

**Interfaces:**
- Consumes: a green tree (Task 2)
- Produces: a single commit containing the multi-provider conversion. This commit's tree is what Phase 5 (Part 2) will orphan-squash into `Initial commit: IA Usage Bar`; the archive tag pushed there preserves today's fork history, so the message here is descriptive, not "Initial commit".

- [ ] **Step 1: Review exactly what will be committed**

Run:
```bash
git add -A
git status
git diff --cached --stat
```
Expected staged set: the modified tracked files (`README.md`, `index.html`, `package*.json`, `src-tauri/Cargo.*`, `src-tauri/capabilities/default.json`, `src-tauri/src/{cost,lib,main,model,pricing,tray_icon}.rs`, `src-tauri/tauri.conf.json`, `src/main.ts`, `src/styles.css`), the two deletions (`claude_api.rs`, `credentials.rs`), and the new untracked files (`Claude Bar.html`, `NOTICE`, `cspell.json`, `src-tauri/src/{config,http,jwt,paths}.rs`, `src-tauri/src/providers/`, `src/vite-env.d.ts`, `docs/design/phase-0-audit.md` already committed).

Note: `Claude Bar.html` and the icon-tree deletions are intentionally **not**
done here — they happen in Task 4 so the "rename cleanup" is one reviewable
commit. If `git add -A` staged `Claude Bar.html`, leave it; it moves in Task 4.

- [ ] **Step 2: Commit**

```bash
git commit -m "Convert Claude Bar fork into IA Usage Bar multi-provider monitor

Working tree from the in-progress conversion: 10 provider modules under
src-tauri/src/providers/, normalized MetricLine/ProviderSnapshot model,
keyring-backed API keys, split http/jwt/config/paths modules. Frontend
rewritten for the multi-provider dashboard. Derived from Claude Bar by
Daybi; see NOTICE for full attribution.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

- [ ] **Step 3: Verify the tree is clean**

Run: `git status`
Expected: `nothing to commit, working tree clean` (or only `docs/superpowers/` plan edits if you are editing plans in the same checkout).

- [ ] **Step 4: CHECKPOINT — user smoke test**

Ask the user to `npm run tauri dev` once more against this exact commit and
confirm parity with Step 4 of Task 1. Do not start Phase 1 until they confirm.

---

## Phase 1 — Rename completion + god-file split

### Task 4: Rename cleanup — move and delete files

**Files:**
- Delete: `src-tauri/src/claude_api.rs`, `src-tauri/src/credentials.rs` (if still present after Task 3 — they were `git rm`'d in the working tree, so likely already gone; verify)
- Delete: `src-tauri/icons/android/` (whole tree), `src-tauri/icons/ios/` (whole tree)
- Move: `Claude Bar.html` → `docs/design/canvas-2026-09.html`
- Move: `docs/compact.png` → `docs/screenshots/compact.png`; `docs/panel.png` → `docs/screenshots/panel.png`; `docs/tray.png` → `docs/screenshots/tray.png`
- Copy: `src-tauri/icons/icon.ico` → `docs/design/legacy-icon.ico`
- Modify: any file that references the moved screenshot paths (`README.md` — check for `docs/panel.png` etc.)

**Interfaces:**
- Consumes: the committed baseline (Task 3)
- Produces: a repo with no stray root files, no unused platform icon trees, screenshots under `docs/screenshots/`. No code symbols change.

- [ ] **Step 1: Verify the dead Rust files are gone**

Run: `ls src-tauri/src/claude_api.rs src-tauri/src/credentials.rs 2>/dev/null || echo "already deleted"`
If either still exists: `git rm src-tauri/src/claude_api.rs src-tauri/src/credentials.rs`

- [ ] **Step 2: Move the design canvas and screenshots**

```bash
mkdir -p docs/design docs/screenshots
git mv "Claude Bar.html" docs/design/canvas-2026-09.html
git mv docs/compact.png docs/screenshots/compact.png
git mv docs/panel.png docs/screenshots/panel.png
git mv docs/tray.png docs/screenshots/tray.png
cp src-tauri/icons/icon.ico docs/design/legacy-icon.ico
git add docs/design/legacy-icon.ico
```

- [ ] **Step 3: Delete the unused platform icon trees**

```bash
git rm -r src-tauri/icons/android src-tauri/icons/ios
```

- [ ] **Step 4: Fix screenshot references**

Grep: `grep -rn "docs/compact\|docs/panel\|docs/tray\|(compact.png\|(panel.png\|(tray.png" README.md index.html`
For each hit, update the path to `docs/screenshots/<name>.png`.

- [ ] **Step 5: Verify the build is unaffected**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
npm run build
```
Expected: both pass. `tauri.conf.json` `bundle.icon` must still list only files that exist (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `icon.icns` — all under `src-tauri/icons/`, none under android/ios). If `tauri.conf.json` or `Cargo.toml` references an android/ios path, fix it.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: rename cleanup — move design canvas + screenshots, drop unused platform icon trees

The Tauri-generated android/ios icon directories are removed from the
Windows desktop app; future companion apps carry their own assets.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 5: Sweep remaining "Claude Bar" / "daybi" references

**Files:**
- Modify: whatever the grep in Step 1 turns up, except `NOTICE`, `README.md`, `LICENSE`, `docs/design/`, `docs/superpowers/`

**Interfaces:**
- Consumes: Task 4's tree
- Produces: no stale product-name / old-identifier strings in code, config, or UI copy.

- [ ] **Step 1: Find them**

Run:
```bash
grep -rniE 'claude bar|claudebar|com\.daybi|daybi' \
  --include='*.rs' --include='*.ts' --include='*.json' --include='*.html' \
  --include='*.toml' --include='*.css' --include='*.yml' . \
  | grep -vE 'node_modules|NOTICE|README|LICENSE|docs/design|docs/superpowers'
```
Expected: a short list. Likely spots: `src-tauri/tauri.conf.json` (should already be `IA Usage Bar` / `com.alberth.iausagebar` — verify), window `title`, `Cargo.toml` `description`, any `MenuItem::with_id(app, "tray_header", "…")` string in `lib.rs`, comments.

- [ ] **Step 2: Fix each hit**

- Product name in UI / config / menu → `IA Usage Bar`.
- `com.daybi.claudebar` → `com.alberth.iausagebar`.
- Comments referencing the old name → update or delete.
- Do **not** touch attribution sentences in `NOTICE` / `README` / `LICENSE`.

- [ ] **Step 3: Verify**

Run the Step 1 grep again. Expected: empty (or only attribution lines you deliberately kept). Then:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
npm run build
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: replace remaining Claude Bar / daybi references with IA Usage Bar

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 6: Split `src-tauri/src/lib.rs` into focused modules

**Files:**
- Create: `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/dashboard.rs`
- Rename: `src-tauri/src/tray_icon.rs` → `src-tauri/src/tray.rs` (and grow it)
- Modify: `src-tauri/src/lib.rs` (down to `mod` declarations + `pub fn run()`)
- Modify: `src-tauri/src/main.rs` if it names `tray_icon` (it calls `iausagebar_lib::run()` — check)
- Test: existing `#[cfg(test)] mod tests` in `lib.rs` moves with the code it exercises

**Interfaces:**
- Consumes: Task 5's tree
- Produces (module paths other code/tests import):
  - `crate::state::{AppState, TrayMenuState, NotifyState, UsageAlert}`
  - `crate::commands::*` (the 11 `#[tauri::command]` fns + `parse_id`)
  - `crate::dashboard::{build_dashboard, emit_dashboard, do_refresh, refresh_sync, run_loop, check_notifications, take_usage_alerts, reset_happened, parse_ts}`
  - `crate::tray::{render, on_tray_left_click, position_window, show_window, tray_percent, tooltip, update_tray_from_dashboard}`
  - `lib.rs` keeps only `pub fn run()` and `mod` lines.
- **No signature changes.** Every fn keeps its exact name, params, and return type. Visibility widens from private to `pub(crate)` where a fn is now used across modules.

- [ ] **Step 1: Baseline the suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20`
Record the pass count. This number must not drop for the rest of the task.

- [ ] **Step 2: Rename `tray_icon.rs` → `tray.rs`**

```bash
git mv src-tauri/src/tray_icon.rs src-tauri/src/tray.rs
```
In `lib.rs` change `mod tray_icon;` → `mod tray;` and every `tray_icon::` → `tray::`. In `main.rs`, if `tray_icon` appears, update it. Run `cargo check --manifest-path src-tauri/Cargo.toml` — expected: pass.

- [ ] **Step 3: Create `state.rs`, move the state types**

Cut `struct AppState`, `struct TrayMenuState`, `struct NotifyState`, and the `UsageAlert` type (search `lib.rs` for `UsageAlert` — it is returned by `take_usage_alerts`) plus their `impl` blocks from `lib.rs` into a new `src-tauri/src/state.rs`. Add `use` lines it needs (`std::sync::*`, `std::collections::HashMap`, `std::time::Instant`, `crate::config::AppConfig`, `crate::model::*`). Mark each type `pub(crate)` and its fields `pub(crate)` where accessed from other modules. In `lib.rs` add `mod state;` and `use state::{AppState, TrayMenuState, NotifyState, UsageAlert};`.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: pass (fix visibility errors by widening to `pub(crate)`).

- [ ] **Step 4: Create `tray.rs` behaviour fns**

Move `on_tray_left_click`, `position_window`, `show_window`, `tray_percent`, `tooltip`, `update_tray_from_dashboard` from `lib.rs` into `tray.rs` (which already holds `render`). Move the `mod tests` fn `tray_percent_prefers_primary` (and any other tray test) into `tray.rs`'s test module. Add `use` lines. Mark the fns `pub(crate)`.

Run: `cargo check` then `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: check passes; test count matches Step 1.

- [ ] **Step 5: Create `dashboard.rs`**

Move `build_dashboard`, `emit_dashboard`, `do_refresh`, `refresh_sync`, `run_loop`, `check_notifications`, `take_usage_alerts`, `reset_happened`, `parse_ts` into `src-tauri/src/dashboard.rs`. Move any `mod tests` fns that exercise these (e.g. tests for `take_usage_alerts` / `reset_happened`) into `dashboard.rs`'s test module. Add `mod dashboard;` to `lib.rs`.

Run: `cargo check` then `cargo test`
Expected: check passes; test count matches Step 1.

- [ ] **Step 6: Create `commands.rs`**

Move the 11 `#[tauri::command]` fns + `parse_id` into `src-tauri/src/commands.rs`. Add `mod commands;` to `lib.rs`. In `lib.rs`'s `tauri::generate_handler![…]` macro call, prefix each command with `commands::` (or add `use commands::*;`). `#[tauri::command]` fns must be at least `pub(crate)`.

Run: `cargo check` then `cargo test`
Expected: check passes; test count matches Step 1.

- [ ] **Step 7: Confirm `lib.rs` is now thin**

`lib.rs` should contain only: the `mod` lines, top-level `use` lines still needed by `run()`, and `pub fn run()`. Run `wc -l src-tauri/src/lib.rs` — expected: well under 200 lines. Anything else still in there is a fn that belongs in one of the four modules — move it.

- [ ] **Step 8: Full verification**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
npm run build
```
Expected: all pass; test count == Step 1.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: split lib.rs into state / commands / dashboard / tray modules

Pure move — no signature or behaviour change. lib.rs now holds only the
Tauri builder wiring. tray_icon.rs renamed to tray.rs.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 7: Split `src/main.ts` into `api.ts` / `i18n.ts` / `views/`

**Files:**
- Create: `src/api.ts`, `src/i18n.ts`, `src/views/dash.ts`, `src/views/settings.ts`, `src/views/spend.ts`
- Modify: `src/main.ts` (down to bootstrap + routing + global listeners)
- Modify: `index.html` only if the `<script type="module" src="/src/main.ts">` tag path changes (it should not)

**Interfaces:**
- Consumes: Task 6's tree (frontend untouched by Task 6)
- Produces (ES module exports):
  - `api.ts`: `export function isTauri(): boolean`, `export async function invokeCmd<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null>`, and `export type` / `export interface` for `MetricLine`, `ProviderSnapshot`, `VendorInfo`, `SpendRow`, `Dashboard`
  - `i18n.ts`: `export const I18N: Record<string, Record<string, string>>`, `export let lang: "es" | "en"`, `export function setLang(l: "es" | "en"): void`, `export function t(k: string): string`
  - `views/dash.ts`: `export function renderDash(dash: Dashboard | null, selectedId: string): void` plus its private helpers (not exported)
  - `views/settings.ts`: `export function renderSettings(dash: Dashboard | null): void`, `export async function persistConfig(...): Promise<void>` (keep its current signature)
  - `views/spend.ts`: `export function renderSpend(dash: Dashboard | null): void`, `export function previewDashboard(): Dashboard`
  - `main.ts`: no exports; imports from all of the above.
- **No behaviour change.** `t()` must resolve identically (same `lang` source of truth — `i18n.ts` owns the `localStorage.getItem("lang")` read now; `main.ts` calls `setLang` on the EN/ES buttons).

- [ ] **Step 1: Establish the frontend smoke baseline**

Run: `npm run build`
Record: it emits `dist/` with no TypeScript errors. Open `npm run tauri dev`, click through: dashboard tab switch, open Settings, open Spend, toggle EN/ES, toggle a provider. Note current behaviour — this is the parity target.

- [ ] **Step 2: Extract `api.ts`**

Move the `type MetricLine`, `interface ProviderSnapshot`, `interface VendorInfo`, `interface SpendRow`, `interface Dashboard`, `function isTauri`, `async function invokeCmd` out of `main.ts` into `src/api.ts`, each `export`ed. In `main.ts` add `import { isTauri, invokeCmd, type Dashboard, type ProviderSnapshot, type VendorInfo, type MetricLine, type SpendRow } from "./api";`.

Run: `npm run build`
Expected: no TS errors.

- [ ] **Step 3: Extract `i18n.ts`**

Move `I18N`, the `let lang = …` initializer, and `const t = …` into `src/i18n.ts`. Convert to:
```ts
export const I18N: Record<string, Record<string, string>> = { /* … */ };
export let lang: "es" | "en" = localStorage.getItem("lang") === "en" ? "en" : "es";
export function setLang(l: "es" | "en") { lang = l; localStorage.setItem("lang", l); }
export function t(k: string): string { return I18N[lang][k] ?? k; }
```
In `main.ts` and wherever `I18N[lang]` / `t(` / `lang` is used, add `import { t, I18N, lang, setLang } from "./i18n";`. The EN/ES button handler in `handleAction` calls `setLang("en" | "es")` instead of assigning `lang` directly.

Run: `npm run build`
Expected: no TS errors. (Watch for `lang` used as a value in other soon-to-move files — those imports get added in Steps 4–6.)

- [ ] **Step 4: Extract `views/dash.ts`**

Move `renderDash`, `detailHtml`, `progressBlock`, `addedProviders`, `pctOf`, `windowLabel`, `resetCountdown`, `exactReset`, `paceNote`, `counts`, `glyph`, `accent`, `tabName`, `escapeHtml`, and the `ACCENT` / `TAB_NAME` maps into `src/views/dash.ts`. Export only `renderDash`. It currently reads module globals `dash`, `selectedId`, `view` from `main.ts` — change `renderDash` to take `(dash: Dashboard | null, selectedId: string)` as params; the `view` class-toggle lines (`$("view-dash").classList.toggle(...)`) stay in `main.ts`'s router, not in `renderDash`. Add imports (`./api`, `./i18n`).

If `escapeHtml` is also needed by `views/settings.ts` or `views/spend.ts`, export it from `api.ts` instead and import it in all three.

Run: `npm run build`
Expected: no TS errors.

- [ ] **Step 5: Extract `views/settings.ts`**

Move `renderSettings` and `persistConfig` into `src/views/settings.ts`. `renderSettings` takes `(dash: Dashboard | null)`. Keep `persistConfig`'s current signature and behaviour. Add imports.

Run: `npm run build`

- [ ] **Step 6: Extract `views/spend.ts`**

Move `renderSpend` and `previewDashboard` into `src/views/spend.ts`. `renderSpend` takes `(dash: Dashboard | null)`. Add imports.

Run: `npm run build`

- [ ] **Step 7: Reduce `main.ts` to bootstrap + routing**

What remains in `main.ts`: `const $ = …`, the module state (`let dash`, `let selectedId`, `let view`), `applyDashboard`, `fitWindow`, `handleAction`, the two `document.addEventListener` blocks, `main()`, and a `render()` router that calls `renderDash(dash, selectedId)` / `renderSettings(dash)` / `renderSpend(dash)` plus the `view-*` class toggles. All imported from the new modules.

Run: `wc -l src/main.ts` — expected: roughly 200 lines or fewer.

- [ ] **Step 8: Full parity check**

Run: `npm run build` then `npm run tauri dev`. Repeat every interaction from Step 1. Expected: identical behaviour — same rendering, same EN/ES switch, same Settings/Spend, same provider toggle, same window auto-fit.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: split main.ts into api / i18n / views modules

Pure move — no behaviour change. main.ts now holds bootstrap, routing,
and global listeners only.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 8: Add `cargo test` to CI

**Files:**
- Modify: `.github/workflows/build.yml`

**Interfaces:**
- Consumes: nothing structural
- Produces: CI that runs the Rust test suite on every push / PR.

- [ ] **Step 1: Add the step**

In `.github/workflows/build.yml`, after the `Check Tauri backend` step, add:
```yaml
      - name: Test Tauri backend
        run: cargo test --manifest-path src-tauri/Cargo.toml --locked
```

- [ ] **Step 2: Verify locally**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked`
Expected: pass (matches Task 6 Step 1 count).

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/build.yml
git commit -m "ci: run cargo test on push and PR

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SKkQUXpcbrQz9iThtF8tJL"
```

### Task 9: Local folder + GitHub repo rename — CHECKPOINT

**Files:** none (repo/host operations + one config line)

**Interfaces:**
- Consumes: all prior tasks committed and pushed
- Produces: the project living at `ia-usage-bar` locally and on GitHub, `origin` updated.

- [ ] **Step 1: Push everything**

```bash
git push origin main
```
Expected: the Phase 0 + Phase 1 commits land on `github.com/Shadelight/claude-bar-windows`.

- [ ] **Step 2: User renames the GitHub repo**

The user, in the GitHub web UI: Settings → rename `claude-bar-windows` → `ia-usage-bar`. Then set the repo description to `Windows tray monitor for AI plan usage — quota, reset, and spend across Claude, Codex, Cursor, and more` and add topics `tauri`, `rust`, `windows`, `tray`, `claude`, `openai`, `usage-monitor`. (Topics/description can also wait for Part 2, Phase 5 — note which was done.)

- [ ] **Step 3: Update the local remote**

```bash
git remote set-url origin https://github.com/Shadelight/ia-usage-bar.git
git remote -v
git fetch origin
```
Expected: `origin` points at `ia-usage-bar`, fetch succeeds.

- [ ] **Step 4: User renames the local folder**

The user closes any editor/terminal holding `D:\Users\Escritorio\Claude Bar`,
renames it to `D:\Users\Escritorio\IA Usage Bar`, and reopens. The agent
resumes with the working directory updated. `git status` from the new path
must show a clean tree.

- [ ] **Step 5: CHECKPOINT — end of Part 1**

Confirm: `cargo test`, `cargo check`, `npm run build`, `npm run tauri dev`
smoke all green from the renamed folder; `git log --oneline` shows the
Phase 0–1 commits; `git status` clean. Part 1 is done. Proceed to writing
Part 2 (Phases 2–5) using the audit findings in
`docs/design/phase-0-audit.md`.

---

## Self-Review

**Spec coverage (Phases 0–1 only):**

| Spec item (Phase 0–1) | Task |
|---|---|
| Read full diff module by module, findings list | Task 1 |
| Fix confirmed correctness bugs, defer style items | Task 2 |
| Fixture tests for provider parsing + reset/percentage | Task 2 (per-fix regression tests); broader fixture suite → **Part 2 Phase 2** (noted in spec under Phase 2 "Tests") |
| User runs app; commit squash base | Task 3 |
| Delete `claude_api.rs` / `credentials.rs` | Task 4 Step 1 |
| Move `Claude Bar.html` → `docs/design/` | Task 4 Step 2 |
| `git mv docs/*.png docs/screenshots/` | Task 4 Step 2 |
| Delete android/ios icon trees; keep old `.ico` in `docs/design/` | Task 4 Steps 2–3 |
| Local folder rename; GitHub repo rename; update `origin` | Task 9 |
| Grep for `Claude Bar` / `claudebar` / `daybi` / `com.daybi` | Task 5 |
| Commit `NOTICE`, `cspell.json`, `src/vite-env.d.ts` | Task 3 Step 1 (staged with baseline) |
| Backend split: `lib.rs` → `commands`/`state`/`dashboard`; `tray_icon.rs` → `tray.rs` | Task 6 |
| Frontend split: `api.ts` / `i18n.ts` / `views/*` | Task 7 |
| CI gains `cargo test` | Task 8 |
| `model.rs` split | **Part 2 Phase 2** (spec explicitly defers it there) |

**Deferred to Part 2 (in the spec's Phases 2–5, not gaps):** `ProviderDescriptor` + normalized `ProviderSnapshot`/`Metric` contract, per-provider module folders, architecture-guard tests, dark design system + states + icon regeneration, launch-at-login, configurable notification thresholds, community docs, `release.yml`, the orphan squash + `archive/pre-ia-usage-bar-squash` tag.

**Placeholder scan:** Task 2's code is illustrative because the real findings do not exist until Task 1 runs — this is called out explicitly in the task, and Task 1's deliverable (the audit doc) is what supplies the concrete content, per the spec's own Phase 0 → Phase 2 flow. No other task contains TBD/TODO/"handle edge cases"/uncoded steps.

**Type consistency:** `renderDash(dash, selectedId)`, `renderSettings(dash)`, `renderSpend(dash)` signatures are used consistently in Task 7's Interfaces block and Steps 4–7 and the Step 7 router description. `crate::tray::` (not `tray_icon::`) used consistently after Task 6 Step 2. `pub(crate)` visibility rule stated once and applied per module. `invokeCmd<T>(...) => Promise<T | null>` matches the current signature at `src/main.ts:205`.

## Execution Handoff

See the top-of-file banner. After the user approves this plan, offer:

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks.
2. **Inline Execution** — tasks in this session with checkpoints.

Task 1 (audit) is investigation-heavy and its output gates everything else — run it and review its findings doc carefully before Task 2, regardless of execution mode.
