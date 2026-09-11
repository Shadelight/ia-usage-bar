# Changelog

All notable changes to this project are documented here. This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-09-11

First release under the IA Usage Bar name. `v0.1.0` was the original Claude
Bar fork this project started from; this is the reconstructed multi-provider
monitor.

### Added

- Multi-provider catalog (Claude Code, Codex/ChatGPT, Cursor, Antigravity,
  GitHub Copilot, OpenAI API, OpenRouter, Z.AI, DeepSeek, Grok, Kimi, and more)
  with a shared identity registry, local official icons, and explicit
  connection states instead of a generic connected/error boolean.
- Canonical quota, reset, credit, product-breakdown, and cost models shared
  by every provider adapter.
- Configurable notification thresholds and launch-at-login setting.
- Pin (always-on-top) and compact window modes, both persisted and mirrored
  in the tray menu.
- Header quick-actions menu (compact mode, notifications, spend, provider
  usage/status links, logs folder, settings).
- Categorized Settings screen (General, Providers, Notifications,
  Appearance, Data & logs, About).
- Minimal capped log file and a diagnostics export for bug reports.
- Windows taskbar, minimize, close-to-tray, and single-instance behavior.

### Changed

- Transient provider failures preserve the last valid metrics as stale data.
- API keys entered in Settings are stored in Windows Credential Manager.
- Product identity is now IA Usage Bar; see [NOTICE](NOTICE) for the
  third-party attribution this carries forward from Claude Bar and the other
  MIT-licensed projects it draws on.

[Unreleased]: https://github.com/Shadelight/ia-usage-bar/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.2.0
