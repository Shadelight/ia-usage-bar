# Security Policy

## Supported versions

Security fixes are provided for the latest published release.

## Credential handling

API keys entered in Settings are stored in Windows Credential Manager through the Rust `keyring` crate. Existing plaintext credentials from early development builds are migrated on startup and removed from `config.toml` only after secure storage succeeds. Provider-owned OAuth and session files are read locally and are not copied into the repository.

IA Usage Bar has no telemetry. Network requests are sent only to the endpoints required by enabled providers. Development logging redacts tokens, authorization headers, cookies, secrets, API keys, and email fields.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting flow: open the repository's **Security** tab and select **Report a vulnerability**. Do not open a public issue containing credentials, private provider responses, or exploit details.

Include the affected version, reproduction steps, impact, and any suggested mitigation. You should receive an acknowledgement within seven days.
