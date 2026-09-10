//! Tipos que se serializan al frontend (camelCase).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VendorId {
    Anthropic,
    AnthropicApi,
    Openai,
    OpenaiAdmin,
    Copilot,
    Zai,
    Openrouter,
    Deepseek,
    Kimi,
    Kilo,
    Novita,
    Moonshot,
    Grok,
    Supergrok,
    Antigravity,
    Cursor,
    Minimax,
    Kiro,
    Nous,
    #[serde(rename = "opencode_go")]
    OpenCodeGo,
    CommandCode,
    Groq,
    Windsurf,
}

impl VendorId {
    pub fn slug(self) -> &'static str {
        match self {
            VendorId::Anthropic => "anthropic",
            VendorId::AnthropicApi => "anthropic_api",
            VendorId::Openai => "openai",
            VendorId::OpenaiAdmin => "openai_admin",
            VendorId::Copilot => "copilot",
            VendorId::Zai => "zai",
            VendorId::Openrouter => "openrouter",
            VendorId::Deepseek => "deepseek",
            VendorId::Kimi => "kimi",
            VendorId::Kilo => "kilo",
            VendorId::Novita => "novita",
            VendorId::Moonshot => "moonshot",
            VendorId::Grok => "grok",
            VendorId::Supergrok => "supergrok",
            VendorId::Antigravity => "antigravity",
            VendorId::Cursor => "cursor",
            VendorId::Minimax => "minimax",
            VendorId::Kiro => "kiro",
            VendorId::Nous => "nous",
            VendorId::OpenCodeGo => "opencode_go",
            VendorId::CommandCode => "commandcode",
            VendorId::Groq => "groq",
            VendorId::Windsurf => "windsurf",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            VendorId::Anthropic => "Claude Code",
            VendorId::AnthropicApi => "Anthropic API",
            VendorId::Openai => "Codex / ChatGPT",
            VendorId::OpenaiAdmin => "OpenAI API",
            VendorId::Copilot => "GitHub Copilot",
            VendorId::Zai => "Z.AI / GLM",
            VendorId::Openrouter => "OpenRouter",
            VendorId::Deepseek => "DeepSeek",
            VendorId::Kimi => "Kimi",
            VendorId::Kilo => "Kilo",
            VendorId::Novita => "Novita",
            VendorId::Moonshot => "Moonshot",
            VendorId::Grok => "Grok (xAI)",
            VendorId::Supergrok => "SuperGrok",
            VendorId::Antigravity => "Antigravity",
            VendorId::Cursor => "Cursor",
            VendorId::Minimax => "MiniMax",
            VendorId::Kiro => "Kiro",
            VendorId::Nous => "Nous Research",
            VendorId::OpenCodeGo => "OpenCode Go",
            VendorId::CommandCode => "Command Code",
            VendorId::Groq => "Groq",
            VendorId::Windsurf => "Windsurf",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            VendorId::Anthropic => "CLD",
            VendorId::AnthropicApi => "ANT",
            VendorId::Openai => "CDX",
            VendorId::OpenaiAdmin => "OAI",
            VendorId::Copilot => "COP",
            VendorId::Zai => "ZAI",
            VendorId::Openrouter => "OR",
            VendorId::Deepseek => "DSK",
            VendorId::Kimi => "KMI",
            VendorId::Kilo => "KLO",
            VendorId::Novita => "NOV",
            VendorId::Moonshot => "MSH",
            VendorId::Grok => "GRK",
            VendorId::Supergrok => "SGK",
            VendorId::Antigravity => "AGY",
            VendorId::Cursor => "CUR",
            VendorId::Minimax => "MMX",
            VendorId::Kiro => "KRO",
            VendorId::Nous => "NUS",
            VendorId::OpenCodeGo => "OCG",
            VendorId::CommandCode => "CMD",
            VendorId::Groq => "GRQ",
            VendorId::Windsurf => "WND",
        }
    }

    pub fn all() -> &'static [VendorId] {
        &[
            VendorId::Anthropic,
            VendorId::Openai,
            VendorId::Cursor,
            VendorId::Antigravity,
            VendorId::OpenaiAdmin,
            VendorId::Copilot,
            VendorId::Openrouter,
            VendorId::Zai,
            VendorId::Deepseek,
            VendorId::Grok,
            VendorId::Supergrok,
            VendorId::Kimi,
            VendorId::Kilo,
            VendorId::Novita,
            VendorId::Moonshot,
            VendorId::Minimax,
            VendorId::Kiro,
            VendorId::Nous,
            VendorId::OpenCodeGo,
            VendorId::CommandCode,
            VendorId::AnthropicApi,
            VendorId::Groq,
            VendorId::Windsurf,
        ]
    }

    pub fn env_key(self) -> Option<&'static str> {
        match self {
            VendorId::Zai => Some("ZAI_API_KEY"),
            VendorId::Openrouter => Some("OPENROUTER_API_KEY"),
            VendorId::Deepseek => Some("DEEPSEEK_API_KEY"),
            VendorId::Kimi => Some("KIMI_API_KEY"),
            VendorId::Kilo => Some("KILO_API_KEY"),
            VendorId::Novita => Some("NOVITA_API_KEY"),
            VendorId::Moonshot => Some("MOONSHOT_API_KEY"),
            VendorId::Grok => Some("XAI_MANAGEMENT_KEY"),
            VendorId::Minimax => Some("MINIMAX_API_KEY"),
            VendorId::AnthropicApi => Some("ANTHROPIC_ADMIN_KEY"),
            VendorId::OpenaiAdmin => Some("OPENAI_ADMIN_KEY"),
            VendorId::OpenCodeGo => Some("OPENCODE_GO_API_KEY"),
            VendorId::CommandCode => Some("COMMANDCODE_API_KEY"),
            VendorId::Copilot => Some("GITHUB_COPILOT_TOKEN"),
            VendorId::Groq => Some("GROQ_API_KEY"),
            _ => None,
        }
    }

    pub fn auth_kind(self) -> AuthKind {
        match self {
            VendorId::Anthropic | VendorId::Openai | VendorId::Nous => AuthKind::Oauth,
            VendorId::Cursor
            | VendorId::Antigravity
            | VendorId::Kiro
            | VendorId::Supergrok
            | VendorId::CommandCode
            | VendorId::Windsurf => AuthKind::Local,
            VendorId::Copilot | VendorId::Kimi => AuthKind::Mixed,
            _ => AuthKind::ApiKey,
        }
    }

    pub fn login_hint(self) -> &'static str {
        match self {
            VendorId::Anthropic => "Ejecuta `claude` e inicia sesión.",
            VendorId::Openai => "Ejecuta `codex login`.",
            VendorId::Cursor => "Abre Cursor e inicia sesión.",
            VendorId::Antigravity => "Abre Antigravity e inicia sesión.",
            VendorId::Copilot => "Ejecuta `gh auth login --web` o define GITHUB_COPILOT_TOKEN.",
            VendorId::Kiro => "Ejecuta `kiro-cli login`.",
            VendorId::Supergrok => "Ejecuta `grok login`.",
            VendorId::CommandCode => "Inicia sesión con `commandcode` o pi.",
            VendorId::Nous => "Guarda las credenciales OAuth de Nous.",
            VendorId::Kimi => "Define KIMI_API_KEY o inicia sesión con `kimi`.",
            VendorId::Groq => "Pega GROQ_API_KEY en Ajustes.",
            VendorId::Windsurf => "Abre Windsurf e inicia sesión.",
            _ => "Pega una API key en Ajustes o define la variable de entorno.",
        }
    }

    pub fn needs_api_key_ui(self) -> bool {
        matches!(self.auth_kind(), AuthKind::ApiKey | AuthKind::Mixed) && self.env_key().is_some()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    Oauth,
    ApiKey,
    Local,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MetricLine {
    Progress {
        id: String,
        label: String,
        used: f64,
        limit: f64,
        format: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        resets_at: Option<String>,
        resets_in_label: String,
        window_secs: i64,
        visible: String,
    },
    Values {
        id: String,
        label: String,
        text: String,
        visible: String,
    },
    Badge {
        id: String,
        label: String,
        text: String,
        visible: String,
    },
}

impl MetricLine {
    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        match self {
            MetricLine::Progress { id, .. }
            | MetricLine::Values { id, .. }
            | MetricLine::Badge { id, .. } => id,
        }
    }

    pub fn utilization(&self) -> Option<f64> {
        match self {
            MetricLine::Progress { used, limit, .. } if *limit > 0.0 => {
                Some((*used / *limit) * 100.0)
            }
            MetricLine::Progress { used, format, .. } if format == "percent" => Some(*used),
            _ => None,
        }
    }

    pub fn resets_at(&self) -> Option<&str> {
        match self {
            MetricLine::Progress { resets_at, .. } => resets_at.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub id: String,
    pub name: String,
    pub short: String,
    pub plan: String,
    pub connected: bool,
    pub stale: bool,
    pub error: Option<String>,
    pub hint: Option<String>,
    pub updated_at: String,
    pub lines: Vec<MetricLine>,
    pub primary_utilization: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorInfo {
    pub id: String,
    pub name: String,
    pub short: String,
    pub auth_kind: AuthKind,
    pub env_key: Option<String>,
    pub hint: String,
    pub needs_key: bool,
    pub enabled: bool,
    pub detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub providers: Vec<ProviderSnapshot>,
    pub catalog: Vec<VendorInfo>,
    pub refresh_minutes: u64,
    pub primary: String,
    pub notifications: bool,
    pub show_usage_as: String,
    pub reset_times: String,
    pub next_update_in_secs: u64,
    pub spend_month_usd: f64,
    pub spend: Vec<SpendRow>,
    pub recommend_id: Option<String>,
    pub recommend_name: Option<String>,
    pub recommend_left: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendRow {
    pub id: String,
    pub name: String,
    pub label: String,
    pub usd: f64,
}

pub fn parse_usd(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_prefix('$').unwrap_or(t).replace(',', "");
    let n: f64 = t.parse().ok()?;
    n.is_finite().then_some(n)
}

pub fn monthly_spend(p: &ProviderSnapshot) -> Option<(String, f64)> {
    let mut best: Option<(u8, String, f64)> = None;
    for line in &p.lines {
        let MetricLine::Values { id, label, text, .. } = line else {
            continue;
        };
        let rank = match id.as_str() {
            "cost_month" => 0,
            "cost_30" | "mtd" => 1,
            "cost_week" => 2,
            _ => continue,
        };
        let Some(usd) = parse_usd(text) else { continue };
        if best.as_ref().map(|(r, _, _)| rank < *r).unwrap_or(true) {
            best = Some((rank, label.clone(), usd));
        }
    }
    best.map(|(_, label, usd)| (label, usd))
}

pub fn most_headroom(providers: &[ProviderSnapshot]) -> Option<(String, String, f64)> {
    providers
        .iter()
        .filter(|p| p.connected)
        .filter_map(|p| {
            p.primary_utilization
                .map(|u| (p.id.clone(), p.name.clone(), (100.0 - u).max(0.0)))
        })
        .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
}

pub fn now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

pub fn resets_in_label(iso: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(iso);
    let target = match parsed {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => return String::new(),
    };
    let secs = (target - chrono::Utc::now()).num_seconds();
    if secs <= 0 {
        return "ahora".to_string();
    }
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

pub fn resets_from_unix(ts: i64) -> (Option<String>, String, i64) {
    let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) else {
        return (None, String::new(), 0);
    };
    let iso = dt.to_rfc3339();
    let label = resets_in_label(&iso);
    (Some(iso), label, (dt - chrono::Utc::now()).num_seconds().max(0))
}

pub fn progress_pct(
    id: &str,
    label: &str,
    pct: f64,
    resets_at: Option<String>,
    window_secs: i64,
    visible: &str,
) -> MetricLine {
    let resets_in_label = resets_at
        .as_deref()
        .map(resets_in_label)
        .unwrap_or_default();
    MetricLine::Progress {
        id: id.to_string(),
        label: label.to_string(),
        used: pct.clamp(0.0, 100.0),
        limit: 100.0,
        format: "percent".into(),
        resets_at,
        resets_in_label,
        window_secs,
        visible: visible.to_string(),
    }
}

pub fn values_line(id: &str, label: &str, text: &str, visible: &str) -> MetricLine {
    MetricLine::Values {
        id: id.to_string(),
        label: label.to_string(),
        text: text.to_string(),
        visible: visible.to_string(),
    }
}

pub fn badge_line(id: &str, label: &str, text: &str) -> MetricLine {
    MetricLine::Badge {
        id: id.to_string(),
        label: label.to_string(),
        text: text.to_string(),
        visible: "demand".into(),
    }
}

pub fn snapshot_ok(id: VendorId, plan: &str, lines: Vec<MetricLine>) -> ProviderSnapshot {
    let primary_utilization = lines.iter().find_map(|l| l.utilization());
    ProviderSnapshot {
        id: id.slug().to_string(),
        name: id.display_name().to_string(),
        short: id.short().to_string(),
        plan: if plan.is_empty() {
            id.display_name().to_string()
        } else {
            plan.to_string()
        },
        connected: true,
        stale: false,
        error: None,
        hint: None,
        updated_at: now_iso(),
        lines,
        primary_utilization,
    }
}

pub fn snapshot_err(id: VendorId, error: &str) -> ProviderSnapshot {
    ProviderSnapshot {
        id: id.slug().to_string(),
        name: id.display_name().to_string(),
        short: id.short().to_string(),
        plan: String::new(),
        connected: false,
        stale: false,
        error: Some(error.to_string()),
        hint: Some(id.login_hint().to_string()),
        updated_at: now_iso(),
        lines: vec![],
        primary_utilization: None,
    }
}

pub fn json_f64(v: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for k in keys {
        if let Some(n) = v.get(*k).and_then(|x| x.as_f64()) {
            return Some(n);
        }
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if let Ok(n) = s.parse::<f64>() {
                return Some(n);
            }
        }
    }
    None
}

pub fn json_str(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_percent_maps_utilization() {
        let line = progress_pct("session", "Sesión", 42.4, None, 18_000, "always");
        assert_eq!(line.utilization().unwrap().round(), 42.0);
    }

    #[test]
    fn vendor_slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for id in VendorId::all() {
            assert!(seen.insert(id.slug()), "duplicate {}", id.slug());
        }
    }

    #[test]
    fn parse_usd_strips_dollar() {
        assert_eq!(parse_usd("$12.50"), Some(12.5));
        assert_eq!(parse_usd("1,200.00"), Some(1200.0));
    }

    #[test]
    fn monthly_spend_prefers_cost_month() {
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![
                values_line("cost_today", "Hoy", "$1.00", "always"),
                values_line("cost_month", "Mes", "$40.00", "always"),
            ],
        );
        assert_eq!(monthly_spend(&snap), Some(("Mes".into(), 40.0)));
    }

    #[test]
    fn monthly_spend_ignores_lifetime_used() {
        let snap = snapshot_ok(
            VendorId::Openrouter,
            "OpenRouter",
            vec![values_line("used", "Usado", "$88.00", "always")],
        );
        assert_eq!(monthly_spend(&snap), None);
    }

    #[test]
    fn most_headroom_picks_lowest_utilization() {
        let a = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct("s", "Sesión", 88.0, None, 18_000, "always")],
        );
        let b = snapshot_ok(
            VendorId::Cursor,
            "Ultra",
            vec![progress_pct("s", "Uso", 26.0, None, 18_000, "always")],
        );
        let pick = most_headroom(&[a, b]).unwrap();
        assert_eq!(pick.0, "cursor");
        assert!((pick.2 - 74.0).abs() < 0.01);
    }
}
