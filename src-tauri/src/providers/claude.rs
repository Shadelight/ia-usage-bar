//! Claude Code — OAuth local + endpoint de uso de Anthropic.

use serde_json::Value;

use crate::config::AppConfig;
use crate::cost;
use crate::http::{self, FetchError};
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_err, snapshot_ok, values_line, ProviderSnapshot,
    VendorId,
};
use crate::paths::claude_dir;

use super::Provider;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const ANTHROPIC_BETA: &str = "oauth-2025-04-20";
const FALLBACK_VERSION: &str = "2.1.139";

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
            return snapshot_err(VendorId::Anthropic, "Claude Code no conectado");
        };
        match fetch_usage(&token) {
            Ok(body) => {
                let mut snap = snapshot_from_json(&plan, &body);
                append_cost(&mut snap);
                snap
            }
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
            && entry.path().extension().map(|e| e == "jsonl").unwrap_or(false)
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
    let ua = format!("claude-code/{}", detect_cli_version());
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
    if let Some(w) = body.get("five_hour") {
        let util = json_f64(w, &["utilization"]).unwrap_or(0.0);
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct("session", "Sesión", util, reset, 18_000, "always"));
    }
    if let Some(w) = body.get("seven_day") {
        let util = json_f64(w, &["utilization"]).unwrap_or(0.0);
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct("weekly", "Semanal", util, reset, 604_800, "always"));
    }
    if let Some(w) = body.get("seven_day_sonnet") {
        let util = json_f64(w, &["utilization"]).unwrap_or(0.0);
        let reset = json_str(w, &["resets_at"]);
        lines.push(progress_pct("sonnet", "Sonnet", util, reset, 604_800, "demand"));
    }
    if let Some(w) = body.get("seven_day_opus") {
        let util = json_f64(w, &["utilization"]).unwrap_or(0.0);
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
            let util = json_f64(item, &["utilization"]).unwrap_or(0.0);
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
        let used = json_f64(ex, &["used_credits", "used_usd", "used"]).unwrap_or(0.0);
        let limit = json_f64(ex, &["monthly_limit", "limit_usd", "limit"]).unwrap_or(0.0);
        if limit > 0.0 {
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
    snapshot_ok(VendorId::Anthropic, plan, lines)
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
}
