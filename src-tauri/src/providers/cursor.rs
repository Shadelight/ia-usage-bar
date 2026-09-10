//! Cursor — token local de `state.vscdb` + `cursor.com/api/usage-summary`.

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::jwt;
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_err, snapshot_ok, values_line, ProviderSnapshot,
    VendorId,
};

use super::Provider;

const TOKEN_KEY: &str = "cursorAuth/accessToken";
const USAGE_URL: &str = "https://cursor.com/api/usage-summary";
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Cursor;

impl Provider for Cursor {
    fn id(&self) -> VendorId {
        VendorId::Cursor
    }

    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        read_token().is_some()
    }

    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let Some(token) = read_token() else {
            return snapshot_err(VendorId::Cursor, "Cursor no conectado");
        };
        let Some(cookie) = cookie_value(&token) else {
            return snapshot_err(
                VendorId::Cursor,
                "Token de Cursor ilegible. Vuelve a iniciar sesión en el IDE.",
            );
        };
        match fetch_summary(&cookie) {
            Ok(body) => snapshot_from_json(&body).unwrap_or_else(|e| snapshot_err(VendorId::Cursor, &e)),
            Err(e) => super::map_fetch_err(VendorId::Cursor, e),
        }
    }
}

fn db_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(crate::paths::home_dir)
        .join("Cursor")
        .join("User")
        .join("globalStorage")
        .join("state.vscdb")
}

fn agent_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(crate::paths::home_dir)
        .join("cursor")
        .join("auth.json")
}

fn read_token() -> Option<String> {
    read_db_token().or_else(read_agent_token)
}

fn read_db_token() -> Option<String> {
    let path = db_path();
    if !path.exists() {
        return None;
    }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let token: String = conn
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?1",
            [TOKEN_KEY],
            |row| row.get(0),
        )
        .ok()?;
    if token.trim().is_empty() {
        None
    } else {
        Some(token)
    }
}

fn read_agent_token() -> Option<String> {
    let bytes = std::fs::read(agent_path()).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v.get("accessToken")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

fn cookie_value(token: &str) -> Option<String> {
    let claims = jwt::claims(token)?;
    let sub = claims.get("sub").and_then(|v| v.as_str())?;
    let user_id = sub.split('|').nth(1).filter(|s| !s.is_empty())?;
    Some(format!("{user_id}%3A%3A{token}"))
}

fn fetch_summary(cookie: &str) -> Result<Value, FetchError> {
    http::get_json(
        USAGE_URL,
        &[
            ("Cookie", &format!("WorkosCursorSessionToken={cookie}")),
            ("Origin", "https://cursor.com"),
            ("Referer", "https://cursor.com/dashboard"),
            ("User-Agent", BROWSER_UA),
        ],
    )
}

fn pretty_plan(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "ultra" => "Ultra".into(),
        "pro" => "Pro".into(),
        "free" => "Free".into(),
        "hobby" => "Hobby".into(),
        "business" => "Business".into(),
        other if !other.is_empty() => {
            let mut c = other.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => "Cursor".into(),
            }
        }
        _ => "Cursor".into(),
    }
}

pub fn snapshot_from_json(body: &Value) -> Result<ProviderSnapshot, String> {
    let plan = body
        .get("membershipType")
        .and_then(|v| v.as_str())
        .map(pretty_plan)
        .unwrap_or_else(|| "Cursor".into());
    let reset = json_str(body, &["billingCycleEnd"]);
    let mut lines = Vec::new();
    if let Some(plan_obj) = body.pointer("/individualUsage/plan") {
        let total = json_f64(plan_obj, &["totalPercentUsed"]).unwrap_or(0.0);
        let auto = json_f64(plan_obj, &["autoPercentUsed"]).unwrap_or(0.0);
        let api = json_f64(plan_obj, &["apiPercentUsed"]).unwrap_or(0.0);
        lines.push(progress_pct(
            "total",
            "Uso total",
            total,
            reset.clone(),
            2_592_000,
            "always",
        ));
        lines.push(progress_pct(
            "cursor_models",
            "Cursor Models",
            auto,
            reset.clone(),
            2_592_000,
            "always",
        ));
        lines.push(progress_pct(
            "other_models",
            "Other Models",
            api,
            reset.clone(),
            2_592_000,
            "always",
        ));
    } else {
        // Fallback: mensajes de display de cuentas team/enterprise.
        if let Some(pct) = parse_display_pct(
            body.get("autoModelSelectedDisplayMessage")
                .and_then(|v| v.as_str()),
        ) {
            lines.push(progress_pct(
                "cursor_models",
                "Cursor Models",
                pct,
                reset.clone(),
                2_592_000,
                "always",
            ));
        }
        if let Some(pct) = parse_display_pct(
            body.get("namedModelSelectedDisplayMessage")
                .and_then(|v| v.as_str()),
        ) {
            lines.push(progress_pct(
                "other_models",
                "Other Models",
                pct,
                reset.clone(),
                2_592_000,
                "always",
            ));
        }
    }
    if lines.is_empty() {
        return Err("Cursor no devolvió porcentajes de uso para este plan".into());
    }
    let on_demand = body
        .pointer("/individualUsage/onDemand/enabled")
        .or_else(|| body.pointer("/teamUsage/onDemand/enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    lines.push(values_line(
        "ondemand",
        "On-demand",
        if on_demand { "Activado" } else { "Desactivado" },
        "demand",
    ));
    Ok(snapshot_ok(VendorId::Cursor, &plan, lines))
}

fn parse_display_pct(msg: Option<&str>) -> Option<f64> {
    let msg = msg?;
    let digits: String = msg
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
}
