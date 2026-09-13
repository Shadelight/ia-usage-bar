# IA Usage for VS Code

La extensión muestra cuotas de IA en la barra de estado sin implementar
proveedores ni guardar credenciales. Inicia una sola instancia de:

```text
iausage watch --jsonl
```

El CLI emite un snapshot V1 al conectar, cuando cambian sesiones locales de
Codex (debounce de dos segundos) y en el intervalo remoto de respaldo. La
extensión solo consume stdout JSON Lines.

## Barra de estado

```text
$(claude) CLD 18%   $(openai) CDX 21%   $(cursor) CUR 40%
```

- **Densidad** (`iaUsage.display`): `minimal` (icono + %), `compact` (icono +
  código de 3 letras + %) o `full` (icono + nombre completo + %).
- **Usado o disponible** (`iaUsage.percentageMode`): `CDX 21%` vs `CDX 79%
  libre`.
- **Reset opcional** (`iaUsage.showResetInStatusBar`): `CDX 21% (39m)`.
- **Métrica principal por proveedor** (`iaUsage.primaryMetric`): elegí qué
  cuota representa a cada IA en la barra (p. ej. "5 horas" en vez de
  "Semanal" para Codex) desde **Ajustes rápidos**, no editando JSON.
- Un proveedor con datos viejos se marca con `*` (`iaUsage.showStaleIndicator`);
  el fondo de advertencia solo se activa si **todos** los proveedores
  visibles están stale.
- Iconos de marca reales para Anthropic, OpenAI, Cursor, Antigravity,
  OpenCode Go, OpenRouter, DeepSeek, Groq, Kimi, Kilo Code, MiniMax, Grok,
  GitHub Copilot, Windsurf, Z.ai, Moonshot, Novita, Kiro y Nous — un
  proveedor no soportado nunca rompe la barra: cae a un ícono genérico y un
  código de 3 letras derivado del nombre.

## Ajustes rápidos

`IA Usage: Customize status bar` abre un flujo de QuickPick nativo (sin
WebView) para elegir proveedores visibles, apariencia, métrica principal,
usado/disponible, reset y qué tan detallado es el tooltip — todo se escribe a
Settings (Global) al instante, sin editar `settings.json` ni recargar la
ventana. Al hacer clic en la barra, cada fila de proveedor también abre su
propio detalle con las mismas acciones.

## Tooltip

Nombres completos, iconos de marca, uso **y** disponibilidad juntos, reset en
línea aparte, estado stale explícito y como máximo 5 proveedores antes de
colapsar el resto en "+N más". `iaUsage.showAllMetrics` decide si se muestran
solo las cuotas relevantes o todas (incluidas las que están en 0%).

## Idioma

La extensión detecta el idioma de VS Code (`es`/`en`) para el menú, los
ajustes rápidos, el tooltip y los mensajes de error.

## Desarrollo y VSIX

```powershell
npm install
npm run test     # suite de node:test sobre la lógica pura + un vscode falso
npm run bundle
# Abrir esta carpeta en VS Code y pulsar F5
npm run package
```

El VSIX resultante se instala desde **Extensions → Install from VSIX** o con
`code --install-extension`. La extensión resuelve el CLI en este orden:
`iaUsage.cliPath`, `PATH` y las ubicaciones conocidas del instalador de IA
Usage Desktop. Si no lo encuentra, muestra un error con el botón **Configurar
CLI**. Para desarrollo, apunta `iaUsage.cliPath` a
`%LOCALAPPDATA%\\Programs\\IA Usage\\iausage.exe`.

El mismo VSIX puede probarse en VS Code, Cursor, Windsurf o VSCodium si el
fork acepta extensiones compatibles. La distribución pública se realizará por
Visual Studio Marketplace, Open VSX y las GitHub Releases del proyecto.
