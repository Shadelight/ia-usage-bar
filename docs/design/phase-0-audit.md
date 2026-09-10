# Phase 0 — Working-tree audit (IA Usage Bar)

**Date:** 2026-09-10
**Branch:** `milestone-0-stabilization`
**Base commit:** `1551d49` (`chore: git-ignore .superpowers`)
**Pre-fork tip for regression comparison:** `497c60d` (`Polish UI and rebuild release artifacts`)
**Scope:** the uncommitted ~2100-line conversion of "Claude Bar" into the multi-provider "IA Usage Bar", i.e. `git diff` against `1551d49` plus every untracked file under `src-tauri/src/` and `src/`.

This document is read-only output. **No code was changed by this task.** It is the binding
spec for Task 2, which consumes the `high` + `confirmed` rows. Everything else is deferred
to Part 2.

---

## 1. What was audited

| Area | Files | How |
| --- | --- | --- |
| Tray / app shell | `src-tauri/src/lib.rs` (783 ln, ~796 changed) | read in full + diff vs `HEAD` |
| Domain types | `src-tauri/src/model.rs` (569 ln, ~597 changed) | read in full |
| Cost engine | `src-tauri/src/cost.rs`, `src-tauri/src/pricing.rs` | read in full + diff |
| Tray icon renderer | `src-tauri/src/tray_icon.rs` | read in full + diff |
| New infrastructure | `config.rs`, `http.rs`, `jwt.rs`, `paths.rs` (untracked) | read in full |
| Providers | `providers/{mod,claude,codex,cursor,copilot,antigravity,kiro,local,apikey,openai_admin}.rs` (untracked, ~2470 ln) | `mod`/`claude`/`copilot` read in full; `codex`/`cursor`/`kiro`/`antigravity` read at the credential, subprocess and token-refresh paths; all 10 machine-scanned for `unwrap`/`expect`/`let _ =`/indexing/`Command::new`/print macros/credentials-in-`format!` |
| Frontend | `src/main.ts` (765 ln, ~997 changed), `src/styles.css` (~1044 changed), `index.html` (~252 changed), `src/vite-env.d.ts` | `main.ts` read in full; `index.html` cross-checked programmatically against every `$("id")` and `querySelector` in `main.ts`; `styles.css`/`index.html` treated as presentational and not line-audited |
| Build config | `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`, `main.rs`, `package.json` | diff |
| Deleted originals | `git show HEAD:src-tauri/src/claude_api.rs`, `…:credentials.rs` | read in full for regression comparison |

Not audited in depth (deliberate, out of scope for a correctness pass): `src/styles.css`
visual rules, `README.md`, `NOTICE`, `cspell.json`, `Claude Bar.html`, lockfiles.

## 2. Baseline build state

All three gates are **green** on the working tree as it stands:

| Command | Result |
| --- | --- |
| `cargo check --manifest-path src-tauri/Cargo.toml` | pass, **0 warnings** |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **21 passed, 0 failed** (lib); 0/0 for the bin and doc targets |
| `npm run build` | pass, built in 274 ms, `dist/` emitted |

No `high / confirmed` build-breakage finding is warranted. Note that a clean `cargo check`
is not evidence of correctness here: nearly every failure mode below is a swallowed
`Result`, which compiles perfectly.

### Manual smoke test — PENDING (controller to run with user)

Step 4 of the brief asks the user to exercise the real app. That cannot be done from this
task (no GUI, no user in the loop). The controller should run `npm run tauri dev` with the
user and confirm:

1. The tray icon paints a number, and its ring changes colour as the percentage crosses 40 / 70 / 90.
2. Claude Code shows **session %**, **weekly %**, and the four cost rows (`Hoy (API eq.)`, `Semana`, `30 días`, `Mes`). This is the specific pre-fork feature set that must not have regressed.
3. Each provider the user actually has logged in shows data rather than a hint string.
4. **F-M12:** whether the panel force-opens on Windows login and whether it can be dismissed without the tray (it has no title bar and no taskbar entry).
5. **F-H1:** after leaving the app running past a Codex token expiry (>1 h), whether `codex` CLI still authenticates. This is the single highest-risk behaviour in the tree.
6. **F-M6:** whether a console window flashes every refresh cycle when GitHub Copilot is enabled without `GITHUB_COPILOT_TOKEN` set.

## 3. Findings

Severity: `high` = data loss, credential damage, or a permanently dead runtime path.
`med` = wrong or invisible behaviour a user will hit. `low` = latent, cosmetic, or dead code.
Status: `confirmed` = verified by reading the code path end to end. `suspected` = the code
reads that way but confirming it needs the running app or a provider account.

| id | area | severity | status | file:line | description | fix approach |
| --- | --- | --- | --- | --- | --- | --- |
| **F-H1** | credentials / data loss | high | confirmed | `src-tauri/src/providers/codex.rs:179-197` (called from `:175`, `:39-43`) | `write_back` rewrites the **user's own Codex CLI credential file** (`%USERPROFILE%\.codex\auth.json`, or `$CODEX_HOME\auth.json`) after every token refresh. Three compounding defects: (a) `std::fs::write` truncates-then-writes, so a crash or a concurrent refresh mid-write leaves a corrupt file and the user must re-run `codex login`; (b) `serde_json::to_vec_pretty(&root).unwrap_or_default()` yields an **empty `Vec` on serialization failure**, which then truncates `auth.json` to **zero bytes**; (c) the whole call is `let _ = …`, so a failed write — very likely on Windows if the Codex CLI holds the file — is invisible, and because OpenAI rotates `refresh_token` (`:160-163`) the new token is lost while the on-disk one is now spent, silently breaking the user's CLI login. Separately the write is **lossy**: `Tokens` (`:68-78`) has no `#[serde(flatten)]`, so any field the CLI stores inside `tokens` that this struct does not name is dropped on rewrite. | In `codex.rs`: (1) write atomically — serialize to `Vec<u8>` first, `?`-propagate the serialization error instead of `unwrap_or_default()`, write to `auth.json.tmp` in the same directory, then `fs::rename` over the original; (2) change `write_back` to return `Result<(), FetchError>` and surface a failure into the snapshot's `error` rather than `let _ =`; (3) add `#[serde(flatten)] extra: serde_json::Map<String, Value>` to `Tokens` and re-emit it, so unknown fields survive; (4) hold the `refresh_sync` overlap guard from F-H3 so two threads cannot write concurrently. **Test:** unit tests in `codex.rs` for (i) a round-trip `read_auth` → `write_back` over a fixture `auth.json` containing an unknown field inside `tokens`, asserting the field survives, and (ii) `write_back` into a directory that does not exist, asserting it returns `Err` and leaves no partial file. |
| **F-H2** | swallowed errors / data loss | high | confirmed | `src-tauri/src/config.rs:72-88` and `:137-158`; triggered from `src-tauri/src/lib.rs:529-530` | `AppConfig::load()` does `fs::read_to_string(&path).ok().and_then(\|s\| toml::from_str(&s).ok()).unwrap_or_default()`. A `config.toml` that exists but fails to parse — corrupt file, partial write, a future schema change — is silently replaced by `AppConfig::default()`. Startup then calls `config::run_detect(&mut cfg)` (`lib.rs:530`), which **unconditionally** ends with `let _ = cfg.save();` (`config.rs:156`), overwriting the file on disk. Net effect: one unparseable config file permanently **destroys every stored API key** with no message to the user. | In `config.rs`: split `load()` into `try_load() -> Result<AppConfig, String>` and a `load()` wrapper. On a parse error (as distinct from "file absent"), rename the bad file aside to `config.toml.bak-<unix-ts>` before returning the default, and record the reason in a field the dashboard can surface. Make `run_detect` skip its `cfg.save()` when the load was a recovered failure. **Test:** unit test in `config.rs` writing garbage to a temp `config.toml`, calling `try_load`, asserting `Err` and that the original bytes are still recoverable from the `.bak` file. |
| **F-H3** | tray loop / concurrency | high | confirmed | `src-tauri/src/lib.rs:429-434`, `:436-501`, `:503-515`; `refresh_generation` at `:35`, `:499`, `:539` | There is **no overlap guard on `refresh_sync`**. `do_refresh` (`:429`) spawns a detached thread on every call, and it is called from `refresh_now`, `refresh_provider`, `detect_providers`, `set_app_config`, `set_provider_enabled`, `save_api_key`, and the tray `refresh`/`detect` menu items — while `run_loop` (`:503`) is independently calling `refresh_sync` on a timer. Each run fans out to up to 23 further threads (`:452-460`). Toggling several providers in Settings therefore starts several full refreshes at once: duplicated network calls to every provider, and — critically — **concurrent `write_back` calls to the same `auth.json`** (F-H1). The `refresh_generation: AtomicU64` field is incremented at `:499` and **never read anywhere in the tree**, which is direct evidence that this guard was designed and never wired up. | In `lib.rs`: add `refreshing: AtomicBool` (or reuse `refresh_generation` as a claim token) to `AppState`. `refresh_sync` takes the flag with `compare_exchange` at entry and releases it in a guard that runs on unwind; if the flag is already held, `do_refresh` sets a "rerun requested" bit instead of spawning. Delete `refresh_generation` if it is not repurposed, so no dead field remains. **Test:** unit test that drives two `refresh_sync`-shaped closures against the same flag from two threads and asserts the second observes the guard rather than proceeding. |
| **F-H4** | tray loop / panic | high | confirmed | `src-tauri/src/lib.rs:503-515`; poison sources at `:176`, `:177`, `:183`, `:395`, `:438`, `:450`, `:468`, `:474`, `:498`, `:506-512` | `run_loop` is an unguarded `loop { refresh_sync(…); sleep(…) }` on a detached thread with **no `catch_unwind` and no restart**. Every mutex access in the refresh path is `.lock().unwrap()`. A panic anywhere while a lock is held poisons that mutex, and from then on **every** `.lock().unwrap()` on it panics — so a single transient panic kills the refresh thread permanently. The failure is silent: the tray icon keeps painting the last percentage forever, the tooltip never changes, and the user has no signal that refreshing has stopped. Note `refresh_sync` already tolerates *provider* panics gracefully (`if let Ok(…) = h.join()` at `:462` discards a panicked worker) — the gap is the loop thread itself. | In `lib.rs`: (1) wrap the `refresh_sync` call inside `run_loop` in `std::panic::catch_unwind(AssertUnwindSafe(…))` so a panic logs and the loop continues rather than dying; (2) replace `.lock().unwrap()` in `AppState` accesses with a small `fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<T>` that uses `unwrap_or_else(\|e\| e.into_inner())`, so poisoning degrades instead of cascading; (3) on a caught panic, set a visible state (e.g. mark all snapshots `stale`) before the next iteration. **Test:** unit test over the extracted `lock_or_recover` helper — poison a `Mutex` from a panicking thread, then assert the helper still returns the guard. |
| **F-M1** | performance regression | med | confirmed | `src-tauri/src/providers/claude.rs:112-146`, called at `:149` on every `fetch_usage` | The pre-fork `claude_api.rs` memoised the CLI version in `static UA: OnceLock<String>` (`HEAD:src-tauri/src/claude_api.rs:31-35`), so `detect_cli_version()` ran **once per process**. The rewrite dropped the cache: every refresh now does a `max_depth(4)` `walkdir` over `~/.claude/projects`, `metadata()` on every entry, and a full `std::fs::read_to_string` of the newest `.jsonl` (session logs are routinely tens of MB) — just to build a `User-Agent`. Compounded by `cost::compute()` (`cost.rs:136-177`), which walks the *same* tree with no depth limit on the same refresh. → Part 2, Phase 1 (perf). |
| **F-M2** | fragile coupling | med | confirmed | `src-tauri/src/lib.rs:463-472`, string produced at `src-tauri/src/http.rs:20` | Rate-limit backoff is decided by `snap.error.as_deref().is_some_and(\|e\| e.contains("Límite de peticiones"))` — a substring match on a **localized human-readable string**. Any rewording, or the i18n work the milestone plans, silently disables the 5-minute backoff and the app resumes hammering a 429ing endpoint. The structured `FetchError::RateLimited` variant already exists and is discarded on the way through `map_fetch_err`. → Part 2, Phase 1: carry a `rate_limited: bool` on `ProviderSnapshot` (or return `(VendorId, Result<…>)` from the worker) instead of matching prose. |
| **F-M3** | data loss on transient error | med | confirmed | `src-tauri/src/providers/mod.rs:79-96`; `src-tauri/src/lib.rs:473-494`; `src-tauri/src/model.rs:465-479` | `refresh_sync` only preserves the previous snapshot when the new one has `stale == true`. `snapshot_err` (`model.rs:465`) sets `stale: false`, and `map_fetch_err` only flips it for `RateLimited` and `Network`. So a **401/403** (`mod.rs:86-88`), a `Parse` error, or a "not connected" early return (e.g. `claude.rs:32-34` when `.credentials.json` is momentarily unreadable because the CLI is rewriting it) **replaces** the user's last-good data, blanking the tab. → Part 2, Phase 1: preserve the prior snapshot for every non-authoritative failure and mark it stale; only a definitive 401 with no prior data should clear the tab. |
| **F-M4** | swallowed errors / UI | med | confirmed | `src/main.ts:205-209`; call sites `:575`, `:578`, `:598`, `:641`, `:650`, `:555` | `invokeCmd` returns the raw `invoke` promise with no `try`/`catch`. Only one of the seven call sites is guarded (`:666-671`). Every other call is `await invokeCmd(…)` directly inside an async DOM listener, so a backend failure becomes an **unhandled promise rejection** that silently aborts the rest of the handler. Concretely: clicking **Save** on an API key, toggling a provider checkbox, or pressing **Quit** can do nothing at all with no error anywhere the user can see. → Part 2, Phase 2: make `invokeCmd` catch, return `null`, and route the message to a status region. |
| **F-M5** | swallowed errors / UI | med | confirmed | `src/main.ts:666-671`, with `:373`, `:440`, `:492` | If the initial `get_dashboard` rejects, the handler only does `console.error(err)`. `dash` stays `null`, so `renderDash`, `renderSettings` and `renderSpend` all `return` at their first line and the panel renders **completely blank** — no message, no retry, no empty state. The user sees an empty translucent rectangle. → Part 2, Phase 2: render an explicit error state and a retry affordance when `dash === null`. |
| **F-M6** | Windows UX | med | confirmed | `src-tauri/src/providers/copilot.rs:60-63` (cf. `src-tauri/src/providers/antigravity.rs:262-275`) | `resolve_token` runs `Command::new("gh").args(["auth","token"])` **without** `CREATE_NO_WINDOW`, unlike `antigravity.rs`'s `run_hidden` which sets `0x0800_0000` for exactly this reason. In a `windows_subsystem = "windows"` app, spawning a console executable flashes a console window — here on every refresh cycle (default 5 min) for any user with Copilot enabled and no `GITHUB_COPILOT_TOKEN`. → Part 2, Phase 1: reuse a shared `run_hidden` helper for both call sites. |
| **F-M7** | swallowed errors | med | confirmed | `src-tauri/src/lib.rs:107`, `:120`, `:138`, `:155`, `:165`; `src-tauri/src/config.rs:90-95`, `:155-156` | Every config write is `let _ = cfg.save();`. `AppConfig::save` does return `Result<(), String>`, but no caller looks at it, and all the `#[tauri::command]` functions return `()` so they structurally cannot report it. A failed save — `%APPDATA%` missing, file locked, disk full — leaves the UI showing the setting as applied while it is lost on the next restart. → Part 2, Phase 1: change the commands to `Result<(), String>` and surface the message. |
| **F-M8** | data loss | med | confirmed | `src/main.ts:562-567`; `src-tauri/src/lib.rs:92-106`; field at `src-tauri/src/config.rs:55` | `persistConfig` rebuilds `cfg.providers` from scratch as `{ enabled }` only. `set_app_config` compensates by re-merging `api_key`, `team_id` and `region` from the current config (`lib.rs:96-104`) — but **not `hidden_lines`**, so every settings change silently wipes it for every provider. (Mitigating: `hidden_lines` is never read anywhere in the tree, so today the loss has no visible effect — see F-L4.) → Part 2, Phase 2: either merge `hidden_lines` too, or delete the field. |
| **F-M9** | performance | med | confirmed | `src-tauri/src/lib.rs:439-448` | When no provider is enabled, `refresh_sync` falls back to `VendorId::all().to_vec()` — **all 23 providers**. That spawns 23 threads and issues 23 network calls / credential probes / subprocess spawns, every refresh cycle, for a user who has configured nothing. This is precisely the first-run state. → Part 2, Phase 1: the empty-enabled fallback should be a no-op (or a detection pass), not a full fan-out. |
| **F-M10** | concurrency | med | confirmed | `src-tauri/src/lib.rs:77-79` and `:598-601` | `detect_providers` (and the tray "detect" handler) does `let mut cfg = state.config.lock().unwrap().clone();` … `*state.config.lock().unwrap() = cfg;` — a read-modify-write across **two separate lock acquisitions**. Any config change committed by another command between them (`save_api_key`, `set_provider_enabled`, `set_app_config` — all reachable concurrently, see F-H3) is silently overwritten. → Part 2, Phase 1: hold one lock for the whole read-modify-write, or move detection behind the F-H3 guard. |
| **F-M11** | performance | med | confirmed | `src-tauri/src/lib.rs:209` → `src-tauri/src/providers/mod.rs:33-49`, `:23-25` | `build_dashboard` calls `providers::catalog(&cfg)`, which calls `has_local_credentials` for **all 23 vendors** on every invocation — and `build_dashboard` runs on every `get_dashboard` command (on the UI thread) and on every `emit_dashboard`. Those probes are not free: `cursor.rs:68-86` opens a SQLite connection and runs a query against Cursor's state DB, `antigravity.rs:31` reads the Windows Credential Manager, and the rest stat files. → Part 2, Phase 1: cache the catalog and recompute it only on detect / config change. |
| **F-M12** | Windows UX regression | med | **suspected** | `src-tauri/tauri.conf.json` (`skipTaskbar` `false`→`true`, window 900×700→360×500); `src-tauri/src/lib.rs:633-637`, `:639-651`; `src/main.ts:617-627` | The window is `visible: true`, `decorations: false`, `alwaysOnTop: true`, and now `skipTaskbar: true`. `lib.rs:633-637` force-shows and focuses it at setup (this matches the pre-fork behaviour at `HEAD:lib.rs:487-491`, which carried a comment saying it was deliberate — **not** a regression by itself), and `main.ts:617-627` now shows and focuses it **again** from the webview on every page load. Autostart is enabled on first run (`lib.rs:642`). The combination means the panel opens on every Windows login with no title bar, no taskbar entry, and no Escape/close key handler anywhere in `main.ts` — the tray icon is the only way to dismiss it. Marked `suspected` because whether this is actually objectionable needs the running app. → Part 2, Phase 2; confirm via the manual smoke test above. |
| **F-L1** | credential hygiene | low | suspected | `src-tauri/src/http.rs:51-53`, surfaced at `src/main.ts:350` | `FetchError::Http` embeds the first 180 characters of the **response body** into the message, which becomes `ProviderSnapshot.error` and is rendered in the panel. The pre-fork code returned only `format!("HTTP {}", status.as_u16())` (`HEAD:claude_api.rs`), so this widens what reaches the UI. A full scan found **no path where a token, API key or cookie reaches a print macro, a log, or an error string** — there are no `println!`/`eprintln!`/`log::`/`tracing::` calls anywhere in `src-tauri/src`, and every credential is confined to an `Authorization`/`Cookie` header value. But provider error bodies can carry account identifiers and org ids. → Part 2, Phase 3: keep the body for diagnostics but redact it out of the user-facing string. |
| **F-L2** | regression | low | confirmed | `src-tauri/src/providers/claude.rs:204-224` vs `HEAD:src-tauri/src/claude_api.rs` `parse_extra_usage` | `extra_usage` field-name tolerance narrowed: `used` went from 6 candidate keys (`used_credits, used_usd, used, spent, amount, current`) to 3, and `limit` from 6 (`monthly_limit, limit_usd, limit, cap, max, budget`) to 3. The endpoint's shape is undocumented, which is why the original was tolerant. → Part 2, Phase 3. |
| **F-L3** | dead code / feature loss | low | confirmed | `src-tauri/src/lib.rs:522`; removed `open_logs_folder` (was `HEAD:src-tauri/src/lib.rs:84`) | The `open_logs_folder` command was dropped, but `tauri_plugin_opener::init()` is still registered and now has **zero** call sites anywhere in `src-tauri/src`, `src/main.ts` or `index.html`. Either restore the feature or drop the plugin. → Part 2, Phase 3. |
| **F-L4** | dead config surface | low | confirmed | `src-tauri/src/config.rs:21`, `:23`, `:55`; `src-tauri/src/model.rs:309-310`; `src/main.ts:560-561` | Three config fields are plumbed but inert. `show_usage_as` and `reset_times` travel config → `Dashboard` → frontend, where `main.ts` uses them for **nothing** except echoing them straight back in `persistConfig`. `hidden_lines` is declared and never read anywhere at all (and is silently destroyed by F-M8). → Part 2, Phase 3: implement or delete. |
| **F-L5** | latent XSS | low | suspected | `src/main.ts:391`, `:446`, `:449`, `:459`, `:526`, `:565` | Text content is escaped consistently and correctly — `escapeHtml` (`:304`) is applied at `:320`, `:328`, `:333-334`, `:350-351`, `:360`, `:366-367`, `:393`, `:447`, `:451`, `:459`, `:485`, `:507`. The gap is **attribute** interpolation: `data-select="${p.id}"`, `data-key="${v.id}"`, `placeholder="${v.envKey \|\| …}"`, `style="--accent:${accent(v.id)}"`, `value="${v.id}"` are unescaped. All those values are currently `&'static str` constants from `VendorId::slug()` / `env_key()` and a fixed `ACCENT` map, so **nothing provider-controlled reaches an attribute today** — but the pattern is one new dynamic id away from being exploitable. → Part 2, Phase 3. |
| **F-L6** | fragile Windows path | low | confirmed (pre-existing) | `src-tauri/src/tray_icon.rs:16-22` | Font discovery hardcodes `C:\Windows\Fonts\…` rather than deriving from `%SystemRoot%` / `%WINDIR%`. On a Windows install not on `C:`, `font()` returns `None` and `render` silently paints the ring with **no number in it** — the app's entire reason to exist. Unchanged from `HEAD`, so this is **not** introduced by the diff, but it is in scope for the milestone's Windows-robustness goal. → Part 2, Phase 3. |
| **F-L7** | validation inconsistency | low | confirmed | `src-tauri/src/config.rs:78-86` vs `src-tauri/src/lib.rs:89-113` | `refresh_minutes` is coerced into `{1, 5, 10}` **only** in `load()`. `set_app_config` stores whatever the frontend sends without clamping, so an out-of-range value lives in memory (and on disk) until the next restart, at which point the interval silently changes under the user. The `.max(1)` at `lib.rs:512` prevents a busy-loop but nothing else. → Part 2, Phase 3: clamp in one place, on write. |
| **F-L8** | swallowed errors | low | confirmed | `src-tauri/src/providers/kiro.rs:226-235` | Same shape as F-H1 — `to_vec_pretty(…).unwrap_or_default()` can truncate the file, and `let _ = fs::write(…)` hides the failure — but the target is `%APPDATA%\ia-usagebar\kiro-oauth.json`, a cache the **app itself owns**, so the blast radius is a re-refresh rather than a broken external CLI login. Fix alongside F-H1 with the same atomic-write helper. → Part 2, Phase 1. |
| **F-L9** | lost build knowledge | low | confirmed | `src-tauri/Cargo.toml` (diff) | The comment explaining why `crate-type = ["rlib"]` must stay rlib-only — the GNU linker fails with *"export ordinal too large"* when `cdylib`/`staticlib` are present, because of Tauri's symbol surface — was deleted while the setting was kept. The next person to "fix" the crate type will rediscover this the hard way. → Part 2, Phase 3: restore the comment. |
| **F-L10** | dead branch | low | confirmed | `src-tauri/src/model.rs:253-261` | In `MetricLine::utilization`, the second arm (`format == "percent"`) is unreachable whenever `limit > 0.0`, and `progress_pct` (`:401-424`) **always** sets `limit: 100.0`. The first arm happens to compute the right answer for percent lines (`used / 100 * 100`), so behaviour is correct — but the guard order is confusing and the second arm only fires for a hand-built line with `limit == 0`. → Part 2, Phase 3. |
| **F-L11** | input validation | low | confirmed | `src-tauri/src/lib.rs:127-141` | `save_api_key` does `cfg.providers.entry(id.clone()).or_default()` on a caller-supplied `String` **before** checking it against `parse_id`, so an unknown id silently creates a junk entry that is then persisted to `config.toml` forever. (`set_provider_enabled` at `:116-124` gets this right — it validates first.) → Part 2, Phase 3. |
| **F-L12** | dead code / provider mapping | low | confirmed | `src-tauri/src/providers/codex.rs:258-259` | In `snapshot_from_json`, `let visible = if i == 0 { "always" } else { "always" };` — both branches are identical, so the evident intent to hide a secondary rate-limit window behind `"demand"` (the pattern every other provider uses for its non-primary windows, e.g. `antigravity.rs`'s `push_bucket` calls) was never wired up; every Codex window line is always `"always"`. Separately, only the `id` remap on the next line is scoped to `i == 0`, so a secondary window that also classifies to `"window"` (via `classify()`) keeps that non-standard, non-unique id. → Part 2, Phase 3: make the second arm `"demand"` and give each window a guaranteed-unique `id`. |
| **F-L13** | fragile Windows path | low | confirmed | `src-tauri/src/paths.rs:5-7`, exercised by `codex.rs:86` (`write_back`'s target, cf. F-H1), `kiro.rs` via `app_config_dir` → `paths::home_dir` fallback, `local.rs:103,135,164,167`, `cursor.rs:50,59`, `copilot.rs:53` | `home_dir()` silently falls back to `PathBuf::from(".")` — the process's current directory — when `dirs::home_dir()` returns `None`. Every credential path derived from it, including `codex::write_back`'s destructive rewrite (F-H1) and `kiro::refresh_oauth`'s cache write (F-L8), would then read/write relative to wherever the app was launched from instead of failing loudly. `dirs::home_dir()` is reliable on Windows, so likelihood is low, but the fallback silently reinterprets a hard failure as "provider not connected" (reads) or redirects a write into the CWD (codex/kiro). → Part 2, Phase 3: return `Option<PathBuf>` (or surface the failure) instead of defaulting to `"."`. |

### Counts

| severity | confirmed | suspected | total |
| --- | --- | --- | --- |
| high | 4 | 0 | **4** |
| med | 11 | 1 | **12** |
| low | 11 | 2 | **13** |
| **total** | **26** | **3** | **29** |

Task 2 consumes the four `high / confirmed` rows: **F-H1, F-H2, F-H3, F-H4**.

## 4. Regression check against pre-fork Claude Bar

The brief's primary regression question — *does Claude Code session %, weekly %, and local
cost still get computed and displayed?* — is **yes**, feature parity is preserved:

| Pre-fork behaviour | New location | Status |
| --- | --- | --- |
| `five_hour.utilization` → session % | `providers/claude.rs:163-167` → `progress_pct("session", …)` | preserved |
| `seven_day.utilization` → weekly % | `providers/claude.rs:168-172` | preserved |
| `seven_day_sonnet` / `seven_day_opus` | `providers/claude.rs:173-182` | preserved |
| `extra_usage` | `providers/claude.rs:204-224` | preserved but **narrowed** (F-L2) |
| Local cost from `.jsonl` logs | `cost.rs` unchanged in substance; surfaced via `providers/claude.rs:228-257` as four `values_line` rows | preserved |
| `resets_in_label` countdown | `model.rs:370-390` (verbatim move) | preserved |
| Tray % = session utilization | `model.rs:445` takes the first line with a utilization, which for Claude is `session`; `lib.rs:310-323` prefers the configured primary | preserved |
| Cached `User-Agent` (`OnceLock`) | **dropped** | **regressed — F-M1** |
| `HTTP <code>` errors with no body | now includes 180 body chars | **widened — F-L1** |
| `open_logs_folder` command | **dropped** | **removed — F-L3** |

The `pricing.rs` diff and the `tray_icon.rs` diff are **pure `rustfmt` reflow** with no
behavioural change. The `cost.rs` diff is the `CostReport` struct moving out of `model.rs`
into `cost.rs` plus an import swap from `credentials::claude_dir` to `paths::claude_dir` —
also behaviour-preserving. `capabilities/default.json` changed only its description string;
**no permission was added or removed**.

## 5. Things checked and found clean

Recorded so Task 2 and Part 2 do not re-litigate them:

- **i18n key parity.** The `es` and `en` tables in `main.ts:68-143` both have exactly **35 keys** and the sets are identical — no key falls back to its raw string. (Verified programmatically.)
- **DOM id integrity.** All 20 `$("…")` ids used in `main.ts` resolve: 17 are present in `index.html`, and the three that are not (`cfg-interval`, `cfg-primary`, `cfg-notif`) are created by `renderSettings` and are only ever read through optional chaining (`main.ts:557-559`), so the non-null assertion in `$` (`:199`) cannot fire. `.head` and `.foot`, used by `fitWindow`, both exist.
- **No credential reaches a log or a print.** There is not a single `println!`, `eprintln!`, `dbg!`, `log::` or `tracing::` call in `src-tauri/src`. (The pre-fork code had one — `HEAD:lib.rs:500` — and it was removed.) Every token/key/cookie is confined to an `Authorization` or `Cookie` header value; **no credential is placed in a URL**, so the reqwest error strings at `http.rs:42`/`:50`/`:75` cannot leak one through a URL echo.
- **No slice/index panic in the provider parsers.** Every `cols[n]` in `antigravity.rs:210-249` is guarded by a preceding `cols.len()` check, and `kiro.rs:147-150` guards `parts[0]`. `partial_cmp(…).unwrap_or(Ordering::Equal)` at `lib.rs:322` and `model.rs:363` handles NaN correctly.
- **Provider worker panics do not kill the refresh.** `lib.rs:461-462` joins with `if let Ok(…)`, so a panicking provider thread is discarded rather than propagated. (It is discarded *silently*, which is a mild concern, but not a crash.)
- **`has_local_credentials` is cheap for all 23 providers** — file-existence checks, one keyring read, one SQLite open. It is the **frequency** that is the problem (F-M11), not the cost of any single probe.
- **`needs_api_key_ui` covers every `ApiKey`/`Mixed` vendor.** Cross-checking `model.rs:147-166` against `:168-180` and `:200-202`, every vendor that needs a key field gets one. The only gap is `CommandCode`, which is `AuthKind::Local` yet has a `COMMANDCODE_API_KEY` env key and accepts `cfg.api_key(…)` at `local.rs:70-71` — so its key can be set by env var but not through the UI. Too minor to number; noted here.

## 6. Baseline verdict

**The working tree is not safe to commit as-is, but it is close, and Task 2's four `high`
fixes are sufficient to make it so.**

The tree builds clean, passes its 21 tests, and its feature surface is a genuine superset of
pre-fork Claude Bar with only one real behavioural regression (F-M1, a performance one). The
architecture — a `Provider` trait with a per-vendor module, a shared `http`/`paths`/`config`
layer, and pure `snapshot_from_json` mappers that are already unit-tested against fixtures —
is a clear improvement over the monolithic `claude_api.rs` it replaces, and it is the right
foundation for the rest of the milestone.

What makes it unsafe today is a specific and narrow cluster: **the app writes to files it
does not own, from multiple threads, with every failure swallowed.** F-H1 can break the
user's `codex` CLI login and F-H2 can destroy their stored API keys — both silently, both
without any user action that would suggest a destructive operation was happening. F-H3 is
the concurrency amplifier that turns F-H1 from unlikely into probable, and F-H4 means that
when something does go wrong, the app's response is to stop refreshing forever while
continuing to display stale numbers as if they were live. Those four compound: a poisoned
mutex (F-H4) reached through an unguarded concurrent refresh (F-H3) during an `auth.json`
rewrite (F-H1) is a plausible sequence, not a contrived one.

None of the four requires restructuring. F-H1 and F-H2 are contained to `codex.rs` and
`config.rs` and need an atomic write helper plus a non-destructive parse-failure path.
F-H3 and F-H4 are contained to `lib.rs` and need a refresh guard and a poison-tolerant lock
helper. All four are unit-testable without a running GUI, which matters because the manual
smoke test in §2 cannot be run from an agent session.

**Recommendation:** proceed to Task 2, fix F-H1 through F-H4, then commit the working tree.
Run the §2 manual smoke test with the user before the milestone closes — F-M12 and the
Codex-login half of F-H1 are the two items that genuinely cannot be settled by reading code.
Defer all 12 `med` and 11 `low` findings to Part 2 as tagged.

## 7. Supplemental full-provider review

Task 1 required every new `providers/*.rs` file to be read end to end before Task 2 starts.
§1 above recorded that four of them (`codex.rs`, `cursor.rs`, `kiro.rs`, `antigravity.rs`)
had only been read at the credential / subprocess / token-refresh paths, and three
(`local.rs`, `apikey.rs`, `openai_admin.rs`) had only been machine-scanned. This section
closes that gap: all seven were read in full, line by line, against the same checklist as
§3 (swallowed errors, credential leakage, fragile Windows paths, panic risk on the
refresh/tray path, destructive file writes, regression vs pre-fork). `mod.rs`, `claude.rs`
and `copilot.rs` — already fully read for the base audit — were re-skimmed for
cross-references (dispatch table, shared helpers, `map_fetch_err`) but not re-litigated.

| file | lines | previously | now |
| --- | --- | --- | --- |
| `src-tauri/src/providers/codex.rs` | 313 | credential/subprocess/token-refresh paths only | read in full |
| `src-tauri/src/providers/cursor.rs` | 224 | credential/subprocess/token-refresh paths only | read in full |
| `src-tauri/src/providers/kiro.rs` | 306 | credential/subprocess/token-refresh paths only | read in full |
| `src-tauri/src/providers/antigravity.rs` | 373 | credential/subprocess/token-refresh paths only | read in full |
| `src-tauri/src/providers/local.rs` | 297 | machine-scanned only | read in full |
| `src-tauri/src/providers/apikey.rs` | 361 | machine-scanned only | read in full |
| `src-tauri/src/providers/openai_admin.rs` | 109 | machine-scanned only | read in full |
| **total** | **1983** | | |
| `src-tauri/src/providers/mod.rs`, `claude.rs`, `copilot.rs` | — | already fully read | re-skimmed for cross-references only |

**Result: the finding set changed, but only at the margins.** No new `high` finding —
confirmed or suspected — turned up anywhere in the seven files. The full read did not
overturn, weaken, or contradict any of F-H1–F-H4 or the existing `med`/`low` rows; every
credential (Codex access/refresh/id tokens, Cursor's `cursorAuth/accessToken` cookie value,
Kiro's AWS SSO OIDC tokens, Antigravity's keyring blob, every API key handled by `local.rs`
and `apikey.rs`) stays confined to an HTTP header or an owned struct field, and — confirmed
again by a full read rather than a scan — there is still no `println!`/`eprintln!`/`log::`/
`tracing::` call anywhere in these files. The destructive-write surface is exactly the two
spots already known: `codex.rs`'s `write_back` (F-H1, an externally-owned CLI file) and
`kiro.rs`'s own-cache write (F-L8); grepping the full text of all seven files for
`fs::write`/`fs::create`/`Command::new`/`.spawn(`/`std::process` turns up nothing beyond
those two writes and the two subprocess spawns already tracked (`copilot.rs`'s `gh auth
token` behind F-M6, and `antigravity.rs`'s `run_hidden` tasklist/netstat calls, which
already set `CREATE_NO_WINDOW` and so were not re-flagged).

Two new `low`/`confirmed` findings did surface from the parts of `codex.rs` and `paths.rs`
that had not previously been read closely — **F-L12** (a dead `visible` branch and a
non-unique secondary-window id in Codex's mapper) and **F-L13** (the `home_dir()` `"."`
fallback that every provider's file paths ultimately rest on, including the F-H1/F-L8 write
targets). Both are cosmetic/latent, not `high`, and are filed into the existing findings
table and Counts above; see those rows for detail. No existing finding's severity, status,
or fix approach was changed.

Two observations worth recording without minting new ids, because they are more-specific
instances of an already-tracked pattern rather than new defects:

- **`cursor.rs`'s `read_db_token`** (opening `state.vscdb` with
  `Connection::open_with_flags(..., SQLITE_OPEN_READ_ONLY)`) and **`kiro.rs`'s
  `read_credentials`** (same pattern against `data.sqlite3`) can both return `None`/`Err` on
  a transient `SQLITE_BUSY` while the owning IDE/CLI is mid-write to its own state file. Each
  failure path bottoms out in a `snapshot_err(..)` with `connected: false, stale: false`,
  which is exactly the shape F-M3 already describes as replacing a good last-known snapshot
  with a blank "not connected" state. This is additional evidence for F-M3's Part 2 fix, not
  a new finding.
- **`kiro.rs`'s cache write** (F-L8) can lose a *rotated* AWS SSO OIDC refresh token on a
  failed write, exactly as F-H1 describes for Codex — the difference F-L8 already notes
  (the target is app-owned, not the CLI's own file) still holds, but the practical effect of
  losing a rotated token is that the bar's Kiro tile goes into a persistent error state until
  `kiro-cli login` is re-run, not merely "a re-refresh". This refines the *impact* described
  under F-L8's existing text; it does not change F-L8's `low`/`confirmed` rating, since
  `kiro-cli` itself is unaffected and the atomic-write fix already prescribed for F-L8 (share
  F-H1's helper) also closes this gap.

No file outside the seven above was touched or re-read for this pass, and no production
code was changed.
