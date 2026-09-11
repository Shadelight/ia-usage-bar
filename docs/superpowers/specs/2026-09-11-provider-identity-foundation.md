# Provider Identity Foundation

Date: 2026-09-11
Status: Approved by user review (in-chat, section by section)
Owner: Alberth Salazar
Part of: Milestone 0, Part 2 (sub-project 1 of 2 — see "Decomposition" below)
Relates to: `2026-09-10-ia-usage-bar-stabilization.md` (Milestone 0 Part 1, already
shipped), `docs/design/phase-0-audit.md` (closes F-M2 as a side effect; F-M4/F-M5
remain open, targeted by sub-project 2)

## What this covers

The running app has three visible problems reported by the user:

1. Providers are visually indistinguishable — reused generic glyphs instead of
   brand identity, and the same vendor can render differently on different
   screens because icon/color/label logic is duplicated across views.
2. Backend errors reach the UI as raw HTTP/JSON — a console dump instead of a
   message a human can act on.
3. Connection failures collapse into one generic string, so distinct problems
   ("app not running" vs "no saved session") show identical, sometimes wrong,
   text — confirmed as a real bug in Antigravity's detection.

This spec is the **foundation**: a real provider→visual identity mapping, a
structured connection-status model that replaces the free-text error/connected
boolean, and a user-facing error-normalization layer built on top of it. Every
other visual complaint in the user's brief (notifications, settings layout,
main-screen density, usage-percentage semantics) depends on this contract but
doesn't change it, so it is a separate spec (sub-project 2, written after this
one ships).

### Decomposition

The user's original brief described several independent subsystems (icon
identity, error handling, connection-state model, notifications, settings
restructure). Per the brainstorming skill's decomposition guidance, this was
split into two sub-projects instead of one large spec:

1. **Provider Identity Foundation (this doc).** Touches the Rust provider
   contract (`ProviderSnapshot`, `FetchError` mapping) and introduces the
   frontend's single provider-visual registry and error-normalization layer.
2. **Usage Semantics, Notifications & Settings** (not yet written). Pure
   consumer of this contract: configurable notification thresholds, the
   "better alternative" banner, settings categorization, connection-guide
   copy, main-screen density pass, and the used/available percentage
   convention audit. Brainstormed separately once this ships.

### Explicitly out of scope (this sub-project)

- Notification thresholds, copy, and the recommendation banner — sub-project 2.
- Settings screen restructure ("Administrar proveedores", categories) —
  sub-project 2.
- Main-screen layout/density changes, used/available percentage audit —
  sub-project 2.
- Real three-signal detection (independently probing "app running" vs "auth"
  vs "quota") per provider. Rejected during design — see "Connection status
  model" below for the cheaper alternative that was chosen instead.
- Any change to quota-fetching logic itself. The only backend changes are:
  classifying an already-known failure into a structured status instead of a
  free-text string, and fixing Antigravity's specific state-conflation bug.

## Provider visual identity

### Icon sourcing

`simple-icons` (the initially proposed source) has no entry for OpenAI — removed
after a trademark takedown request — nor for Grok, Groq, Kiro, Kilo Code,
Novita, or Nous Research. Since OpenAI alone covers three of the app's 23
vendors (Codex, ChatGPT-based usage, OpenAI Admin API), that gap was
unacceptable.

`@lobehub/icons-static-svg` (npm, MIT, framework-agnostic flat SVG files —
purpose-built for AI/LLM provider brand logos) was evaluated instead and
covers 21 of 23 `VendorId` variants with an official mark: `openai`,
`anthropic`/`claude`/`claudecode`, `cursor`, `gemini`/`geminicli`, `copilot`,
`openrouter`, `grok`, `xai`, `kimi`, `moonshot`, `minimax`, `kiro`,
`kilocode`, `novita`, `nousresearch`, `groq`, `windsurf`, `deepseek`, `zai`.

**Process:** add `@lobehub/icons-static-svg` as a **devDependency**, run a
one-time copy of the ~21 needed files into `src/assets/providers/*.svg`
(committed to the repo), then **remove the package from `package.json`**. The
app never depends on it at runtime or build time — this keeps Milestone 0's
"no new runtime dependencies" constraint intact and avoids shipping the
package's other ~885 unused icons. Record the source and license in `NOTICE`
(same attribution pattern already used there for Claude Bar/Daybi and the
`ai-usagebar` sources).

**Fallback (no official mark available):**
- `CommandCode` → monogram `CC` in a colored circle (no logo exists anywhere).
- `SuperGrok` → `grok.svg` + a small `SUPER` text badge (shares Grok's mark;
  the user's own spec calls for this explicitly).
- `AnthropicApi` / `OpenaiAdmin` → the parent brand's icon (`anthropic.svg` /
  `openai.svg`) + a small `API` text badge.
- Any future vendor added to `VendorId` before its SVG is extracted → falls
  through to a generic monogram (first letters of `VendorInfo.name`) via
  `DEFAULT_PROVIDER_VISUAL`, never a broken image or a silently blank tab.
  A badge is always a small secondary text tag rendered next to the icon —
  never a replacement icon of its own.

Lucide (or an equivalent small icon set, still to be picked in sub-project 2
if needed) is reserved for **UI chrome only** — refresh, settings, close,
chevron, warning, key, copy — never for provider identity.

### `PROVIDER_VISUAL` registry

The backend (`VendorId` + the `VendorInfo` it already serializes: name, short
name, `authKind`, env key) stays the single source of truth for provider
*domain* data. Duplicating that in TypeScript would let the two drift. What's
missing on the frontend is purely *presentational* — icon path, accent color,
badge — so that's the only thing the new registry holds:

```ts
// src/providers.ts
export type VendorSlugId =
  | "anthropic" | "anthropic_api" | "openai" | "openai_admin" | "copilot"
  | "zai" | "openrouter" | "deepseek" | "kimi" | "kilo" | "novita"
  | "moonshot" | "grok" | "supergrok" | "antigravity" | "cursor" | "minimax"
  | "kiro" | "nous" | "opencode_go" | "commandcode" | "groq" | "windsurf";
  // hand-maintained mirror of VendorId::slug() — TS won't compile if a
  // Record<VendorSlugId, …> below is missing a key, catching drift at
  // build time; DEFAULT_PROVIDER_VISUAL is the runtime safety net for the
  // gap between a backend deploy adding a vendor and this list catching up.

export interface ProviderVisual {
  icon: string;         // path under src/assets/providers/
  accent: string;        // brand hex, used for the selected-tab underline only
  badge?: "API" | "SUPER";
}

export const PROVIDER_VISUAL: Record<VendorSlugId, ProviderVisual> = { /* 23 entries */ };
export const DEFAULT_PROVIDER_VISUAL: (name: string) => ProviderVisual = (name) => ({
  icon: monogramDataUri(name), // generated, not a file
  accent: "var(--accent-neutral)",
});
```

This single map replaces the `ACCENT` / `TAB_NAME` maps and the `glyph()`
helper currently duplicated inside `views/dash.ts` — the actual bug the user
reported ("Claude aparece correcto en una pantalla y convertido en estrella
mutante en otra").

### Provider tabs

Redesigned to: `[logo] name [status dot]`, single row, horizontal scroll (no
wrap), auto-scroll the selected tab into view, a fade at each scrollable
edge, and the selected tab gets an accent-colored underline (not a fully
recolored button — avoids 23 different button colors screaming at once). The
`+` add-provider control sits **outside** the scrollable region, fixed to the
row's trailing edge, so it's always reachable regardless of how many
providers are enabled.

## Connection status model

### Why not three independent booleans per provider

The user's own mockup (`Aplicación detectada / Cuenta detectada / Cuota
disponible`) implies three independently-probed signals. Implementing that
for real would mean adding a genuine "is the process/session alive"
probe to every one of the 23 provider modules — including ones where the
concept doesn't apply (a pure API-key provider like OpenRouter has no "app" to
detect). That's a large, provider-by-provider effort for a UI nicety. Instead:

### The model

```rust
// model.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus { Connected, NeedsAuth, NeedsPermission, Unavailable, Error }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatusReason {
    MissingCredential, InvalidCredential, MissingPermission,
    LocalServiceUnavailable, NetworkUnavailable, RateLimited, ParseFailed, Unknown,
}

pub struct ProviderSnapshot {
    pub status: ProviderStatus,
    pub status_reason: Option<ProviderStatusReason>,
    pub stale: bool,              // data freshness — orthogonal to status
    pub error: Option<String>,    // raw technical detail, never rendered directly
    // id, name, short, plan, hint, updated_at, lines, primary_utilization: unchanged
}
```

`stale` stays independent of `status`: a rate-limited refresh that still has a
good last snapshot is `status: Connected, stale: true, status_reason:
Some(RateLimited)` — not a fake "disconnected" state. This also removes
`connected: bool`, which `status != Unavailable` replaces.

**Classification is mechanical, not a new detection layer** — it reclassifies
errors the code already produces, via `map_fetch_err` in `providers/mod.rs`:

| Cause | status | status_reason |
|---|---|---|
| `has_local_credentials()` false / no token found | `NeedsAuth` | `MissingCredential` |
| `FetchError::Http(401, _)` | `NeedsAuth` | `InvalidCredential` |
| `FetchError::Http(403, _)` | `NeedsPermission` | `MissingPermission` |
| local RPC/process probe fails (Antigravity-style) | `Unavailable` | `LocalServiceUnavailable` |
| `FetchError::Network(_)` | `Unavailable` | `NetworkUnavailable` |
| `FetchError::RateLimited` with a prior good snapshot | previous `status` kept, `stale = true` | `RateLimited` |
| `FetchError::Http` any other code, or no prior snapshot on rate-limit | `Error` | `Unknown` |
| `FetchError::Parse(_)` | `Error` | `ParseFailed` |
| success | `Connected` | `None` |

401 vs 403 uses the HTTP status code alone (standard semantics: 401 =
missing/invalid credential, 403 = authenticated but not permitted) — no
substring matching on response bodies. This also retires the fragile
`error.contains("Límite de peticiones")` check flagged as audit finding
**F-M2**; rate-limit detection uses the existing `FetchError::RateLimited`
variant directly.

`loading` (used only for the UI's pre-first-refresh state) never travels over
the wire — the frontend shows it whenever it has no snapshot yet for a
provider.

### Antigravity fix

`providers/antigravity.rs::refresh()` currently collapses "local RPC
unreachable" and "no saved Google session" into one string. Split:

- `discover_local_bases()` / `fetch_local()` fail → `Unavailable` /
  `LocalServiceUnavailable` — "Antigravity no está respondiendo en el
  servicio local."
- Local RPC responds but no keyring token → `NeedsAuth` / `MissingCredential`
  — "No encontramos una sesión de Google guardada."

This directly fixes the reported bug: the message no longer claims the app
isn't running when it demonstrably is.

### Connection-guide labels

The three-line checklist the user mocked (`Aplicación / Cuenta / Cuota`) is
**not** three real backend signals — it's a fixed label template picked by
`VendorInfo.authKind`, filled in from `status` + `status_reason`:

| `authKind` | Line 1 | Line 2 | Line 3 |
|---|---|---|---|
| `local` (RPC/desktop app) | Aplicación | Cuenta | Cuota |
| `oauth` (CLI login flow) | CLI | Sesión | Cuota |
| `apiKey` | Credencial | Permisos | Cuota |
| `mixed` | Aplicación | Cuenta o credencial | Cuota |

Each line's ✓/✕/— is derived from `status`/`status_reason` (e.g. `NeedsAuth`
→ line 1 ✓, line 2 ✕, line 3 —; `NeedsPermission` → lines 1–2 ✓, line 3 ✕).
No per-provider special-casing beyond the `authKind` lookup.

## Error normalization layer

Runs entirely on the frontend, on top of the structured `status` /
`status_reason` the backend now provides. **It classifies nothing** — B
already decided what happened; this layer only decides how to say it.

```ts
// src/errors.ts
export type ProviderErrorAction =
  | "login" | "configure_credentials" | "open_settings"
  | "retry" | "copy_command" | "open_provider" | "show_details";

export interface NormalizedProviderError {
  title: string;
  message: string;
  action?: ProviderErrorAction;
  actionLabel?: string;
  technicalDetails?: string;   // sanitized, only shown behind "Ver detalles"
  missingScopes?: string[];    // only when the body has a stable, known field
  severity: "info" | "warning" | "error" | "success";
}

export function normalizeProviderError(
  snap: Pick<ProviderSnapshot, "status" | "statusReason" | "error">,
  vendor: VendorInfo,
): NormalizedProviderError | null; // null when status === "connected"
```

- **Copy table, not logic.** `STATUS_COPY[status][reason] → {title, message,
  severity, action}`, one static object, in Spanish (matches the rest of the
  UI's `i18n.ts` — this table's strings register as `I18N` keys like
  everything else). Per-vendor overrides are exceptional (e.g. OpenAI's
  `MissingPermission` message naming `api.usage.read` when a known scope
  field is present) — not a second 23-row table.
- **`sanitizeTechnicalDetails(raw: string): string`** runs before `error` is
  ever put in `technicalDetails`: masks `Authorization`, `Bearer …`,
  `api_key`, any `*token*`, `cookie`, `client_secret` (key-based redaction,
  not a permissive allowlist), and truncates to 4000 chars. This is the only
  thing standing between a provider's raw HTTP body and the "Ver detalles"
  panel, so it fails closed (masks the whole value when unsure).
- **`missingScopes`** is populated only from a known, stable structured field
  the provider's response actually documents — never parsed out of freeform
  text. If no such field is known for a provider, this stays empty and the
  generic reason message is used instead; this is deliberately narrower than
  the user's original ask to avoid re-introducing the substring-matching
  fragility that F-M2 already showed can silently break.
- **Severity → visual treatment:** color touches the status icon, card
  border, and title only. Body text stays the theme's normal foreground
  color at every severity — no full-block red alarm text. `NeedsAuth` /
  `NeedsPermission` render as `warning` (a pending action, not a failure);
  `Unavailable` / `Error` render as `error`.
- **Recovery toast is a separate concern**, not part of
  `normalizeProviderError` (which only ever sees the current snapshot). A
  thin transition watcher in the refresh-apply path compares
  `previousStatus` to the incoming `status` per provider: fires a `success`
  toast only on a real `_ → Connected` transition, never on the first refresh
  (`previousStatus === undefined`) and never repeats while status stays
  `Connected` across subsequent refreshes.

## Testing

Backend (`cargo test`, no new framework):
- `map_fetch_err`: every `FetchError` variant → the correct `(status,
  status_reason)` pair, explicitly covering `Http(401)` vs `Http(403)`.
- `RateLimited` with a prior snapshot keeps the previous `status` and sets
  `stale = true`; `RateLimited` with no prior snapshot falls to `Error` /
  `Unknown`.
- Antigravity: local-RPC-down fixture → `Unavailable`/`LocalServiceUnavailable`;
  local-RPC-up-no-token fixture → `NeedsAuth`/`MissingCredential`.

Frontend (no test framework exists today — per Milestone 0's constraints this
stays a `cargo test`-only project; these become plain assertion-based
`#[test]`-equivalent smoke checks the same way `main.ts`'s existing manual
parity pass works, or — if genuinely needed — the smallest possible
`node --test` script with zero new dependencies):
- `sanitizeTechnicalDetails`: token/Bearer/cookie/client_secret values are
  masked, output length is capped.
- `normalizeProviderError`: every `(status, status_reason)` combination the
  backend can emit resolves to a non-empty `STATUS_COPY` entry — no silent
  fallback to blank text.
- Recovery-toast transition logic: no toast on first refresh, no repeat
  toast across consecutive `Connected` refreshes, fires exactly once on a
  real recovery transition.

Manual smoke test (with the user, same pattern as Milestone 0 Part 1): tabs
render every enabled provider's real logo, no broken images; disconnect a
provider's credential and confirm the connection-guide checklist and error
card both switch correctly; force a 403 (or fixture one) and confirm the
message reads as MissingPermission wording, not raw JSON.

## File structure

| File | Change |
|---|---|
| `src-tauri/src/model.rs` | add `ProviderStatus`, `ProviderStatusReason`; `ProviderSnapshot` gains `status`/`status_reason`, drops `connected` |
| `src-tauri/src/http.rs` | `FetchError` unchanged; new mapping fn (or extend `providers/mod.rs::map_fetch_err`) to `(ProviderStatus, ProviderStatusReason)` |
| `src-tauri/src/providers/mod.rs` | `map_fetch_err` returns structured status instead of a flattened string; retire the `.contains("Límite de peticiones")` check (F-M2) |
| `src-tauri/src/providers/antigravity.rs` | split the local-RPC-down vs no-token paths |
| `src-tauri/src/providers/*.rs` (remaining 22) | sweep: replace any other ad-hoc `snapshot_err(id, "custom string")` with the correct status/reason instead of falling through to `Error`/`Unknown` |
| `src/providers.ts` | **new** — `PROVIDER_VISUAL`, `VendorSlugId`, `DEFAULT_PROVIDER_VISUAL` |
| `src/errors.ts` | **new** — `normalizeProviderError`, `sanitizeTechnicalDetails`, `STATUS_COPY`, `ProviderErrorAction` |
| `src/assets/providers/*.svg` | **new** — ~21 files extracted from `@lobehub/icons-static-svg`, then the package is removed from `package.json` |
| `src/views/dash.ts` | tabs redesigned (logo/name/dot, scroll, fixed `+`); drop local `ACCENT`/`TAB_NAME`/`glyph()` in favor of `providers.ts` |
| `src/api.ts` | `ProviderSnapshot`/`VendorInfo` TS types updated to match the new Rust shape |
| `NOTICE` | add `@lobehub/icons-static-svg` attribution |

## Self-review

- **Placeholders:** none — every field, function signature, and mapping table
  above is concrete.
- **Internal consistency:** `stale` is independent of `status` everywhere it's
  mentioned (model, table, Antigravity section, testing). `loading` is
  frontend-only and stated once, referenced consistently. The connection-guide
  label table and the `PROVIDER_VISUAL` fallback both explicitly handle "a
  vendor this code doesn't know about yet" without a broken/blank UI.
- **Scope:** matches the "Explicitly out of scope" list; every item there maps
  to a named consumer in sub-project 2's future spec, not a silent drop.
- **Ambiguity resolved:** 401→`InvalidCredential`, 403→`MissingPermission` is
  stated as the HTTP-status-only rule (no body inspection) to avoid the two
  readers-could-disagree case of "what if 403 really means invalid token".
  `missingScopes` explicitly degrades to empty rather than attempting
  freeform extraction, closing the one place the design could have quietly
  reintroduced F-M2-style fragility.
