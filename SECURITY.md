# Security Policy

## Supported versions

Security fixes are provided for the latest published release.

## Credential handling

**Your credentials stay on your machine.** IA Usage reuses sessions and credentials that already exist locally where a provider allows it — OAuth/CLI login, device flows, local session files — instead of asking you to re-authenticate.

API keys entered in Settings are stored in Windows Credential Manager through the Rust `keyring` crate. Existing plaintext credentials from early development builds are migrated on startup and removed from `config.toml` only after secure storage succeeds. Provider-owned OAuth and session files are read locally and are not copied into the repository.

IA Usage has no telemetry. Network requests are sent only to the endpoints required by enabled providers. Development logging redacts tokens, authorization headers, cookies, secrets, API keys, and email fields.

| Data | Where it lives |
|---|---|
| Claude / Codex / Cursor session | Managed by the original CLI/app, read locally |
| Manually entered API keys | Windows Credential Manager |
| App configuration | Local (`%APPDATA%\ia-usagebar\config.toml`) |
| Cached usage snapshots | Local |
| Passwords | **Never stored** |
| Telemetry | **None** |

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting flow: open the repository's **Security** tab and select **Report a vulnerability**. Do not open a public issue containing credentials, private provider responses, or exploit details.

Include the affected version, reproduction steps, impact, and any suggested mitigation. You should receive an acknowledgement within seven days.
