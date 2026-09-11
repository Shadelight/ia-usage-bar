# Responsive and startup release blocker — implementation report

Date: 2026-09-11  
Release status: **BLOCKED pending installer smoke test**  
Scope: startup/cache, per-provider loading, responsive shell, scrolling, compact mode,
Appearance, About, update check, current Claude breakdown, and Windows bundles.

## Root causes

1. `setup` synchronously ran credential detection for every provider and then rebuilt a
   catalog that probed the same providers again before the window was ready.
2. Provider requests were started on separate threads but joined in provider order. The
   dashboard was emitted only after every join, so a slow first provider held back all
   completed providers.
3. No last-known-good snapshot survived a full process restart. Every launch therefore
   returned to a whole-panel skeleton.
4. `fitWindow()` forced the window back to 360 px after every render. It defeated manual
   resize, made Settings fragile, and used content height as a substitute for scrolling.
5. The main views used generic `overflow:auto` while the shell and several ancestors used
   `overflow:hidden`; header, navigation, content and footer had no explicit flex ownership.
6. Compact mode was the normal dashboard with selected elements hidden. About used bare
   anchors, and Appearance mixed native checkboxes with disabled placeholder controls.

## Implemented

- Loads cached valid snapshots before the first dashboard request and marks every cached
  quota stale until its provider confirms fresh data.
- Persists only successful, non-empty provider snapshots; a failed refresh keeps the last
  valid snapshot and its timestamp.
- Moves detection out of synchronous startup. The initial catalog uses the persisted
  detection set and detection refreshes in the background.
- Emits once when refresh begins and again after each provider result. Provider loading is
  represented by `loadingProviders`; disabled providers are never scheduled.
- Applies a 12-second fan-out deadline. A timed-out provider becomes a structured network
  error while other provider results remain usable.
- Adds development-only startup milestones and per-provider completion times.
- Replaces automatic 360 px resizing with a 410×640 normal default and dynamic normal/
  compact constraints. Compact mode is 350×220 and preserves the previous normal size
  while switching modes.
- Makes the dashboard a fixed header + fixed provider navigation + independently scrolling
  detail + fixed footer shell. Settings and Spend have their own content scroll areas.
- Adds real horizontal provider/settings scrolling, wheel-to-horizontal behavior, active
  item reveal, edge fades and a one-time “Desliza para ver más” hint. The add button remains
  outside the provider scroll area.
- Adds 340–419 px and 700+ px behavior, a 760 px content cap, thinner scrollbars, and one
  outer rounded surface: the native window owns radius/shadow while the WebView fills its
  clipped bounds without a second CSS radius.
- Adds a dedicated compact quick-glance card with provider, two quota rows, relative resets
  and recommendation.
- Rebuilds General/Appearance controls around one switch, segmented theme and language
  controls, immediate application and persistence.
- Rebuilds About as action rows and uses Tauri's opener plugin for HTTPS destinations.
  Update checking is a real backend GitHub Releases query with checking/current/new/error
  states and an 8-second timeout.
- Updates Claude's current `seven_day_breakdown.rows` parser and no longer creates Sonnet/
  Opus quotas when the provider sends those windows as `null`.

## Measured startup

Debug build, same Windows machine and enabled-provider set:

| Milestone | Result |
| --- | ---: |
| Config loaded on cached relaunch | 1 ms |
| Snapshot cache loaded (3 providers) | 2 ms |
| Tauri `setup` complete | 18 ms |
| SuperGrok result | 17 ms |
| Cursor result | 558 ms |
| Codex result | 568 ms |
| OpenAI Admin result | 696 ms |
| Copilot result | 1,073 ms |
| Claude result | 5,253 ms |

Under the previous join-and-single-emit implementation, the 5,253 ms Claude request would
have kept the already completed Cursor/Codex results invisible. The new implementation
renders cached data during that interval and publishes each fresh provider independently.

## Automated evidence

`scripts/capture-responsive.mjs` uses Chromium's device emulation rather than cropping a
desktop viewport. `responsive-proof.json` records:

- no page-level horizontal overflow at 360, 390, 420 or 600 px;
- vertical content reached its real maximum scroll offset;
- seven-provider navigation reached its maximum horizontal offset;
- Settings categories reached their maximum horizontal offset;
- theme persistence, two language choices, About URL actions, two compact quota rows,
  compact recommendation and hidden compact footer.

Visual evidence:

- `evidence/2026-09-11-responsive/dashboard-360.png`
- `evidence/2026-09-11-responsive/dashboard-390.png`
- `evidence/2026-09-11-responsive/dashboard-420.png`
- `evidence/2026-09-11-responsive/dashboard-600.png`
- `evidence/2026-09-11-responsive/dashboard-light-390.png`
- `evidence/2026-09-11-responsive/settings-360.png`
- `evidence/2026-09-11-responsive/appearance-360.png`
- `evidence/2026-09-11-responsive/about-390.png`
- `evidence/2026-09-11-responsive/compact-350.png`
- `evidence/2026-09-11-responsive/responsive-proof.json`

## Verification and bundles

- Rust: 48 passed, 0 failed.
- Frontend: 11 passed, 0 failed.
- Vite production build: passed.
- Tauri release binary and NSIS: passed.
- Local WiX ICE validation could not start through Tauri's normal linker invocation. The
  already generated WiX object was linked with the same WiX tool using `-sval`; this skips
  ICE validation only and produced the current MSI. CI must run normal validation.

Artifacts:

- `src-tauri/target/release/bundle/nsis/IA Usage Bar_0.2.0_x64-setup.exe`  
  SHA-256 `f9461d8ef76fa8ec436186f47b4c4a1b33c9d14682a969a1e5d5b3665e29c703`
- `src-tauri/target/release/bundle/msi/IA Usage Bar_0.2.0_x64_en-US.msi`  
  SHA-256 `9c67bdf8a4879f58867d1e29683f7e4c563382ee751e394f285611a700792b50`

## Remaining release gate

Do not push/tag/publish yet. Install and smoke both newly generated installers on Windows:
normal startup, cached relaunch, vertical/horizontal scroll, Settings categories, Appearance,
About actions, Compact, minimize/taskbar, close-to-tray/reopen, explicit Exit, Claude, Codex,
uninstall, then repeat the installation check with MSI. The GitHub runner must also produce
an MSI with normal ICE validation before the release draft is approved.
