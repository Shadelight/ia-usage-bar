# IA Usage Bar for Windows

> Keep every AI plan you use in the Windows tray — remaining quota, reset time, and spend, one click away.

![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Platform: Windows 10/11](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6)
![Built with: Rust + Tauri](https://img.shields.io/badge/Built%20with-Rust%20%2B%20Tauri-orange)

A Windows tray app that monitors **Claude Code, Codex/ChatGPT, Cursor, Antigravity, OpenAI API**, GitHub Copilot, OpenRouter, Z.AI, DeepSeek, Grok, Kimi, and the rest of the [ai-usagebar](https://github.com/akitaonrails/ai-usagebar) catalog. Built with **Rust + Tauri 2**.

This project is a maintained fork of [Claude Bar by Daybi](https://github.com/Daybi), expanded into a multi-provider monitor. MIT. Maintained by Alberth Salazar.

## What it shows

Each enabled provider is a card with:

- Quota bars (session / weekly / billing cycle, depending on the vendor)
- Reset countdown (click to flip to the exact time)
- Used vs remaining (click the % to flip)
- Pace notes when you are burning faster than the window
- Local cost estimates for Claude Code (API-equivalent, from `~/.claude` JSONL logs)

The tray icon paints the primary provider's usage percentage (green → amber → red).

## How it works

Credentials stay on your PC. IA Usage Bar only talks to each vendor's own usage endpoint.

| Provider | Auth |
|---|---|
| Claude Code | `%USERPROFILE%\.claude\.credentials.json` |
| Codex / ChatGPT | `%USERPROFILE%\.codex\auth.json` (`codex login`) |
| Cursor | Cursor IDE `state.vscdb` or `cursor-agent` `auth.json` |
| Antigravity | Local language server or Google session in Windows Credential Manager (`gemini:antigravity`) |
| OpenAI API | `OPENAI_ADMIN_KEY` (org costs) |
| Copilot | `gh auth token` or `GITHUB_COPILOT_TOKEN` |
| OpenRouter, Z.AI, DeepSeek, Grok, Kimi, … | Environment variable or API key in Settings |

On first launch it **detects** which tools are already signed in and enables those providers. It never turns a provider off for you.

Polling defaults to every **5 minutes** (1 / 5 / 10 in Settings) because several usage APIs rate-limit harder than that.

## Install

**Requirements:** Windows 10/11. For Claude/Codex/Cursor, sign in to those apps once.

```powershell
npm install
npm run tauri dev      # development
npm run tauri build    # NSIS installer
```

Config lives at `%APPDATA%\ia-usagebar\config.toml`.

## Privacy

Read-only and local-first. No telemetry. Tokens are read from files the official CLIs already maintain, or from keys you paste, and are sent only to that vendor.

## Credits

See [NOTICE](NOTICE). Original Claude Bar: Daybi. Provider catalog adapted from [akitaonrails/ai-usagebar](https://github.com/akitaonrails/ai-usagebar). UI contract inspired by [OpenUsage](https://github.com/robinebers/openusage). OpenAI Admin mapping from [CodexBar](https://github.com/steipete/CodexBar). Cost math modeled after [ccusage](https://github.com/ryoppippi/ccusage).

## License

MIT — © 2026 Daybi (original Claude Bar) · © 2026 Alberth Salazar (IA Usage Bar).
See [LICENSE](LICENSE).
