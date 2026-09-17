//! Catálogo de proveedores y orquestación de refresh.

mod antigravity;
mod apikey;
mod claude;
mod codex;
mod copilot;
mod cursor;
mod kiro;
mod local;
mod openai_admin;

use std::collections::HashSet;

use crate::config::AppConfig;
use crate::descriptor::descriptor;
use crate::http::FetchError;
use crate::model::{
    snapshot_needs_auth, snapshot_with_status, AuthKind, ProviderSnapshot, ProviderStatus,
    ProviderStatusReason, VendorId, VendorInfo,
};

pub trait Provider {
    fn id(&self) -> VendorId;
    fn has_local_credentials(&self, cfg: &AppConfig) -> bool;
    fn refresh(&self, cfg: &AppConfig) -> ProviderSnapshot;
}

pub fn has_local_credentials(id: VendorId, cfg: &AppConfig) -> bool {
    dispatch(id).has_local_credentials(cfg)
}

pub fn refresh(id: VendorId, cfg: &AppConfig) -> ProviderSnapshot {
    let provider = dispatch(id);
    debug_assert_eq!(provider.id(), id);
    let snapshot = provider.refresh(cfg);
    #[cfg(debug_assertions)]
    if let Ok(mut value) = serde_json::to_value(&snapshot) {
        crate::http::redact_json(&mut value);
        eprintln!("[usage normalized {}] {value}", id.slug());
    }
    if snapshot.id != id.slug() {
        debug_assert_eq!(
            snapshot.id,
            id.slug(),
            "ProviderSnapshot.id must equal the requested provider"
        );
        return snapshot_with_status(
            id,
            ProviderStatus::Error,
            ProviderStatusReason::Unknown,
            "snapshot providerId mismatch",
        );
    }
    snapshot
}

/// Actualización local de Codex, usada por el watcher persistente. Mantiene
/// las sesiones como optimización de eventos; `refresh` sigue siendo la ruta
/// autoritativa que consulta el endpoint remoto en el intervalo controlado.
pub fn codex_snapshot_from_sessions() -> Option<ProviderSnapshot> {
    codex::snapshot_from_sessions()
}

pub fn catalog(cfg: &AppConfig) -> Vec<VendorInfo> {
    // Startup must not synchronously probe every CLI/account. `run_detect`
    // refreshes this small persisted set in the background.
    let detected = crate::config::detected_ids();
    VendorId::all()
        .iter()
        .copied()
        .map(|id| {
            let desc = descriptor(id);
            let has_credential = credential_present(id, cfg, &detected);
            let credential_source = cfg.credential_source(id);
            VendorInfo {
                id: id.slug().to_string(),
                name: id.display_name().to_string(),
                short: id.short().to_string(),
                auth_kind: id.auth_kind(),
                env_key: id.env_key().map(|s| s.to_string()),
                hint: id.login_hint().to_string(),
                needs_key: id.needs_api_key_ui(),
                enabled: cfg.is_enabled(id),
                detected: detected.contains(id.slug()),
                has_credential,
                credential_source,
                links: id.links(),
                strategies: desc
                    .strategies
                    .iter()
                    .map(|s| s.slug().to_string())
                    .collect(),
                // Do not surface stale preferences saved by pre-router builds:
                // if the fetcher cannot execute it, it is not a selectable source.
                source_preference: cfg
                    .source_preference(id)
                    .filter(|source| desc.strategies.contains(source))
                    .map(|s| s.slug().to_string()),
            }
        })
        .collect()
}

/// Whether the UI may claim a usable credential exists. `detected` records a
/// locally observed login/session and must never resurrect a deleted API key:
/// pure key vendors derive presence from the secret store (env/keyring) only.
fn credential_present(id: VendorId, cfg: &AppConfig, detected: &HashSet<String>) -> bool {
    match id.auth_kind() {
        AuthKind::ApiKey => cfg.api_key(id).is_some(),
        AuthKind::Mixed => cfg.api_key(id).is_some() || detected.contains(id.slug()),
        AuthKind::Oauth | AuthKind::Local => detected.contains(id.slug()),
    }
}

fn dispatch(id: VendorId) -> Box<dyn Provider> {
    match id {
        VendorId::Anthropic => Box::new(claude::Claude),
        VendorId::Openai => Box::new(codex::Codex),
        VendorId::Cursor => Box::new(cursor::Cursor),
        VendorId::Antigravity => Box::new(antigravity::Antigravity),
        VendorId::OpenaiAdmin => Box::new(openai_admin::OpenaiAdmin),
        VendorId::Copilot => Box::new(copilot::Copilot),
        VendorId::Openrouter => Box::new(apikey::OpenRouter),
        VendorId::Zai => Box::new(apikey::Zai),
        VendorId::Deepseek => Box::new(apikey::DeepSeek),
        VendorId::Grok => Box::new(apikey::Grok),
        VendorId::Kilo => Box::new(apikey::Kilo),
        VendorId::Novita => Box::new(apikey::Novita),
        VendorId::Moonshot => Box::new(apikey::Moonshot),
        VendorId::Minimax => Box::new(apikey::Minimax),
        VendorId::AnthropicApi => Box::new(apikey::AnthropicApi),
        VendorId::OpenCodeGo => Box::new(apikey::OpenCodeGo),
        VendorId::Kimi => Box::new(local::Kimi),
        VendorId::Kiro => Box::new(kiro::Kiro),
        VendorId::Supergrok => Box::new(local::SuperGrok),
        VendorId::CommandCode => Box::new(local::CommandCode),
        VendorId::Nous => Box::new(local::Nous),
        VendorId::Groq => Box::new(apikey::Groq),
        VendorId::Windsurf => Box::new(local::Windsurf),
    }
}

pub fn map_fetch_err(id: VendorId, e: FetchError) -> ProviderSnapshot {
    let message = e.message();
    let (status, reason, stale) = match e {
        FetchError::RateLimited => (
            ProviderStatus::Error,
            ProviderStatusReason::RateLimited,
            true,
        ),
        FetchError::Http(401, _) => (
            ProviderStatus::NeedsAuth,
            ProviderStatusReason::InvalidCredential,
            false,
        ),
        FetchError::Http(403, _) => (
            ProviderStatus::NeedsPermission,
            ProviderStatusReason::MissingPermission,
            false,
        ),
        FetchError::Network(_) => (
            ProviderStatus::Unavailable,
            ProviderStatusReason::NetworkUnavailable,
            false,
        ),
        FetchError::Parse(_) => (
            ProviderStatus::Error,
            ProviderStatusReason::ParseFailed,
            false,
        ),
        FetchError::Http(_, _) => (ProviderStatus::Error, ProviderStatusReason::Unknown, false),
    };
    let mut snap = snapshot_with_status(id, status, reason, &message);
    snap.stale = stale;
    snap
}

pub fn key_or_err(id: VendorId, cfg: &AppConfig) -> Result<String, ProviderSnapshot> {
    cfg.api_key(id)
        .ok_or_else(|| snapshot_needs_auth(id, id.login_hint()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok};

    #[test]
    fn claude_mapper_fixture() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 12.0, "resets_at": "2099-01-01T00:00:00Z"},
            "seven_day": {"utilization": 40.0, "resets_at": "2099-01-08T00:00:00Z"}
        });
        let snap = claude::snapshot_from_json("Max 20x", &body);
        assert!(snap.is_connected());
        assert_eq!(snap.plan, "Max 20x");
        assert_eq!(snap.lines.len(), 2);
    }

    #[test]
    fn codex_classifies_windows_by_duration() {
        let body = serde_json::json!({
            "plan_type": "plus",
            "rate_limit": {
                "primary_window": {"used_percent": 10, "limit_window_seconds": 18000, "reset_at": 4102444800i64},
                "secondary_window": {"used_percent": 30, "limit_window_seconds": 604800, "reset_at": 4103049600i64}
            }
        });
        let snap = codex::snapshot_from_json(&body, Some("plus"));
        assert_eq!(snap.lines[0].id(), "5h");
        assert_eq!(snap.lines[1].id(), "weekly");
    }

    #[test]
    fn cursor_mapper_fixture() {
        let body = serde_json::json!({
            "membershipType": "ultra",
            "billingCycleEnd": "2099-02-01T00:00:00.000Z",
            "individualUsage": {
                "plan": {
                    "autoPercentUsed": 42.0,
                    "apiPercentUsed": 10.0,
                    "totalPercentUsed": 38.0
                }
            }
        });
        let snap = cursor::snapshot_from_json(&body).unwrap();
        assert_eq!(snap.plan, "Ultra");
        assert!(snap.lines.iter().any(|l| l.id() == "cursor_models"));
    }

    #[test]
    fn kiro_mapper_fixture() {
        let body = serde_json::json!({
            "nextDateReset": 4102444800.0,
            "subscriptionInfo": { "subscriptionTitle": "KIRO POWER" },
            "usageBreakdownList": [{
                "resourceType": "CREDIT",
                "currentUsageWithPrecision": 2500.0,
                "usageLimitWithPrecision": 10000.0
            }]
        });
        let snap = kiro::snapshot_from_json(&body);
        assert_eq!(snap.plan, "KIRO POWER");
        assert_eq!(snap.lines[0].utilization().map(|n| n.round()), Some(25.0));
    }

    #[test]
    fn copilot_mapper_fixture() {
        let body = serde_json::json!({
            "copilot_plan": "individual",
            "quota_reset_date_utc": "2099-02-01T00:00:00Z",
            "quota_snapshots": {
                "premium_interactions": { "percent_remaining": 40.0 },
                "chat": { "percent_remaining": 80.0 }
            }
        });
        let snap = copilot::snapshot_from_json(&body);
        assert_eq!(snap.plan, "individual");
        assert_eq!(
            snap.lines
                .iter()
                .find(|l| l.id() == "premium_interactions")
                .and_then(|l| l.utilization())
                .map(|n| n.round()),
            Some(60.0)
        );
    }

    #[test]
    fn antigravity_mapper_remaining_fraction() {
        let body = serde_json::json!({
            "response": {
                "groups": [{
                    "displayName": "Gemini Models",
                    "buckets": [
                        {"bucketId": "gemini-5h", "displayName": "Five Hour", "window": "5h",
                         "remainingFraction": 0.5, "resetTime": "2099-01-01T00:00:00Z"}
                    ]
                }]
            }
        });
        let snap = antigravity::snapshot_from_quota(&body, "Google AI Pro");
        assert_eq!(snap.plan, "Google AI Pro");
        assert!(snap
            .lines
            .iter()
            .any(|l| l.utilization().map(|n| n.round()) == Some(50.0)));
    }

    #[test]
    fn progress_line_helper() {
        let line = progress_pct("x", "X", 99.0, None, 3600, "always");
        assert_eq!(line.utilization(), Some(99.0));
        let _ = snapshot_ok(VendorId::Deepseek, "PayG", vec![line]);
    }

    #[test]
    fn fetch_errors_map_to_structured_statuses() {
        use crate::model::{ProviderStatus, ProviderStatusReason};

        let snap = map_fetch_err(VendorId::Anthropic, crate::http::FetchError::RateLimited);
        assert_eq!(snap.status, ProviderStatus::Error);
        assert_eq!(snap.status_reason, Some(ProviderStatusReason::RateLimited));
        assert!(snap.stale);

        let invalid = map_fetch_err(
            VendorId::Anthropic,
            crate::http::FetchError::Http(401, String::new()),
        );
        assert_eq!(invalid.status, ProviderStatus::NeedsAuth);
        assert_eq!(
            invalid.status_reason,
            Some(ProviderStatusReason::InvalidCredential)
        );

        let denied = map_fetch_err(
            VendorId::Anthropic,
            crate::http::FetchError::Http(403, String::new()),
        );
        assert_eq!(denied.status, ProviderStatus::NeedsPermission);
        assert_eq!(
            denied.status_reason,
            Some(ProviderStatusReason::MissingPermission)
        );

        let offline = map_fetch_err(
            VendorId::Anthropic,
            crate::http::FetchError::Network("offline".into()),
        );
        assert_eq!(offline.status, ProviderStatus::Unavailable);
        assert_eq!(
            offline.status_reason,
            Some(ProviderStatusReason::NetworkUnavailable)
        );

        let malformed = map_fetch_err(
            VendorId::Anthropic,
            crate::http::FetchError::Parse("bad json".into()),
        );
        assert_eq!(malformed.status, ProviderStatus::Error);
        assert_eq!(
            malformed.status_reason,
            Some(ProviderStatusReason::ParseFailed)
        );

        let server = map_fetch_err(
            VendorId::Anthropic,
            crate::http::FetchError::Http(500, String::new()),
        );
        assert_eq!(server.status, ProviderStatus::Error);
        assert_eq!(server.status_reason, Some(ProviderStatusReason::Unknown));
    }

    #[test]
    fn catalog_exposes_credential_presence_without_leaking_secrets() {
        // DEEPSEEK_API_KEY is unique to this test; no other test reads it.
        std::env::set_var("DEEPSEEK_API_KEY", "sk-test-must-never-serialize");
        let cat = catalog(&AppConfig::default());
        std::env::remove_var("DEEPSEEK_API_KEY");
        let deepseek = cat
            .iter()
            .find(|v| v.id == "deepseek")
            .expect("deepseek in catalog");
        assert!(
            deepseek.has_credential,
            "env credential must surface as a boolean"
        );
        let grok = cat
            .iter()
            .find(|v| v.id == "grok")
            .expect("grok in catalog");
        assert!(
            !grok.has_credential,
            "unset credential must surface as false"
        );
        let serialized = serde_json::to_string(&cat).unwrap();
        assert!(
            !serialized.contains("sk-test-must-never-serialize"),
            "the secret itself must never reach the frontend"
        );
    }

    #[test]
    fn stale_detected_entry_never_resurrects_a_deleted_api_key() {
        // KILO_API_KEY is unique to this test; no other test reads it.
        std::env::remove_var("KILO_API_KEY");
        let cfg = AppConfig::default();
        let detected: HashSet<String> = ["kilo".into()].iter().cloned().collect();
        assert!(
            !credential_present(VendorId::Kilo, &cfg, &detected),
            "a stale detected.json entry must not fake an ApiKey credential"
        );
        std::env::set_var("KILO_API_KEY", "sk-test-kilo-present");
        assert!(
            credential_present(VendorId::Kilo, &cfg, &HashSet::new()),
            "a stored key alone must count for ApiKey vendors"
        );
        std::env::remove_var("KILO_API_KEY");
    }

    #[test]
    fn mixed_vendors_accept_either_secret_or_local_login() {
        std::env::remove_var("GITHUB_COPILOT_TOKEN");
        let cfg = AppConfig::default();
        let detected: HashSet<String> = ["copilot".into()].iter().cloned().collect();
        assert!(credential_present(VendorId::Copilot, &cfg, &detected));
        assert!(!credential_present(
            VendorId::Copilot,
            &cfg,
            &HashSet::new()
        ));
    }

    #[test]
    fn oauth_and_local_vendors_derive_presence_from_detection_only() {
        let cfg = AppConfig::default();
        let detected: HashSet<String> = ["anthropic".into(), "cursor".into()]
            .iter()
            .cloned()
            .collect();
        assert!(credential_present(VendorId::Anthropic, &cfg, &detected));
        assert!(credential_present(VendorId::Cursor, &cfg, &detected));
        assert!(!credential_present(
            VendorId::Anthropic,
            &cfg,
            &HashSet::new()
        ));
        assert!(!credential_present(VendorId::Openai, &cfg, &detected));
    }

    #[test]
    fn credential_links_are_official_https_or_absent() {
        for id in VendorId::all().iter().copied() {
            let links = id.links();
            for url in [links.api_key_url.as_deref(), links.signup_url.as_deref()]
                .into_iter()
                .flatten()
            {
                assert!(
                    url.starts_with("https://"),
                    "{} credential link must be https: {url}",
                    id.slug()
                );
            }
            let is_key_vendor = matches!(
                id.auth_kind(),
                crate::model::AuthKind::ApiKey | crate::model::AuthKind::Mixed
            ) && id.env_key().is_some();
            if !is_key_vendor {
                assert!(
                    links.api_key_url.is_none() && links.signup_url.is_none(),
                    "{} is not a key vendor and must not show credential links",
                    id.slug()
                );
            }
        }
        // The reported cases stay covered end to end.
        let openrouter = VendorId::Openrouter.links();
        assert_eq!(
            openrouter.api_key_url.as_deref(),
            Some("https://openrouter.ai/settings/keys")
        );
        assert!(openrouter.signup_url.is_some());
        let go = VendorId::OpenCodeGo.links();
        assert_eq!(go.api_key_url.as_deref(), Some("https://opencode.ai/auth"));
        assert!(go.signup_url.is_some());
    }
}
