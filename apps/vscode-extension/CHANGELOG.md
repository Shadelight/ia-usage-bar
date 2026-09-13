# Changelog

All notable changes to IA Usage will be documented in this file.

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
