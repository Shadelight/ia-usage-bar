//! Vendors con login local (CLI / sqlite / keyring), no API key pura.

use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    badge_line, json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok,
    snapshot_with_status, values_line, ProviderSnapshot, ProviderStatus, ProviderStatusReason,
    VendorId,
};
use crate::paths::{app_config_dir, home_dir};

use super::Provider;

pub struct Kimi;
pub struct SuperGrok;
pub struct CommandCode;
pub struct Nous;
pub struct Windsurf;

impl Provider for Kimi {
    fn id(&self) -> VendorId {
        VendorId::Kimi
    }
    fn has_local_credentials(&self, cfg: &AppConfig) -> bool {
        cfg.api_key(VendorId::Kimi).is_some() || kimi_creds_path().exists()
    }
    fn refresh(&self, cfg: &AppConfig) -> ProviderSnapshot {
        let token = cfg.api_key(VendorId::Kimi).or_else(read_kimi_token);
        let Some(token) = token else {
            return snapshot_needs_auth(VendorId::Kimi, VendorId::Kimi.login_hint());
        };
        match fetch_kimi(&token) {
            Ok(s) => s,
            Err(e) => super::map_fetch_err(VendorId::Kimi, e),
        }
    }
}

impl Provider for SuperGrok {
    fn id(&self) -> VendorId {
        VendorId::Supergrok
    }
    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        grok_auth_path().exists()
    }
    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let Some(key) = read_grok_key() else {
            return snapshot_needs_auth(VendorId::Supergrok, VendorId::Supergrok.login_hint());
        };
        match fetch_supergrok(&key) {
            Ok(s) => s,
            Err(e) => super::map_fetch_err(VendorId::Supergrok, e),
        }
    }
}

impl Provider for CommandCode {
    fn id(&self) -> VendorId {
        VendorId::CommandCode
    }
    fn has_local_credentials(&self, cfg: &AppConfig) -> bool {
        cfg.api_key(VendorId::CommandCode).is_some()
            || commandcode_auth().exists()
            || pi_auth().exists()
    }
    fn refresh(&self, cfg: &AppConfig) -> ProviderSnapshot {
        let token = cfg
            .api_key(VendorId::CommandCode)
            .or_else(|| {
                read_json_token(
                    &commandcode_auth(),
                    &["token", "access_token", "accessToken"],
                )
            })
            .or_else(|| read_json_token(&pi_auth(), &["token", "access_token"]));
        let Some(token) = token else {
            return snapshot_needs_auth(VendorId::CommandCode, VendorId::CommandCode.login_hint());
        };
        match fetch_commandcode(&token) {
            Ok(s) => s,
            Err(e) => super::map_fetch_err(VendorId::CommandCode, e),
        }
    }
}

impl Provider for Nous {
    fn id(&self) -> VendorId {
        VendorId::Nous
    }
    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        nous_creds().exists()
    }
    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let Some(token) = read_json_token(&nous_creds(), &["access_token", "accessToken", "token"])
        else {
            return snapshot_needs_auth(VendorId::Nous, VendorId::Nous.login_hint());
        };
        match fetch_nous(&token) {
            Ok(s) => s,
            Err(e) => super::map_fetch_err(VendorId::Nous, e),
        }
    }
}

fn kimi_creds_path() -> std::path::PathBuf {
    home_dir()
        .join(".kimi-code")
        .join("credentials")
        .join("kimi-code.json")
}

fn read_kimi_token() -> Option<String> {
    read_json_token(
        &kimi_creds_path(),
        &["access_token", "accessToken", "token"],
    )
}

fn fetch_kimi(token: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.kimi.com/coding/v1/usages",
        &[("Authorization", &format!("Bearer {token}"))],
    )?;
    let mut lines = Vec::new();
    let weekly = body.get("weekly").or_else(|| body.get("subscription"));
    if let Some((w, pct)) = weekly.and_then(|value| {
        json_f64(value, &["utilization", "usedPercent", "percent"]).map(|pct| (value, pct))
    }) {
        let reset = json_str(w, &["resets_at", "resetAt"]);
        lines.push(progress_pct(
            "weekly", "Semanal", pct, reset, 604_800, "always",
        ));
    }
    if let Some((w, pct)) = body
        .get("five_hour")
        .or_else(|| body.get("rate_limit"))
        .and_then(|value| {
            json_f64(value, &["utilization", "usedPercent", "percent"]).map(|pct| (value, pct))
        })
    {
        let reset = json_str(w, &["resets_at", "resetAt"]);
        lines.push(progress_pct(
            "session",
            "Sesión 5h",
            pct,
            reset,
            18_000,
            "always",
        ));
    }
    let plan = json_str(&body, &["plan", "membership", "level"]).unwrap_or_else(|| "Kimi".into());
    Ok(snapshot_ok(VendorId::Kimi, &plan, lines))
}

fn grok_auth_path() -> std::path::PathBuf {
    home_dir().join(".grok").join("auth.json")
}

fn read_grok_key() -> Option<String> {
    read_json_token(
        &grok_auth_path(),
        &["key", "api_key", "access_token", "token"],
    )
}

fn fetch_supergrok(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://cli-chat-proxy.grok.com/v1/billing",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let mut lines = Vec::new();
    if let Some((w, pct)) = body
        .get("weekly")
        .or_else(|| body.get("included"))
        .and_then(|value| {
            json_f64(value, &["percent", "utilization", "usedPercent"]).map(|pct| (value, pct))
        })
    {
        let reset = json_str(w, &["resets_at", "resetAt"]);
        lines.push(progress_pct(
            "weekly", "Semanal", pct, reset, 604_800, "always",
        ));
    }
    if let Some(bal) = json_f64(&body, &["prepaid_balance", "balance"]) {
        lines.push(values_line(
            "prepaid",
            "Prepago",
            &format!("${bal:.2}"),
            "demand",
        ));
    }
    if lines.is_empty() {
        if let Some(pct) = json_f64(&body, &["percent", "utilization"]) {
            lines.push(progress_pct("usage", "Uso", pct, None, 604_800, "always"));
        }
    }
    Ok(snapshot_ok(VendorId::Supergrok, "SuperGrok", lines))
}

fn commandcode_auth() -> std::path::PathBuf {
    home_dir().join(".commandcode").join("auth.json")
}
fn pi_auth() -> std::path::PathBuf {
    home_dir().join(".pi").join("agent").join("auth.json")
}

fn fetch_commandcode(token: &str) -> Result<ProviderSnapshot, FetchError> {
    let headers = [("Authorization", format!("Bearer {token}"))];
    let refs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let credits = http::get_json("https://api.commandcode.ai/alpha/billing/credits", &refs)?;
    let subs = http::get_json(
        "https://api.commandcode.ai/alpha/billing/subscriptions",
        &refs,
    )
    .ok();
    let mut lines = Vec::new();
    if let Some(w) = credits
        .get("fiveHour")
        .or_else(|| credits.pointer("/windowLimits/fiveHour"))
    {
        if let Some((used, cap)) = json_f64(w, &["used", "spent"])
            .zip(json_f64(w, &["limit", "cap"]))
            .filter(|(_, cap)| *cap > 0.0)
        {
            lines.push(progress_pct(
                "session",
                "5h",
                (used / cap) * 100.0,
                json_str(w, &["resetsAt", "resets_at"]),
                18_000,
                "always",
            ));
        }
    }
    if let Some(w) = credits
        .get("weekly")
        .or_else(|| credits.pointer("/windowLimits/weekly"))
    {
        if let Some((used, cap)) = json_f64(w, &["used", "spent"])
            .zip(json_f64(w, &["limit", "cap"]))
            .filter(|(_, cap)| *cap > 0.0)
        {
            lines.push(progress_pct(
                "weekly",
                "Semanal",
                (used / cap) * 100.0,
                json_str(w, &["resetsAt", "resets_at"]),
                604_800,
                "always",
            ));
        }
    }
    let remaining = json_f64(&credits, &["remaining", "credits", "balance"]);
    if let Some(r) = remaining {
        lines.push(values_line(
            "credits",
            "Créditos",
            &format!("${r:.2}"),
            "demand",
        ));
    }
    let plan = subs
        .as_ref()
        .and_then(|s| json_str(s, &["plan", "planId", "name"]))
        .unwrap_or_else(|| "Command Code".into());
    Ok(snapshot_ok(VendorId::CommandCode, &plan, lines))
}

fn nous_creds() -> std::path::PathBuf {
    app_config_dir().join("credentials.json")
}

fn fetch_nous(token: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://portal.nousresearch.com/api/oauth/account",
        &[("Authorization", &format!("Bearer {token}"))],
    )?;
    let mut lines = Vec::new();
    if let Some((monthly, remaining)) =
        json_f64(&body, &["monthly_credits", "subscription_credits"])
            .zip(json_f64(
                &body,
                &["subscription_credits_remaining", "remaining"],
            ))
            .filter(|(monthly, _)| *monthly > 0.0)
    {
        lines.push(progress_pct(
            "sub",
            "Suscripción",
            ((monthly - remaining) / monthly) * 100.0,
            json_str(&body, &["renews_at", "renewal"]),
            2_592_000,
            "always",
        ));
        lines.push(values_line(
            "credits",
            "Créditos",
            &format!("{remaining:.0} / {monthly:.0}"),
            "demand",
        ));
    }
    Ok(snapshot_ok(VendorId::Nous, "Nous", lines))
}

impl Provider for Windsurf {
    fn id(&self) -> VendorId {
        VendorId::Windsurf
    }
    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        windsurf_installed()
    }
    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        if !windsurf_installed() {
            return snapshot_with_status(
                VendorId::Windsurf,
                ProviderStatus::Unavailable,
                ProviderStatusReason::LocalServiceUnavailable,
                "Windsurf no está disponible en este equipo",
            );
        }
        snapshot_ok(
            VendorId::Windsurf,
            "Windsurf",
            vec![
                badge_line("status", "Estado", "Instalado"),
                values_line(
                    "note",
                    "Cuota",
                    "Abre Windsurf para ver los límites del plan",
                    "always",
                ),
            ],
        )
    }
}

fn windsurf_installed() -> bool {
    let config = dirs::config_dir().unwrap_or_else(home_dir).join("Windsurf");
    let user = config.join("User");
    let db = user.join("globalStorage").join("state.vscdb");
    db.exists() || user.exists()
}

fn read_json_token(path: &std::path::Path, keys: &[&str]) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}
