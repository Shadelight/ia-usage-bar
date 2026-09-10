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

use crate::config::AppConfig;
use crate::http::FetchError;
use crate::model::{snapshot_err, ProviderSnapshot, VendorId, VendorInfo};

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
    provider.refresh(cfg)
}

pub fn catalog(cfg: &AppConfig) -> Vec<VendorInfo> {
    VendorId::all()
        .iter()
        .copied()
        .map(|id| VendorInfo {
            id: id.slug().to_string(),
            name: id.display_name().to_string(),
            short: id.short().to_string(),
            auth_kind: id.auth_kind(),
            env_key: id.env_key().map(|s| s.to_string()),
            hint: id.login_hint().to_string(),
            needs_key: id.needs_api_key_ui(),
            enabled: cfg.is_enabled(id),
            detected: has_local_credentials(id, cfg),
        })
        .collect()
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
    let mut snap = snapshot_err(id, &e.message());
    match e {
        FetchError::RateLimited => {
            snap.connected = true;
            snap.stale = true;
        }
        FetchError::Http(401 | 403, _) => {
            snap.hint = Some(id.login_hint().to_string());
        }
        FetchError::Network(_) => {
            snap.stale = true;
            snap.connected = true;
        }
        _ => {}
    }
    snap
}

pub fn key_or_err(id: VendorId, cfg: &AppConfig) -> Result<String, ProviderSnapshot> {
    cfg.api_key(id).ok_or_else(|| snapshot_err(id, id.login_hint()))
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
        assert!(snap.connected);
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
        assert_eq!(snap.lines[0].id(), "session");
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
            snap.lines.iter().find(|l| l.id() == "premium_interactions").and_then(|l| l.utilization()).map(|n| n.round()),
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
        assert!(snap.lines.iter().any(|l| l.utilization().map(|n| n.round()) == Some(50.0)));
    }

    #[test]
    fn progress_line_helper() {
        let line = progress_pct("x", "X", 99.0, None, 3600, "always");
        assert_eq!(line.utilization(), Some(99.0));
        let _ = snapshot_ok(VendorId::Deepseek, "PayG", vec![line]);
    }

    #[test]
    fn rate_limited_stays_connected_and_stale() {
        let snap = map_fetch_err(VendorId::Anthropic, crate::http::FetchError::RateLimited);
        assert!(snap.connected);
        assert!(snap.stale);
    }
}
