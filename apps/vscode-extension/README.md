# IA Usage — AI Usage & Quota Monitor

[![Visual Studio Marketplace Version](https://img.shields.io/visual-studio-marketplace/v/shadelightdev.ia-usage?logo=visual-studio-code&label=Marketplace)](https://marketplace.visualstudio.com/items?itemName=shadelightdev.ia-usage)
[![Visual Studio Marketplace Installs](https://img.shields.io/visual-studio-marketplace/i/shadelightdev.ia-usage)](https://marketplace.visualstudio.com/items?itemName=shadelightdev.ia-usage)
[![Open VSX Version](https://img.shields.io/open-vsx/v/shadelightdev/ia-usage?logo=eclipseide&label=Open%20VSX)](https://open-vsx.org/extension/shadelightdev/ia-usage)
[![Open VSX Downloads](https://img.shields.io/open-vsx/dt/shadelightdev/ia-usage)](https://open-vsx.org/extension/shadelightdev/ia-usage)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/Shadelight/ia-usage-bar/blob/main/LICENSE)

Monitor your **Claude Code, OpenAI Codex, Cursor, Antigravity and other AI usage limits** directly from the VS Code status bar.

See session quotas, weekly limits, reset countdowns, and remaining usage without opening separate dashboards or breaking your coding flow.

---

## Supported Providers

IA Usage integrates official brand icons and dedicated quota tracking for:

- **Anthropic / Claude Code** (Session & Daily/Weekly limits)
- **OpenAI Codex / ChatGPT** (5-hour window & Weekly limits)
- **Cursor** (Fast requests & Monthly quota)
- **Antigravity** (Local & Remote agent quotas)
- **GitHub Copilot**
- **Windsurf**
- **OpenRouter**
- **DeepSeek**
- **Groq**
- **Kimi (Moonshot)**
- **MiniMax**
- **Grok (xAI)**
- **Z.AI / OpenCode Go / Novita / Kiro / Nous**
- *Any unsupported provider automatically falls back to a generic icon with a clean 3-letter code — never breaking your status bar.*

---

## Why IA Usage?

When developing with multiple AI tools, keeping track of how much quota or token budget you have left is tedious. You shouldn't have to switch tabs or open separate dashboards just to check if your rate limit is about to reset.

**IA Usage** brings all your AI quotas together into a clean, lightweight status bar widget:

```text
$(claude) CLD 18%   $(openai) CDX 21% (39m)   $(cursor) CUR 40%
```

- **Instant Visibility**: See at a glance if you're burning through quota faster than expected.
- **Accurate Reset Times**: Know exactly when your session or weekly quota refreshes (`(39m)`).
- **Zero Configuration Overhead**: Interactive quick settings let you customize visible providers and display formats without manually editing JSON files.
- **Privacy-First & Secure**: Runs completely locally via `iausage watch --jsonl`. The extension never asks for API keys, stores no credentials, and performs no external telemetry.

---

## Key Features

### 1. Flexible Status Bar Display
Customize how information appears in your status bar (`iaUsage.display`):
- **Minimal**: Brand icon + percentage (`$(openai) 21%`)
- **Compact**: Brand icon + 3-letter code + percentage (`$(openai) CDX 21%`)
- **Full**: Brand icon + Provider name + percentage (`$(openai) Codex 21%`)

You can also customize:
- **Used vs. Remaining**: Display used quota (`21%`) or remaining quota (`79% free`).
- **Reset Countdown**: Show time left until quota reset directly in the status bar (`21% (39m)`).
- **Primary Metric**: Choose which limit represents each provider (e.g., 5-hour rolling limit vs weekly limit for Codex).
- **Stale Data Indicators**: Providers with temporarily unrefreshed data are subtly marked (`*`) so you always know what's live.

### 2. Rich Native Tooltip
Hover over any provider in the status bar to see comprehensive metrics:
- Full provider name and brand icon.
- Exact used and available quota percentages.
- Exact countdown time to the next reset.
- Detailed breakdown of all active metrics (or only relevant ones via `iaUsage.showAllMetrics`).

### 3. Interactive Quick Settings
Press `Ctrl+Shift+P` (or `Cmd+Shift+P`) and select:
```text
IA Usage: Customize status bar
```
Or simply click on any provider in the status bar. A native VS Code QuickPick menu allows you to:
- Toggle which providers are visible.
- Change display density (`minimal`, `compact`, `full`).
- Switch between used and available percentages.
- Toggle reset timers and stale indicators.
- Select primary metrics per provider.
- All changes apply immediately without reloading the window.

### 4. Bilingual Support
Automatically detects your VS Code display language (`en` / `es`) for commands, menus, quick settings, tooltips, and status messages.

---

## Getting Started

1. **Install the Extension**: From [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=shadelightdev.ia-usage) or [Open VSX](https://open-vsx.org/extension/shadelightdev/ia-usage).
2. **Ensure CLI is Installed**: The extension communicates with the `iausage` CLI. If you have [IA Usage Desktop](https://github.com/Shadelight/ia-usage-bar/releases/latest) installed, it is auto-detected!
   - You can also configure a custom CLI binary path in VS Code Settings: `iaUsage.cliPath`.
3. That's it! Your AI quotas will appear in the status bar automatically.

---

## Settings Reference

| Setting | Type | Default | Description |
|---|---|---|---|
| `iaUsage.display` | `string` | `"compact"` | Status bar density: `"minimal"`, `"compact"`, or `"full"`. |
| `iaUsage.percentageMode` | `string` | `"used"` | Display `"used"` percentage or `"available"` remaining quota. |
| `iaUsage.showResetInStatusBar` | `boolean` | `false` | Show reset countdown in parentheses next to percentage. |
| `iaUsage.showStaleIndicator` | `boolean` | `true` | Append `*` to providers whose data has not refreshed recently. |
| `iaUsage.showAllMetrics` | `boolean` | `false` | In tooltips, show all metrics including zero-usage quotas. |
| `iaUsage.showProviderIcons` | `boolean` | `true` | Show each provider's brand icon in the status bar. |
| `iaUsage.primaryMetric` | `object` | `{}` | Maps a provider ID to the quota ID that represents it in the status bar. |
| `iaUsage.providers` | `array` | `["anthropic", "openai", "cursor"]` | Provider IDs visible in the status bar, in order. |
| `iaUsage.remotePollSeconds` | `number` | `60` | Remote fallback polling interval used by `iausage watch`. |
| `iaUsage.cliPath` | `string` | `""` | Optional path to custom `iausage` binary. |

---

## Documentación en Español

<details>
<summary><b>Haz clic aquí para ver la documentación en español</b></summary>

### Descripción
La extensión muestra cuotas de IA en la barra de estado sin implementar proveedores ni guardar credenciales. Inicia una sola instancia local de:
```text
iausage watch --jsonl
```
El CLI emite un snapshot V1 al conectar, cuando cambian sesiones locales de Codex (debounce de dos segundos) y en el intervalo remoto de respaldo. La extensión solo consume stdout JSON Lines.

### Barra de estado
- **Densidad** (`iaUsage.display`): `minimal` (icono + %), `compact` (icono + código de 3 letras + %) o `full` (icono + nombre completo + %).
- **Usado o disponible** (`iaUsage.percentageMode`): `CDX 21%` vs `CDX 79% libre`.
- **Reset opcional** (`iaUsage.showResetInStatusBar`): `CDX 21% (39m)`.
- **Métrica principal por proveedor** (`iaUsage.primaryMetric`): elige qué cuota representa a cada IA en la barra desde **Ajustes rápidos**.
- Iconos de marca reales para Anthropic, OpenAI, Cursor, Antigravity, OpenRouter, DeepSeek, Groq, Kimi, MiniMax, Copilot, Windsurf y más.

### Ajustes rápidos
`IA Usage: Customize status bar` abre un flujo de QuickPick nativo para elegir proveedores visibles, apariencia, métrica principal, usado/disponible, reset y tooltip — todo se guarda en Settings al instante sin editar JSON ni recargar la ventana.

### Desarrollo y VSIX
```powershell
npm install
npm run test     # suite de tests
npm run bundle   # compilar bundle
npm run package  # crear paquete .vsix
```
</details>

---

## License

MIT © [Shadelight](https://github.com/Shadelight)
