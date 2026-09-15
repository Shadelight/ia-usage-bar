//! Claude Code — OAuth local + endpoint de uso de Anthropic.

use serde_json::Value;
use std::sync::OnceLock;

use crate::config::AppConfig;
use crate::cost;
use crate::http::{self, FetchError};
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok, snapshot_with_status,
    values_line, ProductUsage, ProviderSnapshot, ProviderStatus, ProviderStatusReason, UsageCost,
    VendorId,
};
use crate::paths::claude_dir;

use super::Provider;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const ANTHROPIC_BETA: &str = "oauth-2025-04-20";
const FALLBACK_VERSION: &str = "2.1.139";
static CLI_VERSION: OnceLock<String> = OnceLock::new();

pub struct Claude;

impl Provider for Claude {
    fn id(&self) -> VendorId {
        VendorId::Anthropic
    }

    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        read_token().is_some()
    }

    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let Some((token, plan)) = read_token() else {
            return snapshot_needs_auth(VendorId::Anthropic, "Claude Code no conectado");
        };
        match fetch_usage(&token) {
            Ok(body) => {
                let mut snap = snapshot_from_json(&plan, &body);
                append_cost(&mut snap);
                snap
            }
            // Claude Code owns token renewal. A 401 from its OAuth usage
            // endpoint means the local session needs `claude` to sign in
            // again; presenting it as a generic bad credential is misleading.
            Err(FetchError::Http(401, detail)) => snapshot_with_status(
                VendorId::Anthropic,
                ProviderStatus::NeedsAuth,
                ProviderStatusReason::OAuthExpired,
                &format!("La sesión OAuth de Claude Code venció. Ejecuta `claude` para volver a iniciar sesión. {detail}"),
            ),
            Err(e) => super::map_fetch_err(VendorId::Anthropic, e),
        }
    }
}

#[derive(serde::Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OauthBlock>,
}

#[derive(serde::Deserialize)]
struct OauthBlock {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier")]
    rate_limit_tier: Option<String>,
}

fn pretty_plan(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "max" => "Max".into(),
        "pro" => "Pro".into(),
        "free" => "Free".into(),
        other if !other.is_empty() => {
            let mut c = other.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => other.to_string(),
            }
        }
        _ => "Claude".into(),
    }
}

fn tier_multiplier(tier: &str) -> Option<String> {
    tier.split('_').find_map(|tok| {
        let t = tok.to_ascii_lowercase();
        if t.len() >= 2 && t.ends_with('x') {
            let num = &t[..t.len() - 1];
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                return Some(t);
            }
        }
        None
    })
}

fn format_plan(subscription: &str, tier: &str) -> String {
    let base = pretty_plan(subscription);
    match tier_multiplier(tier) {
        Some(m) => format!("{base} {m}"),
        None => base,
    }
}

fn read_token() -> Option<(String, String)> {
    let path = claude_dir().join(".credentials.json");
    let bytes = std::fs::read(&path).ok()?;
    let parsed: CredentialsFile = serde_json::from_slice(&bytes).ok()?;
    let oauth = parsed.claude_ai_oauth?;
    let token = oauth.access_token.filter(|s| !s.trim().is_empty())?;
    let plan = format_plan(
        oauth.subscription_type.as_deref().unwrap_or(""),
        oauth.rate_limit_tier.as_deref().unwrap_or(""),
    );
    Some((token, plan))
}

fn detect_cli_version() -> String {
    let projects = claude_dir().join("projects");
    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in walkdir::WalkDir::new(&projects)
        .max_depth(4)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .map(|e| e == "jsonl")
                .unwrap_or(false)
        {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                        newest = Some((modified, entry.path().to_path_buf()));
                    }
                }
            }
        }
    }
    if let Some((_, path)) = newest {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines().rev().take(50) {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                        if !ver.is_empty() {
                            return ver.to_string();
                        }
                    }
                }
            }
        }
    }
    FALLBACK_VERSION.to_string()
}

fn fetch_usage(token: &str) -> Result<Value, FetchError> {
    let ua = format!(
        "claude-code/{}",
        CLI_VERSION.get_or_init(detect_cli_version)
    );
    http::get_json(
        USAGE_URL,
        &[
            ("Authorization", &format!("Bearer {token}")),
            ("anthropic-beta", ANTHROPIC_BETA),
            ("User-Agent", &ua),
            ("Content-Type", "application/json"),
        ],
    )
}

pub fn snapshot_from_json(plan: &str, body: &Value) -> ProviderSnapshot {
    let mut lines = Vec::new();
    if let Some((w, util)) = body
        .get("five_hour")
        .and_then(|value| json_f64(value, &["utilization"]).map(|util| (value, util)))
    {
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct(
            "session", "Sesión", util, reset, 18_000, "always",
        ));
    }
    if let Some((w, util)) = body
        .get("seven_day")
        .and_then(|value| json_f64(value, &["utilization"]).map(|util| (value, util)))
    {
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct(
            "weekly", "Semanal", util, reset, 604_800, "always",
        ));
    }
    if let Some((w, util)) = body
        .get("seven_day_sonnet")
        .and_then(|value| json_f64(value, &["utilization"]).map(|util| (value, util)))
    {
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct(
            "sonnet", "Sonnet", util, reset, 604_800, "demand",
        ));
    }
    if let Some((w, util)) = body
        .get("seven_day_opus")
        .and_then(|value| json_f64(value, &["utilization"]).map(|util| (value, util)))
    {
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct("opus", "Opus", util, reset, 604_800, "demand"));
    }
    if let Some(arr) = body.get("limits").and_then(|v| v.as_array()) {
        for item in arr {
            if item.get("kind").and_then(|k| k.as_str()) != Some("weekly_scoped") {
                continue;
            }
            let label = item
                .pointer("/scope/model/display_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Modelo");
            let Some(util) = json_f64(item, &["utilization"]) else {
                continue;
            };
            let reset = json_str(item, &["resets_at"]);
            lines.push(progress_pct(
                &format!("scoped_{}", label.to_ascii_lowercase()),
                label,
                util,
                reset,
                604_800,
                "demand",
            ));
        }
    }
    if let Some(ex) = body.get("extra_usage") {
        if let Some((used, limit)) = json_f64(
            ex,
            &[
                "used_credits",
                "used_usd",
                "used",
                "spent",
                "amount",
                "current",
            ],
        )
        .zip(json_f64(
            ex,
            &[
                "monthly_limit",
                "limit_usd",
                "limit",
                "cap",
                "max",
                "budget",
            ],
        ))
        .filter(|(_, limit)| *limit > 0.0)
        {
            let pct = json_f64(ex, &["utilization"]).unwrap_or((used / limit) * 100.0);
            lines.push(progress_pct(
                "extra",
                "Uso extra",
                pct,
                None,
                2_592_000,
                "demand",
            ));
            lines.push(values_line(
                "extra_usd",
                "Extra $",
                &format!("${used:.2} / ${limit:.2}"),
                "demand",
            ));
        }
    }
    let mut snapshot = snapshot_ok(VendorId::Anthropic, plan, lines);
    snapshot.product_breakdown = product_breakdown(body);
    apply_temporary_limit(&mut snapshot, body);
    snapshot
}

fn product_name(raw: &str) -> String {
    match raw.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
        "claude_code" | "code" => "Claude Code".into(),
        "chat" | "chats" | "claude_ai" => "Chats".into(),
        "cowork" => "Cowork".into(),
        "other" | "otros" => "Otro".into(),
        _ => raw.replace('_', " "),
    }
}

fn product_percent(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| {
        json_f64(
            value,
            &[
                "used_percent",
                "usedPercent",
                "utilization",
                "percent",
                "usage_percent",
            ],
        )
    })
}

fn parse_product_value(value: &Value) -> Vec<ProductUsage> {
    let mut out = Vec::new();
    if let Some(items) = value.as_array() {
        for item in items {
            let Some(name) = json_str(
                item,
                &[
                    "name",
                    "display_name",
                    "key",
                    "product",
                    "product_name",
                    "label",
                ],
            ) else {
                continue;
            };
            if let Some(percent) = product_percent(item) {
                out.push(ProductUsage::new(
                    product_name(&name),
                    percent.clamp(0.0, 100.0),
                ));
            }
        }
    } else if let Some(items) = value.as_object() {
        for (name, item) in items {
            if let Some(percent) = product_percent(item) {
                out.push(ProductUsage::new(
                    product_name(name),
                    percent.clamp(0.0, 100.0),
                ));
            }
        }
    }
    out
}

fn product_breakdown(body: &Value) -> Vec<ProductUsage> {
    const PATHS: &[&str] = &[
        "/product_breakdown",
        "/usage_by_product",
        "/seven_day/product_breakdown",
        "/seven_day/usage_by_product",
        "/seven_day/by_product",
        "/weekly/product_breakdown",
        "/seven_day_breakdown/rows",
    ];
    for path in PATHS {
        if let Some(value) = body.pointer(path) {
            let parsed = parse_product_value(value);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

fn multiplier(value: &Value) -> Option<f64> {
    let raw = value.as_f64().or_else(|| {
        json_f64(
            value,
            &["multiplier", "temporary_multiplier", "limit_multiplier"],
        )
    })?;
    let normalized = if raw >= 10.0 { 1.0 + raw / 100.0 } else { raw };
    (normalized > 1.0).then_some(normalized)
}

fn apply_temporary_limit(snapshot: &mut ProviderSnapshot, body: &Value) {
    const MULTIPLIER_PATHS: &[&str] = &[
        "/seven_day/temporary_multiplier",
        "/seven_day/limit_multiplier",
        "/seven_day/temporary_limit",
        "/temporary_limit",
        "/promotion",
    ];
    let found = MULTIPLIER_PATHS
        .iter()
        .find_map(|path| body.pointer(path).and_then(multiplier));
    let expires_at = [
        "/seven_day/temporary_limit/expires_at",
        "/seven_day/temporary_limit_ends_at",
        "/temporary_limit/expires_at",
        "/temporary_limit_ends_at",
        "/promotion/expires_at",
    ]
    .iter()
    .find_map(|path| {
        body.pointer(path)
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    if found.is_none() && expires_at.is_none() {
        return;
    }
    if let Some(quota) = snapshot.quotas.iter_mut().find(|q| q.id == "weekly") {
        quota.temporary_multiplier = found;
        quota.temporary_expires_at = expires_at;
    }
}

fn append_cost(snap: &mut ProviderSnapshot) {
    let report = cost::compute();
    if report.empty {
        return;
    }
    snap.lines.push(values_line(
        "cost_today",
        "Hoy (API eq.)",
        &format!("${:.2}", report.today_usd),
        "always",
    ));
    snap.lines.push(values_line(
        "cost_week",
        "Semana",
        &format!("${:.2}", report.week_usd),
        "demand",
    ));
    snap.lines.push(values_line(
        "cost_30",
        "30 días",
        &format!("${:.2}", report.last30_usd),
        "demand",
    ));
    snap.lines.push(values_line(
        "cost_month",
        "Mes",
        &format!("${:.2}", report.month_usd),
        "always",
    ));
    // Estimación desde logs locales: nunca presentarla como factura.
    snap.cost = Some(UsageCost {
        today: Some(report.today_usd),
        week: Some(report.week_usd),
        thirty_days: Some(report.last30_usd),
        month: Some(report.month_usd),
        confidence: crate::model::DataConfidence::Estimated,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resets_product_breakdown_and_temporary_limit() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 3.0, "resets_at": "2099-09-11T17:40:00Z"},
            "seven_day": {
                "utilization": 22.0,
                "resets_at": "2099-09-16T09:00:00Z",
                "temporary_limit": {"multiplier": 1.5, "expires_at": "2099-09-13T00:00:00Z"},
                "product_breakdown": {
                    "claude_code": 88.0,
                    "chats": 0.0,
                    "cowork": 12.0,
                    "other": 0.0
                }
            }
        });
        let snapshot = snapshot_from_json("Pro", &body);
        assert_eq!(
            snapshot.quotas[0].reset_status,
            crate::model::ResetStatus::Known
        );
        assert_eq!(snapshot.quotas[1].temporary_multiplier, Some(1.5));
        assert_eq!(snapshot.product_breakdown.len(), 4);
        let code = snapshot
            .product_breakdown
            .iter()
            .find(|item| item.name == "Claude Code")
            .expect("Claude Code breakdown");
        assert_eq!(code.used_percent, 88.0);
    }

    #[test]
    fn parses_current_breakdown_rows_and_ignores_null_model_windows() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 3.0, "resets_at": "2099-09-11T17:40:00Z"},
            "seven_day": {"utilization": 22.0, "resets_at": "2099-09-16T09:00:00Z"},
            "seven_day_sonnet": null,
            "seven_day_opus": null,
            "seven_day_breakdown": {"rows": [
                {"display_name": "Claude Code", "key": "claude_code", "percent": 88},
                {"display_name": "Cowork", "key": "cowork", "percent": 12}
            ]}
        });
        let snapshot = snapshot_from_json("Pro", &body);
        assert_eq!(snapshot.product_breakdown.len(), 2);
        assert!(!snapshot
            .quotas
            .iter()
            .any(|quota| quota.id == "sonnet" || quota.id == "opus"));
    }

    #[test]
    fn missing_utilization_does_not_become_zero_percent() {
        let snapshot = snapshot_from_json(
            "Pro",
            &serde_json::json!({"five_hour": {"resets_at": "2099-01-01T00:00:00Z"}}),
        );
        assert!(snapshot.quotas.is_empty());
    }
}
