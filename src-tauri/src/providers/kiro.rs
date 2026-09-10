//! Kiro CLI — sesión local de `data.sqlite3` + `GetUsageLimits`.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    progress_pct, resets_from_unix, snapshot_ok, values_line, ProviderSnapshot,
    VendorId,
};
use crate::paths::{app_config_dir, home_dir};

use super::Provider;

const TOKEN_KEY: &str = "kirocli:odic:token";
const DEVICE_KEY: &str = "kirocli:odic:device-registration";
const PROFILE_KEY: &str = "api.codewhisperer.profile";
const REQUEST_TARGET: &str = "AmazonCodeWhispererService.GetUsageLimits";
const REFRESH_BUFFER: i64 = 300;

pub struct Kiro;

impl Provider for Kiro {
    fn id(&self) -> VendorId {
        VendorId::Kiro
    }

    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        db_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        match load_and_fetch() {
            Ok(snap) => snap,
            Err(e) => super::map_fetch_err(VendorId::Kiro, e),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TokenRow {
    access_token: String,
    refresh_token: String,
    expires_at: String,
    region: String,
}

#[derive(Debug, Deserialize)]
struct DeviceRow {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Default, Deserialize)]
struct ProfileRow {
    #[serde(default)]
    arn: String,
}

struct Creds {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
    region: String,
    client_id: String,
    client_secret: String,
    profile_arn: String,
}

fn db_path() -> Option<std::path::PathBuf> {
    let candidates = [
        dirs::data_local_dir().map(|p| p.join("kiro-cli").join("data.sqlite3")),
        dirs::data_dir().map(|p| p.join("kiro-cli").join("data.sqlite3")),
        Some(home_dir().join(".local").join("share").join("kiro-cli").join("data.sqlite3")),
    ];
    candidates.into_iter().flatten().find(|p| p.exists())
}

fn load_and_fetch() -> Result<ProviderSnapshot, FetchError> {
    let path = db_path().ok_or_else(|| {
        FetchError::Parse("Kiro: no hay `data.sqlite3`. Ejecuta `kiro-cli login`.".into())
    })?;
    let mut creds = read_credentials(&path)?;
    apply_cached_oauth(&mut creds);
    if needs_refresh(&creds) {
        refresh_oauth(&mut creds)?;
    }
    fetch_limits(&creds)
}

fn read_credentials(path: &std::path::Path) -> Result<Creds, FetchError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| FetchError::Parse(format!("Kiro: no se pudo abrir la base local ({e})")))?;
    let token: TokenRow = read_kv(&conn, "auth_kv", TOKEN_KEY)?;
    let device: DeviceRow = read_kv(&conn, "auth_kv", DEVICE_KEY)?;
    let profile = read_optional_kv::<ProfileRow>(&conn, "state", PROFILE_KEY)
        .ok()
        .flatten()
        .unwrap_or_default();
    if !valid_region(&token.region) {
        return Err(FetchError::Parse(
            "Kiro: región AWS inválida. Ejecuta `kiro-cli login`.".into(),
        ));
    }
    let expires_at = DateTime::parse_from_rfc3339(&token.expires_at)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| FetchError::Parse("Kiro: caducidad del token ilegible.".into()))?;
    Ok(Creds {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at,
        region: token.region,
        client_id: device.client_id,
        client_secret: device.client_secret,
        profile_arn: profile.arn,
    })
}

fn read_kv<T: for<'de> Deserialize<'de>>(
    conn: &Connection,
    table: &str,
    key: &str,
) -> Result<T, FetchError> {
    let sql = format!("SELECT value FROM {table} WHERE key = ?1");
    let raw: String = conn
        .query_row(&sql, [key], |row| row.get(0))
        .map_err(|_| FetchError::Parse(format!("Kiro: falta `{key}`. Ejecuta `kiro-cli login`.")))?;
    serde_json::from_str(&raw)
        .map_err(|e| FetchError::Parse(format!("Kiro: `{key}` malformado ({e})")))
}

fn read_optional_kv<T: for<'de> Deserialize<'de>>(
    conn: &Connection,
    table: &str,
    key: &str,
) -> Result<Option<T>, FetchError> {
    match read_kv::<T>(conn, table, key) {
        Ok(v) => Ok(Some(v)),
        Err(_) => Ok(None),
    }
}

fn valid_region(region: &str) -> bool {
    let parts: Vec<_> = region.split('-').collect();
    (3..=5).contains(&parts.len())
        && region.len() <= 32
        && parts[0].chars().all(|c| c.is_ascii_lowercase())
        && parts[parts.len() - 1].chars().all(|c| c.is_ascii_digit())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

fn needs_refresh(creds: &Creds) -> bool {
    creds.expires_at.timestamp() - Utc::now().timestamp() < REFRESH_BUFFER
}

fn oauth_cache_path() -> std::path::PathBuf {
    app_config_dir().join("kiro-oauth.json")
}

fn apply_cached_oauth(creds: &mut Creds) {
    let Ok(raw) = std::fs::read_to_string(oauth_cache_path()) else {
        return;
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return;
    };
    let Some(exp) = v
        .get("expires_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
    else {
        return;
    };
    if exp <= creds.expires_at {
        return;
    }
    if let Some(at) = v.get("access_token").and_then(|x| x.as_str()) {
        creds.access_token = at.to_string();
        creds.expires_at = exp;
    }
    if let Some(rt) = v.get("refresh_token").and_then(|x| x.as_str()) {
        if !rt.is_empty() {
            creds.refresh_token = rt.to_string();
        }
    }
}

fn refresh_oauth(creds: &mut Creds) -> Result<(), FetchError> {
    let url = format!("https://oidc.{}.amazonaws.com/token", creds.region);
    let body = serde_json::json!({
        "clientId": creds.client_id,
        "clientSecret": creds.client_secret,
        "grantType": "refresh_token",
        "refreshToken": creds.refresh_token,
    });
    let resp = http::post_json(&url, &[("Content-Type", "application/json")], &body)?;
    let access = resp
        .get("accessToken")
        .or_else(|| resp.get("access_token"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| FetchError::Parse("Kiro: refresh sin accessToken".into()))?;
    creds.access_token = access.to_string();
    if let Some(rt) = resp
        .get("refreshToken")
        .or_else(|| resp.get("refresh_token"))
        .and_then(|v| v.as_str())
    {
        if !rt.is_empty() {
            creds.refresh_token = rt.to_string();
        }
    }
    let secs = resp
        .get("expiresIn")
        .or_else(|| resp.get("expires_in"))
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);
    if let Some(dt) = DateTime::from_timestamp(Utc::now().timestamp() + secs as i64, 0) {
        creds.expires_at = dt;
    }
    let _ = std::fs::create_dir_all(app_config_dir());
    let _ = std::fs::write(
        oauth_cache_path(),
        serde_json::to_vec_pretty(&serde_json::json!({
            "access_token": creds.access_token,
            "refresh_token": creds.refresh_token,
            "expires_at": creds.expires_at.to_rfc3339(),
        }))
        .unwrap_or_default(),
    );
    Ok(())
}

fn fetch_limits(creds: &Creds) -> Result<ProviderSnapshot, FetchError> {
    let url = format!("https://codewhisperer.{}.amazonaws.com/", creds.region);
    let body = serde_json::json!({
        "origin": "AI_EDITOR",
        "profileArn": creds.profile_arn,
        "resourceType": "AGENTIC_REQUEST",
    });
    let authz = format!("Bearer {}", creds.access_token);
    let parsed = http::post_json(
        &url,
        &[
            ("Authorization", &authz),
            ("Content-Type", "application/x-amz-json-1.0"),
            ("x-amz-target", REQUEST_TARGET),
        ],
        &body,
    )?;
    Ok(snapshot_from_json(&parsed))
}

pub fn snapshot_from_json(body: &Value) -> ProviderSnapshot {
    let plan = body
        .pointer("/subscriptionInfo/subscriptionTitle")
        .and_then(|v| v.as_str())
        .unwrap_or("Kiro");
    let list = body
        .get("usageBreakdownList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let credit = list
        .iter()
        .find(|b| b.get("resourceType").and_then(|v| v.as_str()) == Some("CREDIT"))
        .or_else(|| list.first());
    let mut lines = Vec::new();
    if let Some(row) = credit {
        let used = row
            .get("currentUsageWithPrecision")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let limit = row
            .get("usageLimitWithPrecision")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let reset = body
            .get("nextDateReset")
            .and_then(|v| v.as_f64())
            .map(|secs| resets_from_unix(secs as i64).0)
            .flatten();
        if limit > 0.0 {
            lines.push(progress_pct(
                "credits",
                "Créditos",
                (used / limit) * 100.0,
                reset,
                2_592_000,
                "always",
            ));
        }
        lines.push(values_line(
            "credit_n",
            "Uso",
            &format!("{used:.1} / {limit:.0}"),
            "always",
        ));
    }
    snapshot_ok(VendorId::Kiro, plan, lines)
}
