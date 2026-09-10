//! Proveedores autenticados con API key.

use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_ok, values_line, ProviderSnapshot, VendorId,
};

use super::Provider;

macro_rules! key_provider {
    ($name:ident, $id:expr) => {
        pub struct $name;
        impl Provider for $name {
            fn id(&self) -> VendorId {
                $id
            }
            fn has_local_credentials(&self, cfg: &AppConfig) -> bool {
                cfg.api_key($id).is_some()
            }
            fn refresh(&self, cfg: &AppConfig) -> ProviderSnapshot {
                match super::key_or_err($id, cfg) {
                    Ok(key) => match self.fetch(&key, cfg) {
                        Ok(snap) => snap,
                        Err(e) => super::map_fetch_err($id, e),
                    },
                    Err(s) => s,
                }
            }
        }
        impl $name {
            fn fetch(&self, key: &str, cfg: &AppConfig) -> Result<ProviderSnapshot, FetchError> {
                fetch_named($id, key, cfg)
            }
        }
    };
}

key_provider!(OpenRouter, VendorId::Openrouter);
key_provider!(Zai, VendorId::Zai);
key_provider!(DeepSeek, VendorId::Deepseek);
key_provider!(Grok, VendorId::Grok);
key_provider!(Kilo, VendorId::Kilo);
key_provider!(Novita, VendorId::Novita);
key_provider!(Moonshot, VendorId::Moonshot);
key_provider!(Minimax, VendorId::Minimax);
key_provider!(AnthropicApi, VendorId::AnthropicApi);
key_provider!(OpenCodeGo, VendorId::OpenCodeGo);
key_provider!(Groq, VendorId::Groq);

fn fetch_named(id: VendorId, key: &str, cfg: &AppConfig) -> Result<ProviderSnapshot, FetchError> {
    match id {
        VendorId::Openrouter => fetch_openrouter(key),
        VendorId::Zai => fetch_zai(key),
        VendorId::Deepseek => fetch_deepseek(key),
        VendorId::Grok => fetch_grok(key, cfg),
        VendorId::Kilo => fetch_kilo(key),
        VendorId::Novita => fetch_novita(key),
        VendorId::Moonshot => fetch_moonshot(key, cfg),
        VendorId::Minimax => fetch_minimax(key, cfg),
        VendorId::AnthropicApi => fetch_anthropic_api(key),
        VendorId::OpenCodeGo => fetch_opencode_go(key),
        VendorId::Groq => fetch_groq(key),
        _ => Err(FetchError::Parse("vendor interno".into())),
    }
}

fn money(id: VendorId, plan: &str, label: &str, amount: f64) -> ProviderSnapshot {
    snapshot_ok(
        id,
        plan,
        vec![values_line("balance", label, &format!("${amount:.2}"), "always")],
    )
}

fn fetch_openrouter(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let credits = http::get_json(
        "https://openrouter.ai/api/v1/credits",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let info = http::get_json(
        "https://openrouter.ai/api/v1/key",
        &[("Authorization", &format!("Bearer {key}"))],
    )
    .ok();
    let data = credits.get("data").unwrap_or(&credits);
    let total = json_f64(data, &["total_credits", "total"]).unwrap_or(0.0);
    let used = json_f64(data, &["total_usage", "usage"]).unwrap_or(0.0);
    let remaining = total - used;
    let mut lines = vec![values_line(
        "balance",
        "Saldo",
        &format!("${remaining:.2}"),
        "always",
    )];
    if used > 0.0 {
        lines.push(values_line("used", "Usado", &format!("${used:.2}"), "demand"));
    }
    if let Some(limit) = info
        .as_ref()
        .and_then(|v| v.pointer("/data/limit"))
        .and_then(|v| v.as_f64())
    {
        if limit > 0.0 {
            lines.push(values_line("limit", "Límite", &format!("${limit:.2}"), "demand"));
        }
    }
    Ok(snapshot_ok(VendorId::Openrouter, "OpenRouter", lines))
}

fn fetch_zai(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.z.ai/api/monitor/usage/quota/limit",
        &[("Authorization", key)],
    )?;
    let data = body.get("data").unwrap_or(&body);
    let mut lines = Vec::new();
    push_zai_window(&mut lines, data, "fiveHour", "session", "Sesión", 18_000, "always");
    push_zai_window(&mut lines, data, "weekly", "weekly", "Semanal", 604_800, "always");
    if let Some(mcp) = data.get("mcp") {
        let pct = json_f64(mcp, &["utilization", "usedPercent", "percent"]).unwrap_or(0.0);
        lines.push(progress_pct("mcp", "MCP", pct, None, 2_592_000, "demand"));
    }
    let plan = json_str(data, &["plan", "planName"]).unwrap_or_else(|| "GLM".into());
    Ok(snapshot_ok(VendorId::Zai, &plan, lines))
}

fn push_zai_window(
    lines: &mut Vec<crate::model::MetricLine>,
    data: &Value,
    key: &str,
    id: &str,
    label: &str,
    window: i64,
    visible: &str,
) {
    let Some(w) = data.get(key).or_else(|| data.get(id)) else { return };
    let pct = json_f64(w, &["utilization", "usedPercent", "percent"]).unwrap_or(0.0);
    let reset = json_str(w, &["resets_at", "resetAt", "reset_at"]);
    lines.push(progress_pct(id, label, pct, reset, window, visible));
}

fn fetch_deepseek(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.deepseek.com/user/balance",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let infos = body
        .get("balance_infos")
        .or_else(|| body.get("data"))
        .cloned()
        .unwrap_or(body.clone());
    let mut total = 0.0;
    if let Some(arr) = infos.as_array() {
        for row in arr {
            total += json_f64(row, &["total_balance", "balance"]).unwrap_or(0.0);
        }
    } else {
        total = json_f64(&infos, &["total_balance", "balance"]).unwrap_or(0.0);
    }
    Ok(money(VendorId::Deepseek, "DeepSeek", "Saldo", total))
}

fn fetch_grok(key: &str, cfg: &AppConfig) -> Result<ProviderSnapshot, FetchError> {
    let team = cfg
        .provider(VendorId::Grok)
        .team_id
        .or_else(|| resolve_grok_team(key).ok());
    let Some(team) = team else {
        return Err(FetchError::Parse(
            "Grok: hace falta team_id para una key de organización".into(),
        ));
    };
    let url = format!("https://management-api.x.ai/v1/billing/teams/{team}/prepaid/balance");
    let body = http::get_json(&url, &[("Authorization", &format!("Bearer {key}"))])?;
    let bal = json_f64(&body, &["balance", "amount", "prepaid_balance"]).unwrap_or(0.0);
    Ok(money(VendorId::Grok, "xAI", "Saldo prepago", bal / if bal > 1000.0 { 100.0 } else { 1.0 }))
}

fn resolve_grok_team(key: &str) -> Result<String, FetchError> {
    let body = http::get_json(
        "https://management-api.x.ai/auth/management-keys/validation",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    json_str(&body, &["teamId", "team_id", "scopeId"])
        .ok_or_else(|| FetchError::Parse("Grok: no se pudo resolver el team".into()))
}

fn fetch_kilo(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.kilo.ai/api/profile/balance",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let data = body.get("data").unwrap_or(&body);
    let bal = json_f64(data, &["balance", "credits", "remaining"]).unwrap_or(0.0);
    Ok(money(VendorId::Kilo, "Kilo", "Saldo", bal))
}

fn fetch_novita(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.novita.ai/openapi/v1/billing/balance/detail",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let data = body.get("data").unwrap_or(&body);
    let bal = json_f64(data, &["balance", "credit_balance", "remaining"]).unwrap_or(0.0);
    Ok(money(VendorId::Novita, "Novita", "Saldo", bal))
}

fn fetch_moonshot(key: &str, cfg: &AppConfig) -> Result<ProviderSnapshot, FetchError> {
    let cn = cfg
        .provider(VendorId::Moonshot)
        .region
        .as_deref()
        .map(|r| r.eq_ignore_ascii_case("cn"))
        .unwrap_or(false);
    let host = if cn {
        "https://api.moonshot.cn"
    } else {
        "https://api.moonshot.ai"
    };
    let body = http::get_json(
        &format!("{host}/v1/users/me/balance"),
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let data = body.get("data").unwrap_or(&body);
    let bal = json_f64(data, &["available_balance", "balance", "cash_balance"]).unwrap_or(0.0);
    let label = if cn { "Saldo (¥)" } else { "Saldo" };
    let text = if cn {
        format!("¥{bal:.2}")
    } else {
        format!("${bal:.2}")
    };
    Ok(snapshot_ok(
        VendorId::Moonshot,
        "Moonshot",
        vec![values_line("balance", label, &text, "always")],
    ))
}

fn fetch_minimax(key: &str, cfg: &AppConfig) -> Result<ProviderSnapshot, FetchError> {
    let cn = cfg
        .provider(VendorId::Minimax)
        .region
        .as_deref()
        .map(|r| r.eq_ignore_ascii_case("cn"))
        .unwrap_or(false);
    let host = if cn {
        "https://api.minimaxi.com"
    } else {
        "https://api.minimax.io"
    };
    let body = http::get_json(
        &format!("{host}/v1/token_plan/remains"),
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let data = body.get("data").unwrap_or(&body);
    let mut lines = Vec::new();
    if let Some(interval) = data.get("interval").or_else(|| data.get("rolling")) {
        let pct = used_pct(interval);
        let reset = json_str(interval, &["resets_at", "resetAt"]);
        lines.push(progress_pct("interval", "Intervalo", pct, reset, 18_000, "always"));
    }
    if let Some(weekly) = data.get("weekly") {
        let pct = used_pct(weekly);
        let reset = json_str(weekly, &["resets_at", "resetAt"]);
        lines.push(progress_pct("weekly", "Semanal", pct, reset, 604_800, "always"));
    }
    if lines.is_empty() {
        let pct = json_f64(data, &["usedPercent", "utilization"]).unwrap_or(0.0);
        lines.push(progress_pct("plan", "Token Plan", pct, None, 604_800, "always"));
    }
    Ok(snapshot_ok(VendorId::Minimax, "MiniMax Token Plan", lines))
}

fn used_pct(v: &Value) -> f64 {
    if let Some(p) = json_f64(v, &["usedPercent", "utilization", "percent"]) {
        return p;
    }
    let remain = json_f64(v, &["remainPercent", "remaining_percent"]);
    remain.map(|r| 100.0 - r).unwrap_or(0.0)
}

fn fetch_anthropic_api(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let now = chrono::Utc::now();
    let start = format!("{}-{:02}-01T00:00:00Z", now.format("%Y"), now.format("%m"));
    let url = format!(
        "https://api.anthropic.com/v1/organizations/cost_report?starting_at={start}&limit=31&group_by[]=description"
    );
    let body = http::get_json(
        &url,
        &[
            ("x-api-key", key),
            ("anthropic-version", "2023-06-01"),
        ],
    )?;
    let mut total = 0.0;
    if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
        for row in arr {
            if let Some(results) = row.get("results").and_then(|v| v.as_array()) {
                for r in results {
                    total += json_f64(r, &["amount"]).unwrap_or(0.0);
                }
            } else {
                total += json_f64(row, &["amount"]).unwrap_or(0.0);
            }
        }
    }
    let usd = total / 100.0;
    Ok(snapshot_ok(
        VendorId::AnthropicApi,
        "Admin",
        vec![values_line(
            "cost_month",
            "Gasto del mes",
            &format!("${usd:.2}"),
            "always",
        )],
    ))
}

fn fetch_groq(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.groq.com/openai/v1/models",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let n = body
        .get("data")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    Ok(snapshot_ok(
        VendorId::Groq,
        "Groq Cloud",
        vec![
            crate::model::badge_line("status", "Estado", "Conectado"),
            values_line("models", "Modelos", &n.to_string(), "always"),
        ],
    ))
}

fn fetch_opencode_go(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://opencode.ai/zen/go/v1/usage",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let mut lines = Vec::new();
    for (key_name, label, window, vis) in [
        ("rolling", "Rolling", 18_000i64, "always"),
        ("weekly", "Semanal", 604_800, "always"),
        ("monthly", "Mensual", 2_592_000, "demand"),
    ] {
        if let Some(w) = body.get(key_name) {
            let pct = json_f64(w, &["percent", "utilization"]).unwrap_or(0.0);
            let reset = json_str(w, &["reset", "resets_at", "resetAt"]);
            lines.push(progress_pct(key_name, label, pct, reset, window, vis));
        }
    }
    Ok(snapshot_ok(VendorId::OpenCodeGo, "OpenCode Go", lines))
}
