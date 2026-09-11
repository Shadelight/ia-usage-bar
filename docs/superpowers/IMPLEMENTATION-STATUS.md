# Superpowers implementation status

Updated: 2026-09-11

This index separates executable requirements from roadmap material and records what is
still intentionally outside an automated repository change.

## Document disposition

| Document | Disposition |
| --- | --- |
| `specs/2026-09-11-provider-identity-foundation.md` | Implemented: structured provider status/reasons, Antigravity state split, one frontend visual registry, local provider assets, error normalization/sanitization, recovery transitions, connection checklists, and tests. |
| `specs/2026-09-11-usage-semantics-notifications-settings.md` | Implemented: canonical quota/reset semantics, stale-data retention, loading/error states, Windows window behavior, secure credential storage, autostart, configurable alerts, and verification gates. |
| `specs/2026-09-10-ia-usage-bar-stabilization.md` | User-facing and release requirements implemented. The older draft's proposed `ProviderDescriptor`/one-folder-per-provider topology is superseded for this milestone by the later approved identity decision that keeps `VendorId` + serialized `VendorInfo` as backend domain truth. Adapters remain separated by integration type and emit the canonical usage contract. |
| `plans/2026-09-10-milestone-0-stabilization.md` | Phases 0–1 implemented: audit, rename cleanup, focused Rust/TypeScript modules, current remote path, and Rust tests in CI. Historical commit choreography was not replayed because it would rewrite the user's current working history. |
| `specs/2026-09-10-ia-usage-hub-architecture.md` | Roadmap only, as stated by the document itself. Android companion, sync protocol, CLI, SQLite event store, and later milestones require their own approved design and plan. |

## Completed repository work

- Canonical quota model: used/remaining percentages, amounts, units, window type,
  source, freshness, explicit reset status, ISO reset, relative reset, credits, reset
  count, product breakdown, temporary limit, and costs.
- Claude and Codex adapters retain the existing correct percentage semantics and now
  preserve resets and provider metadata without UI reinterpretation.
- Relative reset clocks update locally each minute; absolute reset formatting is
  timezone-aware and both forms are tested.
- Non-authoritative refresh failures retain the last valid metrics as stale data.
- Settings expose real Windows autostart state and validated notification thresholds;
  alerts deduplicate once per provider/threshold/window.
- Manual API keys use Windows Credential Manager. Legacy plaintext values migrate only
  after secure storage succeeds.
- Provider detection/config writes are serialized and recoverable; the catalog is cached;
  an empty provider selection performs no 23-provider refresh fan-out.
- Main-window minimize/taskbar/Alt+Tab, close-to-tray, tray reopen, and explicit exit
  behavior are implemented.
- Contributor, changelog, security, issue/PR templates, build CI, and tag-driven Windows
  release automation are present. Current local NSIS and MSI bundle artifacts were
  generated; WiX ICE validation is left to the clean GitHub runner because the local
  Windows Installer service became unavailable to ICE after the first successful pass.

## Deliberately manual or externally gated

- Verify real provider accounts, tray legibility on light/dark taskbars, autostart across
  a Windows login, and the close/minimize/tray sequence on the user's desktop.
- Push a test `v*` tag and approve the draft GitHub Release.
- Set GitHub repository description/topics.
- Any archive tag, orphan branch, squash, or force-push requires a separate explicit user
  approval because it rewrites published history.
