# Changelog

All notable changes to IA Usage will be documented in this file.

## 0.4.0 — 2026-09-13

### Added

- `IA Usage: Customize status bar` command (`iaUsage.customizeStatusBar`): a
  native QuickPick flow (no WebView) covering visible providers, appearance,
  primary metric, used/available, reset display and tooltip detail — writes
  to Settings (Global) instantly.
- `iaUsage.primaryMetric`: choose which quota represents each provider in
  the status bar (e.g. "5 hours" instead of "Weekly" for Codex), by stable
  quota id, with automatic fallback if the configured id disappears.
- `iaUsage.percentageMode` (`used`/`available`): the status bar and tooltip
  can show "21%" or "79% free"; the tooltip always shows both regardless.
- `iaUsage.showResetInStatusBar`: optional compact reset countdown next to
  the percentage, e.g. `CDX 21% (39m)`.
- `iaUsage.showProviderIcons` / `iaUsage.showStaleIndicator` toggles.
- `IA Usage: Reorder providers` command for sequential reordering without
  hand-editing `iaUsage.providers`.
- Clicking a provider row in the main menu now opens a real detail view
  (primary metric picker, show/hide toggle) instead of being decorative.
- Brand icons for 16 more providers (Antigravity, OpenCode Go, OpenRouter,
  DeepSeek, Groq, Kimi, Kilo Code, MiniMax, Grok, GitHub Copilot, Windsurf,
  Z.ai, Moonshot, Novita, Kiro, Nous), reusing the desktop app's SVGs.
- Spanish/English localization (`src/i18n.ts`) for the menu, quick settings,
  tooltip and error messages, following VS Code's own language.
- A refresh spinner (`$(sync~spin)`) now shows next to the existing values
  during a manual refresh instead of the status bar going blank.
- A `node:test` suite (`npm run test`) covering the pure formatting/decision
  logic and, via a small in-repo `vscode` mock, the QuickPick/tooltip flows.

### Changed

- A cosmetic settings change (display density, percentage mode, primary
  metric, icons, stale indicator) now only re-renders the status bar; only
  `cliPath`/`remotePollSeconds` still restart the CLI watch process.
- The status bar no longer prefixes an aggregate `check`/`clock`/`pulse`
  icon; a stale provider is marked individually with `*` instead of the
  whole bar turning into one icon.
- Internal: pure formatting/decision logic (`getPrimaryQuota`,
  `formatPercentage`, `buildStatusBarLabel`, `visible`,
  `shouldWarnBackground`, `isRenderOnlyChange`) now lives in vscode-free
  modules (`status/format.ts`, `config-change.ts`) so it can be unit tested
  without a running VS Code instance.

## 0.3.2 — 2026-09-13

### Added

- Real provider logos (Claude, Codex, Cursor) now render directly in the
  status bar, not just the tooltip — via a small custom icon font
  (`contributes.icons`) built from the same brand SVGs the desktop app uses.
- `IA Usage: Configure providers` command: a checkbox picker for which
  providers show in the status bar, replacing hand-edited JSON.
- `iaUsage.display` gained a `minimal` mode (icon + percentage only).

### Changed

- Internal refactor: `status-bar.ts` split into `status/{status-bar,
  tooltip, quick-menu, provider-visuals, format}.ts`.

## 0.3.1 — 2026-09-13

An earlier `0.3.0` was published to Open VSX before this project's other
products (desktop, CLI) settled on their own 0.2.x line; this release
supersedes it and folds in everything shipped under 0.2.4–0.2.7 in the
meantime, so 0.2.x version numbers for this extension are skipped.

### Added

- Real per-provider logos (Claude, Codex, Cursor) in the tooltip.
- `iaUsage.showAllMetrics` setting to reveal zero-value secondary quotas.

### Changed

- Compact status bar codes are now 3 letters (`CLD`/`CDX`/`CUR`).
- Tooltip shows a stale/updated badge with the data's age and finer-grained
  reset countdowns (hours+minutes, days+hours).
- Status bar icon reflects state (`check`/`clock`/`pulse`); the warning
  background only applies when every visible provider is stale.

### Fixed

- Tooltip no longer sets `isTrusted`, and provider/quota names from the CLI
  are Markdown-escaped, closing a potential command-link injection.

## 0.1.0 — 2026-09-12

### Added

- Live Claude Code usage in the VS Code status bar.
- Live Codex / ChatGPT quota monitoring.
- Cursor usage monitoring.
- Near real-time Codex updates from local session activity.
- Detailed tooltips with quota reset times.
- Stale-data indicators.
- Manual refresh command.
- Reconnect CLI command.
- Configurable IA Usage CLI path.
- Compact and full display modes.

### Security

- The extension does not store provider API keys, cookies, or sessions.
- Provider authentication remains managed by the local IA Usage CLI.
