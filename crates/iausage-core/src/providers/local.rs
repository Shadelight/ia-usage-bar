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
        let Some(session) = read_grok_session() else {
            return snapshot_needs_auth(VendorId::Supergrok, VendorId::Supergrok.login_hint());
        };
        // La CLI renueva `key` con su refresh token solo cuando se usa; uno
        // vencido solo consigue un 401.
        if session.expired {
            return grok_session_expired();
        }
        match fetch_supergrok(&session.key) {
            Ok(s) => s,
            Err(FetchError::Http(401, _)) => grok_session_expired(),
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

struct GrokSession {
    key: String,
    expired: bool,
}

fn read_grok_session() -> Option<GrokSession> {
    let bytes = std::fs::read(grok_auth_path()).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    grok_session_from(&v, chrono::Utc::now().timestamp())
}

/// La CLI actual anida la sesión bajo el emisor y la cuenta:
/// `{"https://auth.x.ai::<id>": {"key", "refresh_token", "expires_at", ...}}`.
/// Las versiones viejas escribían `{"key": ...}` en la raíz; se aceptan ambas.
fn grok_session_from(v: &Value, now_unix: i64) -> Option<GrokSession> {
    let nested = v
        .as_object()
        .into_iter()
        .flat_map(|entries| entries.values());
    std::iter::once(v).chain(nested).find_map(|entry| {
        let key = ["key", "api_key", "access_token", "token"]
            .iter()
            .find_map(|k| {
                entry
                    .get(*k)
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
            })?;
        let expired = entry
            .get("expires_at")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .is_some_and(|at| at.timestamp() <= now_unix);
        Some(GrokSession {
            key: key.to_string(),
            expired,
        })
    })
}

fn grok_session_expired() -> ProviderSnapshot {
    snapshot_with_status(
        VendorId::Supergrok,
        ProviderStatus::NeedsAuth,
        ProviderStatusReason::OAuthExpired,
        "La sesión de la CLI de Grok caducó: ejecuta `grok` (en Windows, %USERPROFILE%\\.grok\\bin\\grok.exe) para renovarla.",
    )
}

fn fetch_supergrok(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://cli-chat-proxy.grok.com/v1/billing",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    Ok(snapshot_ok(
        VendorId::Supergrok,
        "SuperGrok",
        supergrok_lines(&body),
    ))
}

fn supergrok_lines(body: &Value) -> Vec<crate::model::MetricLine> {
    let mut lines = Vec::new();
    // Respuesta actual: {"config": {"used": {"val"}, "monthlyLimit": {"val"},
    // "billingPeriodEnd", "history": [...]}}. `val` no tiene unidad
    // documentada, así que solo se usa como proporción used/limit. Un límite
    // 0 significa que el plan no tiene pago por uso: no hay cuota que medir.
    if let Some(config) = body.get("config") {
        let val = |k: &str| {
            config
                .get(k)
                .and_then(|v| v.get("val"))
                .and_then(Value::as_f64)
        };
        match (val("used"), val("monthlyLimit")) {
            (Some(used), Some(limit)) if limit > 0.0 => lines.push(progress_pct(
                "monthly",
                "Uso mensual",
                used / limit * 100.0,
                json_str(config, &["billingPeriodEnd"]),
                2_592_000,
                "always",
            )),
            (Some(used), _) => lines.push(values_line(
                "billing",
                "Pago por uso",
                if used > 0.0 {
                    "Con cargos este período"
                } else {
                    "Sin cargos este período"
                },
                "always",
            )),
            _ => {}
        }
    }
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
        if let Some(pct) = json_f64(body, &["percent", "utilization"]) {
            lines.push(progress_pct("usage", "Uso", pct, None, 604_800, "always"));
        }
    }
    lines
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

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-09-14T12:00:00Z")
            .unwrap()
            .timestamp()
    }

    #[test]
    fn supergrok_billing_config_without_metered_limit_reports_no_charges() {
        // Forma real observada (plan sin pago por uso: todo en cero).
        let body = serde_json::json!({"config": {
            "billingPeriodEnd": "2026-10-01T00:00:00+00:00",
            "billingPeriodStart": "2026-09-01T00:00:00+00:00",
            "history": [{"billingCycle": {"month": 8, "year": 2026}, "totalUsed": {"val": 0}}],
            "monthlyLimit": {"val": 0},
            "onDemandCap": {"val": 0},
            "used": {"val": 0}
        }});
        let lines = supergrok_lines(&body);
        assert_eq!(lines.len(), 1);
        assert!(matches!(
            &lines[0],
            crate::model::MetricLine::Values { text, .. } if text == "Sin cargos este período"
        ));
    }

    #[test]
    fn supergrok_billing_config_with_limit_reports_monthly_percent() {
        let body = serde_json::json!({"config": {
            "billingPeriodEnd": "2026-10-01T00:00:00+00:00",
            "monthlyLimit": {"val": 2000},
            "used": {"val": 500}
        }});
        let lines = supergrok_lines(&body);
        assert!(matches!(
            &lines[0],
            crate::model::MetricLine::Progress { used, resets_at: Some(reset), .. }
                if (*used - 25.0).abs() < 0.01 && reset == "2026-10-01T00:00:00+00:00"
        ));
    }

    #[test]
    fn grok_session_is_read_from_the_nested_account_entry() {
        let auth = serde_json::json!({
            "https://auth.x.ai::b1a00492-073a-47ea-816f-4c329264a828": {
                "key": "xai-session",
                "auth_mode": "oidc",
                "refresh_token": "r",
                "expires_at": "2026-09-14T13:00:00.4792648Z"
            }
        });
        let session = grok_session_from(&auth, now()).unwrap();
        assert_eq!(session.key, "xai-session");
        assert!(!session.expired);
    }

    #[test]
    fn expired_grok_session_is_flagged_and_flat_legacy_file_still_reads() {
        let nested = serde_json::json!({
            "https://auth.x.ai::id": {"key": "k", "expires_at": "2026-08-31T19:05:46.479264800Z"}
        });
        assert!(grok_session_from(&nested, now()).unwrap().expired);
        let flat = serde_json::json!({"key": "legacy"});
        let session = grok_session_from(&flat, now()).unwrap();
        assert_eq!(session.key, "legacy");
        assert!(!session.expired);
        assert!(grok_session_from(&serde_json::json!({}), now()).is_none());
        assert_eq!(
            grok_session_expired().status_reason,
            Some(ProviderStatusReason::OAuthExpired)
        );
    }
}
