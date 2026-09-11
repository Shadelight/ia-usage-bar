# Quick Controls, Window Modes & Settings Reorganization

Date: 2026-09-11
Status: Approved by user (in-chat design brief, translated/organized here)
Owner: Alberth Salazar
Relates to: `2026-09-11-provider-identity-foundation.md` (provider visual
registry this spec extends with links), `2026-09-11-usage-semantics-notifications-settings.md`
(settings/credential/autostart groundwork this spec builds a real nav on top of)

## What this covers

The header, tray, and Settings screen still carry rough edges inherited from
the app's Claude-Bar-fork origins: three header icons that don't communicate
what they do, no compact mode, no way to pin the window, provider "panel"
links hardcoded nowhere, and a Settings screen that is one long scroll instead
of categories. This spec turns that area into an intentional "quick controls"
surface, adapted to a multi-provider app — not simply restoring the old
program's features as-is.

Explicitly NOT in scope: an auto-updater (no `tauri-plugin-updater` is
installed; "Buscar actualizaciones" opens the GitHub releases page in the
default browser, nothing more), a real per-provider three-signal status
check, and any change to quota-fetching logic.

## Header

Replace the three unclear icons (`↻ $ ⚙`) with:

```
IA Usage        ↻   📌   ⋯   ─   ×
```

- `↻` Refresh (unchanged).
- 📌 Pin — toggles OS-level always-on-top. Persisted (`AppConfig.always_on_top`,
  default `true`, matching current behavior) so the tray checkbox and the
  header button always agree.
- `⋯` opens a small dropdown menu:
  - Modo compacto (toggle)
  - Notificaciones (toggle)
  - Gasto del mes (opens the existing spend view)
  - Panel del proveedor / Estado del servicio — for the *currently selected*
    provider, each opening `VendorInfo.links.usageUrl` /
    `.statusUrl` in the system browser; hidden when the provider has no such
    link ("No disponible" is not shown as a dead menu item — the row is
    omitted).
  - Abrir carpeta de datos y registros
  - Configuración
- `─` minimize, `×` close-to-tray (unchanged).

## Compact mode

A persisted (`AppConfig.compact_mode`) view state, toggleable from the `⋯`
menu, the tray menu, and Settings → Apariencia. When on:

- Hides: product breakdown, extra quotas/cost rows, the connection guide,
  the "add provider" affordance, and the footer's long text.
- Shows: provider tabs (icon + status dot only, no label) and the primary
  quota's percent + reset countdown for the selected provider, plus the
  headroom recommendation banner if present.
- The window resizes smaller (`fitWindow` picks a lower floor/ceiling in
  compact mode, no fixed pixel constant duplicated from the CSS).

Implemented as a CSS-driven view (an `.panel.compact` class hides sections),
not a second render path — the existing DOM from `dash.ts` is reused.

## Pin (always-on-top)

`AppConfig.always_on_top: bool`, default `true` (matches the app's current
fixed behavior, so this is additive, not a regression). A `set_always_on_top`
command persists it, applies it to the window immediately, and updates the
tray checkbox. Startup applies the stored value instead of the config's
current hardcoded `alwaysOnTop: true`.

## Provider links

`VendorInfo` gains a `links` field:

```rust
pub struct VendorLinks {
    pub usage_url: Option<String>,
    pub billing_url: Option<String>,
    pub status_url: Option<String>,
}
```

Populated only for providers where the official URL is well-known and
verifiable (Anthropic, OpenAI, GitHub Copilot, Cursor, OpenRouter, DeepSeek,
Groq). Every other provider gets `None` for all three — **no invented
links**. The frontend must render "No disponible" (or omit the menu row,
per surface) rather than guessing.

This is backend/registry data, not `if (provider === "x")` branches in the
UI — one lookup, consumed the same way by every view.

## Settings reorganization

Replace the single scrolling Settings body with categories:

```
General | Proveedores | Notificaciones | Apariencia | Datos y registros | Acerca de
```

- **General**: autostart, pin (always-on-top), refresh interval, primary
  provider.
- **Proveedores**: the existing provider list + API key fields (unchanged
  content, moved here).
- **Notificaciones**: the existing notifications toggle + thresholds
  (unchanged content, moved here).
- **Apariencia**: compact mode, language (moved out of the footer segmented
  control — the footer keeps a lang shortcut too; this is not a removal,
  just an additional entry point).
- **Datos y registros**: abrir carpeta de logs, limpiar logs, exportar
  diagnóstico (sanitized JSON: app version, OS, per-provider status +
  statusReason + updatedAt — no tokens, no keys, no raw HTTP bodies).
- **Acerca de**: app name, version (via Tauri's `getVersion()`, not
  hardcoded), one-line description, author, and a link to the repository.
  This is product branding — it does not touch `NOTICE`/`LICENSE`, which
  keep any legally-required third-party attribution regardless of how the
  app describes itself here.

The existing "connection guide" content stays, folded into **Proveedores**
(it's the same subject).

## Tray menu

```
Abrir IA Usage Bar
Actualizar ahora
Detectar proveedores
Proveedor principal >     (submenu: one radio-style item per enabled provider)
Modo compacto             (checkbox)
Siempre visible           (checkbox)
Iniciar con Windows       (checkbox, existing)
Configuración
Salir
```

The primary-provider submenu rebuilds whenever the catalog changes (same
place `refresh_catalog` already runs).

## Logs and diagnostics

The app currently has no on-disk log — failures only reach `eprintln!`,
invisible in a windowed build. This spec adds the minimum needed to make
"Abrir logs" and "Exportar diagnóstico" meaningful:

- A small append-only log at `%APPDATA%\ia-usagebar\logs\app.log`, one line
  per provider refresh failure (timestamp, provider id, status reason —
  no secrets), capped at 2000 lines (oldest trimmed on write).
- "Abrir carpeta de logs" opens that `logs` folder in Explorer.
- "Limpiar logs" truncates the file.
- "Exportar diagnóstico" writes a sanitized JSON snapshot next to the log
  file and opens the folder so the user can attach it to a bug report.

No logging crate is added — this is a plain buffered file append, well
within stdlib.

## Explicitly out of scope

- Auto-updater / "Buscar actualizaciones" beyond opening the releases page.
- Real per-signal (app/account/quota) status probing.
- A save-file dialog (`tauri-plugin-dialog`) — diagnostics/logs use a fixed,
  disclosed folder instead.
- Removing "Fork de Claude Bar" branding from the UI — it was not found
  anywhere in the running app's rendered strings (only in `README.md`,
  `NOTICE`, and design docs, which are correct places for it). The new
  **Acerca de** panel simply never mentions Claude Bar; no deletion needed.
