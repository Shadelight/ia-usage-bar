# Usage Semantics, Notifications & Settings

Date: 2026-09-11
Status: Implemented
Owner: Alberth Salazar
Depends on: `2026-09-11-provider-identity-foundation.md`

## Goal

Make provider data trustworthy before further visual work: every adapter emits the same
meaning for usage, remaining quota, resets, credits, product breakdown, temporary limits,
and freshness; the UI only formats that canonical data. Complete the Windows utility
behavior and the settings/alert work explicitly deferred by the provider-identity spec.

## Canonical usage contract

```ts
interface UsageQuota {
  id: string;
  label: string;
  windowType: "session" | "5h" | "daily" | "weekly" | "monthly" | "credits" | "custom";
  usedPercent: number | null;
  remainingPercent: number | null;
  usedAmount: number | null;
  limitAmount: number | null;
  unit: "percent" | "credits" | "usd" | "requests" | "tokens" | null;
  resetAt: string | null;
  resetInSeconds: number | null;
  resetStatus: "known" | "not_provided" | "not_applicable" | "fetch_failed";
  temporaryMultiplier: number | null;
  temporaryExpiresAt: string | null;
  source: "cli" | "oauth" | "api" | "local-session" | "web-session";
  fetchedAt: string;
  stale: boolean;
}

interface ProviderUsage {
  id: string;
  plan: string;
  status: ProviderStatus;
  statusReason: ProviderStatusReason | null;
  stale: boolean;
  quotas: UsageQuota[];
  credits: { remaining: number; resetsAvailable: number | null } | null;
  productBreakdown: { name: string; usedPercent: number }[];
  cost: { today: number | null; week: number | null; thirtyDays: number | null; month: number | null } | null;
}
```

Rust owns the serialized contract in `src-tauri/src/model.rs`; `src/api.ts` mirrors it.
Legacy `MetricLine` remains as a compatibility projection while adapters migrate, but the
UI's quota blocks consume `UsageQuota`.

## Percentage semantics

- The large number always means **used percent**.
- The supporting number always means **available percent**.
- If a provider reports remaining percent, its adapter performs
  `used = 100 - remaining` exactly once.
- Store and UI code never invert or reinterpret percentages.
- Values are clamped to `[0, 100]` at normalization boundaries.

Required examples:

| Provider value | Canonical/UI value |
| --- | --- |
| Codex 5-hour `100% remaining` | `0% used`, `100% available` |
| Codex weekly `71% remaining` | `29% used`, `71% available` |
| Claude session `3% utilization` | `3% used`, `97% available` |

## Reset semantics

Adapters preserve the provider's reset instant as an RFC 3339 timestamp. A known reset
renders both:

```text
Reinicia en 4 h 35 min
Hoy, 13:40
```

`formatResetRelative(resetAt)` and `formatResetAbsolute(resetAt)` live in
`src/reset-format.ts`. The relative clock updates once per minute in the webview without a
provider fetch. Unknown states render explicit copy from `resetStatus`; `windowType` is
never substituted into the reset label.

## Provider-specific normalization

### Claude

Preserve session and weekly usage/reset data, weekly product breakdown, and temporary
limit multipliers/expiry. Product breakdown labels represent Claude Code, Chats, Cowork,
and Other when supplied; they are not model-level Sonnet/Opus quota rows.

### Codex / ChatGPT

Normalize 5-hour and weekly windows, weekly reset, remaining credits, and reset count.
Credit balances display with zero fractional digits by flooring the provider balance.

## Freshness and failure behavior

- Provider errors use the structured identity status/reason contract.
- A non-authoritative refresh failure retains the last valid metrics and marks them stale.
- A rate-limited refresh with prior data remains connected/stale with reason
  `rate_limited`; without prior data it becomes generic error/unknown.
- The dashboard does not probe credentials while rendering. Catalog detection is cached
  and recomputed only after detection or relevant configuration changes.
- When no provider is enabled, refresh is a no-op rather than a 23-provider fan-out.
- Tauri command failures render an accessible error toast; initial dashboard failure
  renders a retry state instead of a blank panel.
- Before the first provider snapshot, enabled provider tabs show a UI-only loading state
  and skeleton; `loading` never crosses the backend contract.

## Settings and credential storage

- Launch at login reflects the actual `tauri-plugin-autostart` OS state and can be toggled
  from Settings or the tray menu.
- Notification thresholds live in `AppConfig.notify_thresholds`, default to
  `[75, 90, 95]`, and are validated to `1..=99`, sorted, and deduplicated.
- A provider/threshold alert fires once per upward crossing within a quota window; reset
  starts a new window and clears the dedupe set.
- Manual API keys are stored in Windows Credential Manager through `keyring`. A legacy
  plaintext value is removed from `config.toml` only after secure migration succeeds.
- Config and app-owned OAuth cache writes use same-directory temporary files plus a
  recoverable replacement sequence; persistence errors return through Tauri commands.

## Windows window behavior

- The custom header provides Minimize and Close controls with accessible labels.
- Minimize uses native Tauri minimize and keeps the window in the taskbar/Alt+Tab.
- Close is intercepted, hides the main window, and leaves the process in the tray.
- Tray click shows/focuses the existing main window; it never creates another window.
- Tray menu contains Open, Refresh, Settings, and Exit. Exit is the explicit process
  termination path.
- The main window uses `skipTaskbar: false` and the application branding across executable,
  window, taskbar, Alt+Tab, tray, and installers.

## Tests and verification

Automated coverage includes:

- remaining-to-used conversion and credit flooring;
- Claude/Codex reset parsing, time zones, relative and absolute formatting;
- unknown reset, product breakdown, temporary limits, and stale propagation;
- structured fetch-error mapping, technical-detail redaction, and recovery transitions;
- notification threshold validation/deduplication;
- atomic/recoverable writes, corrupt config recovery, refresh overlap, and poisoned locks.

Required gates:

```powershell
npm run build
npm run test:frontend
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri build
```

Manual Windows verification remains required for real provider accounts, taskbar/tray
interaction, autostart after login, notification delivery, and light/dark taskbar icon
legibility.

## Out of scope

The Rust-core/collector/Android-companion architecture document is a roadmap, not part of
this implementation. Publishing a tag, changing GitHub metadata, or rewriting Git history
also requires explicit external/user approval.
