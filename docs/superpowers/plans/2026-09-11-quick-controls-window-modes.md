# Quick Controls, Window Modes & Settings Reorganization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> Executed inline in the authoring session (full codebase context already
> loaded) rather than dispatched — see spec for rationale on scope choices.

**Goal:** Replace the unclear header icons, add pin/compact window modes, a
real provider-links registry, a categorized Settings screen, a minimal log
file, and an "Acerca de" panel — without a new Cargo/npm dependency.

**Architecture:** `AppConfig` gains two persisted booleans
(`always_on_top`, `compact_mode`) that flow through `Dashboard` to both the
frontend and the tray, mirroring the existing `autostart`/`notifications`
pattern exactly. `VendorInfo` gains a `links` field populated from a new
per-`VendorId` match in `model.rs`. A new `logfile.rs` module appends
provider-failure lines to a capped file. The frontend gets a header dropdown,
a compact CSS mode, and `settings.ts` rewritten around categories.

**Tech Stack:** Rust/Tauri 2 backend (existing crates only), TypeScript
frontend (existing `@tauri-apps/api` only), vanilla CSS.

**Spec:** `docs/superpowers/specs/2026-09-11-quick-controls-window-modes.md`

## Global Constraints

- No new Cargo or npm dependency.
- Never fabricate a provider URL — only Anthropic, OpenAI, Copilot, Cursor,
  OpenRouter, DeepSeek, Groq get real links; everyone else gets `None`.
- Follow the existing `autostart` pattern for any new persisted toggle that
  the tray also needs to show (`AppConfig` field → command → `TrayMenuState`
  `CheckMenuItem` → menu-event handler).
- UI copy ships in both `es` and `en` in `i18n.ts` — never a bare string in
  a view file.
- `NOTICE`/`LICENSE`/`README.md` are not touched by this plan.

---

### Task 1: `AppConfig` — `always_on_top` and `compact_mode`

**Files:**
- Modify: `src-tauri/src/config.rs`

**Interfaces:**
- Produces: `AppConfig.always_on_top: bool`, `AppConfig.compact_mode: bool`
  (both `#[serde(default = ...)]`, defaults `true`/`false`).

- [ ] Add `default_always_on_top() -> bool { true }` and reuse the existing
  `default_true`/default pattern; add both fields to the struct, `Default`
  impl, and `normalize()` is a no-op for them (plain booleans, nothing to
  clamp).
- [ ] Test: extend `normalize_clamps_refresh_and_notification_thresholds` — or
  add `defaults_match_current_window_behavior()` asserting
  `AppConfig::default().always_on_top == true` and `.compact_mode == false`
  (this is the regression guard: shipping this must not change the window's
  current always-on-top behavior for existing users, since `config.toml`
  written before this change has no such key).
- [ ] Run: `cd src-tauri && cargo test config::` — expect PASS.
- [ ] Commit.

### Task 2: Minimal capped log file

**Files:**
- Create: `src-tauri/src/logfile.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod logfile;`)
- Modify: `src-tauri/src/dashboard.rs` (call it from the refresh-error path)

**Interfaces:**
- Produces: `logfile::append(line: &str)`, `logfile::dir() -> PathBuf`,
  `logfile::clear()`.

- [ ] `append` opens `paths::app_config_dir().join("logs").join("app.log")`
  in append mode (create dir/file as needed), writes
  `"{timestamp} {line}\n"`; if the file exceeds 2000 lines, rewrite it
  keeping only the last 2000 (read, split, truncate, rewrite — plain
  `std::fs`, no crate).
- [ ] `dir()` returns the `logs` folder path; `clear()` truncates the file
  (`fs::write(path, b"")`, ignoring a missing file).
- [ ] In `dashboard.rs`, wherever a provider snapshot lands in
  `ProviderStatus::Error | NeedsAuth | NeedsPermission | Unavailable` during
  `do_refresh`, call
  `logfile::append(&format!("{} {:?}", snapshot.id, snapshot.status_reason))`.
- [ ] Test (in `logfile.rs`, `#[cfg(test)] mod tests`, scratch dir like
  `config.rs`'s): `append_trims_to_last_2000_lines()` — write 2100 lines,
  assert the file has exactly 2000 and the first surviving line is line 101.
- [ ] Run: `cargo test logfile::` — expect PASS.
- [ ] Commit.

### Task 3: Commands — window modes, logs, diagnostics

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/model.rs` (`Dashboard` gains `always_on_top`,
  `compact_mode`)
- Modify: `src-tauri/src/dashboard.rs` (`build_dashboard` populates them)
- Modify: `src-tauri/src/lib.rs` (register new commands, apply
  `always_on_top` to the window at startup, register handler)

**Interfaces:**
- Consumes: `logfile::{dir, clear}` (Task 2), `AppConfig.always_on_top` /
  `.compact_mode` (Task 1).
- Produces: commands `set_always_on_top(enabled)`, `set_compact_mode(enabled)`,
  `open_logs_folder()`, `clear_logs()`, `export_diagnostics(app, state)`.

- [ ] `set_always_on_top`: persist on `AppConfig`, save, call
  `win.set_always_on_top(enabled)` on the `"main"` window, update
  `TrayMenuState` checkbox (added in Task 4 — for now store the field and
  leave the tray sync as a no-op if `try_state` misses, same defensive style
  already used for `autostart`).
- [ ] `set_compact_mode`: persist + save + sync tray checkbox the same way;
  no window-size change here (frontend's `fitWindow` reacts to the
  `Dashboard.compactMode` field on the next `dashboard-updated` event).
- [ ] `open_logs_folder`: `std::process::Command::new("explorer").arg(logfile::dir())`
  (Windows-only app, matches the rest of the codebase's Windows-first
  paths.rs).
- [ ] `clear_logs`: calls `logfile::clear()`.
- [ ] `export_diagnostics`: builds a `serde_json::json!` object with
  `app_version` (`app.package_info().version.to_string()`), `os` (`"windows"`
  literal — matches the codebase's Windows-only scope), and for each
  snapshot in `state.snapshots`: `id`, `status`, `status_reason`,
  `updated_at`. Writes it to `logfile::dir().join(format!("diagnostics-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S")))`
  then runs the same `explorer` reveal as `open_logs_folder`. No API keys,
  tokens, or HTTP bodies are in scope, so nothing to redact beyond what's
  already excluded from `ProviderSnapshot`.
- [ ] `Dashboard` gains `always_on_top: bool` and `compact_mode: bool`
  (`#[serde(rename_all = "camelCase")]` already on the struct handles the
  casing); `build_dashboard` copies them straight from the locked
  `AppConfig`.
- [ ] In `lib.rs`'s `.setup()`, after the window is fetched
  (`app.get_webview_window("main")`), call
  `let _ = win.set_always_on_top(cfg.always_on_top);` before `win.show()`.
- [ ] Add all five new commands to `tauri::generate_handler![...]`.
- [ ] Run: `cargo build` (no dedicated unit test for I/O-and-window glue code
  — matches this file's existing convention of zero tests in
  `commands.rs`; Task 1/2's unit tests are the coverage for the logic these
  commands call). Expect a clean build.
- [ ] Commit.

### Task 4: Tray — pin/compact checkboxes + primary-provider submenu

**Files:**
- Modify: `src-tauri/src/state.rs` (`TrayMenuState` gains
  `always_on_top: CheckMenuItem<Wry>`, `compact_mode: CheckMenuItem<Wry>`,
  `primary_submenu: Submenu<Wry>`)
- Modify: `src-tauri/src/lib.rs` (build the two checkboxes + submenu, wire
  `on_menu_event`)
- Modify: `src-tauri/src/commands.rs` (`refresh_catalog` also rebuilds the
  primary submenu)

**Interfaces:**
- Consumes: `AppConfig.always_on_top/.compact_mode` (Task 1), commands from
  Task 3.
- Produces: `pub(crate) fn rebuild_primary_submenu(app: &AppHandle, catalog: &[VendorInfo], primary: &str)`
  in `tray.rs`, called from both `lib.rs setup()` and
  `commands::refresh_catalog`.

- [ ] Add `CheckMenuItem::with_id(app, "pin", "Siempre visible", true, cfg.always_on_top, None::<&str>)`
  and the same shape for `"compact"` / "Modo compacto", inserted into the
  existing `Menu::with_items` list (after `detect_i`, before `settings_i`,
  matching the spec's tray order).
- [ ] Build an empty `Submenu::with_id(app, "primary_provider", "Proveedor principal", true)`
  at startup (populated by the shared rebuild function right after), insert
  it into the menu before the pin checkbox.
- [ ] `rebuild_primary_submenu` clears the submenu's items and re-adds one
  `CheckMenuItem` per **enabled** catalog entry, checked iff its id matches
  `primary`; clicking one calls `set_app_config`'s existing primary-switch
  path — reuse `commands::set_app_config`-equivalent logic by calling
  `AppConfig::save` directly with `primary` swapped (small helper
  `commands::set_primary(app, state, id)` is simplest — add it alongside the
  other commands and call it from both the tray handler and, optionally,
  nowhere else for now).
- [ ] `on_menu_event`: `"pin"` toggles `always_on_top` (flip current
  `CheckMenuItem::is_checked()`, call the same logic as
  `commands::set_always_on_top` — extract that logic into a plain function
  `commands::apply_always_on_top(app, state, enabled)` that both the Tauri
  command and the tray handler call, avoiding duplicated persistence code);
  `"compact"` mirrors it with `apply_compact_mode`.
- [ ] Run: `cargo build`. Expect a clean build (menu/window glue, no new unit
  test — same rationale as Task 3).
- [ ] Commit.

### Task 5: `VendorLinks` registry

**Files:**
- Modify: `src-tauri/src/model.rs` (add `VendorLinks` struct +
  `VendorId::links(self) -> VendorLinks`)
- Modify: `src-tauri/src/providers/mod.rs` (`catalog()` populates
  `VendorInfo.links`)
- Modify: `src/api.ts` (add `VendorLinks` interface + `links` field on
  `VendorInfo`)

**Interfaces:**
- Produces: `VendorLinks { usage_url: Option<String>, billing_url: Option<String>, status_url: Option<String> }`,
  camelCase-serialized like the rest of `model.rs`.

- [ ] `VendorId::links(self) -> VendorLinks` — a `match` with real URLs only
  for: `Anthropic`/`AnthropicApi` → usage `https://console.anthropic.com/settings/usage`,
  status `https://status.anthropic.com`; `Openai`/`OpenaiAdmin` → usage
  `https://platform.openai.com/usage`, status `https://status.openai.com`;
  `Copilot` → billing `https://github.com/settings/billing`, status
  `https://www.githubstatus.com`; `Cursor` → usage
  `https://cursor.com/dashboard`; `Openrouter` → usage
  `https://openrouter.ai/activity`; `Deepseek` → usage
  `https://platform.deepseek.com/usage`; `Groq` → usage
  `https://console.groq.com/dashboard/usage`. Every other `VendorId` →
  `VendorLinks { usage_url: None, billing_url: None, status_url: None }`
  (the `_ =>` arm — no per-provider guessing).
- [ ] `catalog()` sets `links: id.links()` on each `VendorInfo`.
- [ ] Test: `vendor_links_never_guessed_for_unlisted_providers()` — assert
  e.g. `VendorId::Kiro.links().usage_url.is_none()` and
  `VendorId::Nous.links().status_url.is_none()`; plus
  `known_vendor_links_are_https()` asserting every `Some` URL for the seven
  listed vendors starts with `"https://"`.
- [ ] Run: `cargo test model::` — expect PASS.
- [ ] Update `src/api.ts`: `VendorLinks` interface, `links: VendorLinks` on
  `VendorInfo`.
- [ ] Commit.

### Task 6: Frontend types — `Dashboard` new fields

**Files:**
- Modify: `src/api.ts`

- [ ] Add `alwaysOnTop: boolean` and `compactMode: boolean` to the
  `Dashboard` interface (already covered for `links` in Task 5).
- [ ] Run: `npm run build` (TypeScript compile catches any missed call site).
  Expect a clean build (no consumer reads these fields yet, so nothing
  breaks).
- [ ] Commit.

### Task 7: Header — pin + `⋯` dropdown

**Files:**
- Modify: `index.html` (header markup)
- Modify: `src/main.ts` (dropdown open/close, new `data-act` handlers)
- Modify: `src/styles.css` (dropdown menu styles)
- Modify: `src/i18n.ts` (new keys: `pin`, `unpin`, `menu`, `compactMode`,
  `openLogs`, `providerPanel`, `serviceStatus`, `configuration`)

**Interfaces:**
- Consumes: `invokeCmd("set_always_on_top", { enabled })`,
  `invokeCmd("set_compact_mode", { enabled })`,
  `invokeCmd("open_logs_folder")`, `Dashboard.alwaysOnTop`,
  `Dashboard.compactMode`, `VendorInfo.links` (all from Tasks 3/5/6).

- [ ] Replace the `#btn-spend` and settings `icon-btn` in `index.html`'s
  `.head-actions` with a pin `button.icon-btn[data-act=toggle-pin]` (a
  pin-shaped SVG, filled when active) and a `⋯` `button.icon-btn[data-act=toggle-menu]`
  with an SVG ellipsis, followed by a `<div id="head-menu" class="head-menu hidden">`
  containing the six `data-act` rows from the spec (`compact`, `notif`
  toggle, `spend`, `provider-panel`, `service-status`, `open-logs`,
  `settings`) — each rendered by `main.ts` (item text via `t()`, links open
  via `window.open` is unavailable in a Tauri webview without the shell
  plugin, so provider/status links use `invokeCmd`-free
  `import("@tauri-apps/api/core")`... actually simplest: add
  `core:app:default`? No — Tauri 2's webview already lets an `<a target="_blank">`
  open the OS browser for `https://` links with no extra permission when
  the link is a normal anchor tag; use `<a href="..." target="_blank">` for
  provider/status rows instead of JS, matching the zero-extra-permission
  goal in the spec).
- [ ] `main.ts`: `handleAction` gains `"toggle-menu"` (toggle `#head-menu`
  visibility, close on outside click / next action), `"toggle-pin"` (flip
  local `alwaysOnTop` bool from `dash`, call
  `invokeCmd("set_always_on_top", { enabled })`, update the button's
  `.active` class), `"compact"` (same shape calling `set_compact_mode`),
  `"open-logs"` (`invokeCmd("open_logs_folder")`).
- [ ] `paintDash` (or a new small `paintMenu`) rebuilds the menu's
  provider-panel/service-status rows from the *currently selected* vendor's
  `links`, hiding a row entirely when its URL is `null` (per spec — no
  "No disponible" dead menu item).
- [ ] Manual verify (no unit-test infra exists for `main.ts`'s DOM wiring,
  matching this file's current test coverage): `npm run dev` under Tauri,
  click pin (window stays on top / releases), click `⋯` (menu opens/closes
  on outside click), click each menu row.
- [ ] Commit.

### Task 8: Compact mode CSS

**Files:**
- Modify: `src/styles.css`
- Modify: `src/main.ts` (`fitWindow` compact sizing, apply `.compact` class
  to `#panel` from `Dashboard.compactMode`)

- [ ] `.panel.compact .details, .panel.compact .breakdown, .panel.compact .hint, .panel.compact .guide-row, .panel.compact .tab .label, .panel.compact .foot-name, .panel.compact .tab-add { display: none; }`
  plus a tighter `.panel.compact .block { margin: 8px 0 4px; }` so the
  primary quota block doesn't leave dead space once everything below it is
  hidden.
- [ ] `applyDashboard` toggles `document.getElementById("panel").classList.toggle("compact", d.compactMode)`.
- [ ] `fitWindow`: when `dash?.compactMode`, clamp height to
  `Math.min(220, Math.max(160, content))` and width stays 320 (a second
  `LogicalSize` branch) instead of the existing `360×[420,640]`.
- [ ] Manual verify: toggle compact mode, confirm window shrinks and the
  mockup's three visible rows (header/session/weekly + stall banner) match.
- [ ] Commit.

### Task 9: Settings — categorized nav + About panel

**Files:**
- Modify: `index.html` (settings view markup: category rail + body)
- Modify: `src/views/settings.ts` (rewrite around categories)
- Modify: `src/i18n.ts` (category labels, About copy, diagnostics strings)
- Modify: `src/main.ts` (persist selected settings category like `selectedId`)

**Interfaces:**
- Consumes: `invokeCmd("clear_logs")`, `invokeCmd("export_diagnostics")`
  (Task 3), `getVersion()` from `@tauri-apps/api/app`.

- [ ] `index.html`: inside `#view-settings`, add
  `<nav id="settings-cats" class="settings-cats"></nav>` before
  `#settings-body`.
- [ ] `settings.ts`: introduce
  `type SettingsCategory = "general" | "providers" | "notifications" | "appearance" | "data" | "about";`
  and a per-category render function; `renderSettings(dash, category)`
  paints `#settings-cats` (six buttons, `data-cat` attribute) and delegates
  the body to the matching function. **General** = autostart + pin +
  interval + primary (existing markup, moved). **Proveedores** = today's
  provider list + connection guide (existing `guide()`/`logo()` helpers,
  unchanged). **Notificaciones** = existing notif toggle + thresholds.
  **Apariencia** = compact-mode toggle + the existing lang buttons
  duplicated here (footer keeps its own too, per spec). **Datos y
  registros** = three buttons wired to `open_logs_folder`, `clear_logs`,
  `export_diagnostics`. **Acerca de** = name/version (`await getVersion()`)/
  description/author/repo link, static markup, no "Fork de Claude Bar" text
  anywhere.
- [ ] `main.ts`: `handleAction("settings")` keeps its current behavior;
  add a `data-setcat` click handler (mirrors the existing `data-select`
  handler) storing the chosen category in `localStorage` under
  `"settingsCategory"`, read back on next `renderSettings` call the same
  way `selectedId` is read back for the dashboard.
- [ ] `persistConfig` gains the two new fields the General/Appearance
  categories can change (`always_on_top`, `compact_mode`) in the
  `set_app_config` payload it already builds, alongside the untouched
  provider/notification fields.
- [ ] Manual verify: `npm run dev`, open Settings, click through all six
  categories, confirm provider list / API key save / autostart toggle /
  thresholds all still work exactly as before (regression check on existing
  behavior, since this task moves their markup).
- [ ] Commit.

### Task 10: i18n completeness pass

**Files:**
- Modify: `src/i18n.ts`

- [ ] Grep the diff from Tasks 7–9 for every `t("...")` call introduced and
  confirm a matching key exists in both the `es` and `en` blocks (the
  fastest way: `npm run build` — a missing key silently falls back to the
  raw key per `t()`'s `?? k`, so this is a manual `grep -o 't("[a-zA-Z]*")' src/**/*.ts | sort -u`
  cross-check, not a compiler error).
- [ ] Run `npm run test:frontend` — expect PASS (existing pure-logic tests
  unaffected).
- [ ] Commit.

---

## Self-review notes

- Spec coverage: header (Task 7), compact mode (Task 8), pin (Tasks 1/3/4/7),
  provider links (Task 5), Settings categories + About (Task 9), tray
  (Task 4), logs/diagnostics (Tasks 2/3/9) — all covered. Auto-updater and
  the save-file dialog are explicitly out of scope per the spec, so no task
  exists for them.
- No task invents a provider URL; Task 5 enumerates the only seven vendors
  that get one.
- `always_on_top`/`compact_mode` naming is consistent Rust `snake_case` ↔
  TS `camelCase` (`alwaysOnTop`/`compactMode`) across every task that
  touches them (1, 3, 4, 6, 7, 8, 9) — matches the existing
  `refresh_minutes`/`refreshMinutes` convention already in the codebase.
