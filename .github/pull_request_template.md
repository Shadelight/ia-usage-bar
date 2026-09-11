## Summary

Describe the user-visible outcome and why the change is needed.

## Verification

- [ ] `npm run build`
- [ ] `npm run test:frontend`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml --locked`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] Manual Windows smoke test completed when behavior or UI changed

## Safety

- [ ] No credentials, session files, personal provider responses, or generated build artifacts are included
- [ ] Provider errors and development logs remain sanitized
- [ ] Screenshots are attached for visual changes
