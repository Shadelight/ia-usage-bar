# Contributing to IA Usage

Thanks for helping improve IA Usage. The application targets Windows 10/11 and uses Tauri 2, Rust, TypeScript, and Vite.

## Development setup

Install Rust stable, Node.js 20, and the Windows prerequisites listed in the [Tauri documentation](https://v2.tauri.app/start/prerequisites/). Then run:

```powershell
npm install
npm run tauri dev
```

Before opening a pull request, run:

```powershell
npm run build
npm run test:frontend
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

## Project layout

- `src/`: UI bootstrap, provider visuals, reset formatting, and focused views.
- `src-tauri/src/providers/`: provider authentication, fetching, and normalization adapters.
- `src-tauri/src/model.rs`: canonical provider, quota, reset, credit, product, and cost contracts.
- `src-tauri/src/dashboard.rs`: refresh orchestration, stale-data retention, and alerts.
- `docs/superpowers/`: approved implementation specifications and longer-term architecture.

## Adding or changing a provider

1. Keep provider-specific parsing and authentication in `src-tauri/src/providers/`.
2. Normalize the response into `ProviderSnapshot`/`UsageQuota`; the large percentage is always usage, never remaining quota.
3. Give every failure a `ProviderStatus` and `ProviderStatusReason`. Never infer authentication failure from arbitrary error text.
4. Add the provider visual identity in `src/providers.ts` and place a local SVG asset under `src/assets/providers/` when an official mark is available.
5. Add fixture-style tests for percentages, resets, time zones, missing fields, and sanitized failures. Tests must not call real provider APIs.
6. Never log credentials, cookies, authorization headers, or raw session files.

The provider-identity contract is documented in [`docs/superpowers/specs/2026-09-11-provider-identity-foundation.md`](docs/superpowers/specs/2026-09-11-provider-identity-foundation.md).

## Pull requests

Keep changes focused, explain user-visible behavior, include tests for logic changes, and attach screenshots for visual changes. Do not commit secrets, generated `target/` or `dist/` artifacts, or provider response fixtures containing personal data.
