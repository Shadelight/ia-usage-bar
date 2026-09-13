//! Codex / ChatGPT — OAuth del CLI `codex` + usage de chatgpt.com.

use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::jwt;
use std::path::Path;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use walkdir::WalkDir;

use crate::model::{
    progress_pct, resets_from_unix, snapshot_needs_auth, snapshot_ok, values_line,
    ProviderSnapshot, VendorId,
};
use crate::paths::home_dir;
use crate::watch::codex_sessions_dir;

use super::Provider;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const SCOPE: &str = "openid profile email";
const REFRESH_BUFFER: i64 = 300;

pub struct Codex;

impl Provider for Codex {
    fn id(&self) -> VendorId {
        VendorId::Openai
    }

    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        read_auth().is_some()
    }

    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let Some(mut auth) = read_auth() else {
            return snapshot_needs_auth(VendorId::Openai, "Codex no conectado");
        };
        if needs_refresh(&auth) {
            if let Err(e) = refresh_tokens(&mut auth) {
                return super::map_fetch_err(VendorId::Openai, e);
            }
        }
        let plan_hint = plan_from_id(&auth.id_token);
        match fetch_usage(&auth) {
            Ok(body) => snapshot_from_json(&body, plan_hint.as_deref()),
            Err(e) => super::map_fetch_err(VendorId::Openai, e),
        }
    }
}

struct Auth {
    access_token: String,
    refresh_token: String,
    id_token: String,
    account_id: Option<String>,
    expires_at: Option<String>,
    /// Keys at the root of `auth.json`, outside `tokens`.
    extra: serde_json::Map<String, Value>,
    /// Keys inside `tokens` that this struct does not name — preserved on write-back.
    tokens_extra: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct AuthFile {
    tokens: Tokens,
    #[serde(flatten, default)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    id_token: String,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

fn auth_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        if !home.trim().is_empty() {
            return std::path::PathBuf::from(home).join("auth.json");
        }
    }
    home_dir().join(".codex").join("auth.json")
}

/// Fuente rápida para el watcher: abre los últimos `rate_limits` que Codex
/// acaba de registrar. Nunca lee ni propaga mensajes de sesión; la salida se
/// reduce inmediatamente al mismo `ProviderSnapshot` normalizado.
pub fn snapshot_from_sessions() -> Option<ProviderSnapshot> {
    let rate_limits = latest_session_rate_limits(&codex_sessions_dir())?;
    let body = session_rate_limits_body(&rate_limits)?;
    let plan_hint = read_auth().and_then(|auth| plan_from_id(&auth.id_token));
    let snapshot = snapshot_from_json(&body, plan_hint.as_deref());
    (!snapshot.quotas.is_empty()).then_some(snapshot)
}

fn latest_session_rate_limits(root: &Path) -> Option<Value> {
    let mut files: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
        })
        .filter_map(|entry| {
            entry.metadata().ok().and_then(|meta| {
                meta.modified()
                    .ok()
                    .map(|modified| (modified, entry.into_path()))
            })
        })
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.into_iter().take(20).find_map(|(_, path)| {
        let mut latest = None;
        let raw = read_tail(&path, 1_048_576).ok()?;
        for line in raw.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if let Some(rate) = find_rate_limits(&value) {
                latest = Some(rate.clone());
            }
        }
        latest
    })
}

fn read_tail(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut raw = String::new();
    file.read_to_string(&mut raw)?;
    if start > 0 {
        Ok(raw
            .split_once('\n')
            .map(|(_, rest)| rest)
            .unwrap_or("")
            .to_string())
    } else {
        Ok(raw)
    }
}

fn find_rate_limits(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(map) => map
            .get("rate_limits")
            .or_else(|| map.values().find_map(find_rate_limits)),
        Value::Array(values) => values.iter().find_map(find_rate_limits),
        _ => None,
    }
}

fn session_rate_limits_body(rate_limits: &Value) -> Option<Value> {
    let primary = rate_limits
        .get("primary_window")
        .or_else(|| rate_limits.get("primary"))
        .and_then(normalize_session_window)?;
    let secondary = rate_limits
        .get("secondary_window")
        .or_else(|| rate_limits.get("secondary"))
        .and_then(normalize_session_window);
    let mut limit = serde_json::Map::new();
    limit.insert("primary_window".into(), primary);
    if let Some(window) = secondary {
        limit.insert("secondary_window".into(), window);
    }
    Some(serde_json::json!({ "rate_limit": Value::Object(limit) }))
}

fn normalize_session_window(window: &Value) -> Option<Value> {
    let used = normalized_used_percent(window)?;
    let seconds = window
        .get("limit_window_seconds")
        .and_then(Value::as_i64)
        .or_else(|| window.get("window_seconds").and_then(Value::as_i64))
        .or_else(|| {
            window
                .get("window_minutes")
                .and_then(Value::as_i64)
                .map(|mins| mins * 60)
        })?;
    let mut normalized =
        serde_json::json!({ "used_percent": used, "limit_window_seconds": seconds });
    let reset = window.get("reset_at").or_else(|| window.get("resets_at"));
    if let Some(seconds) = reset.and_then(Value::as_i64) {
        normalized["reset_at"] = Value::from(seconds);
    } else if let Some(iso) = reset
        .and_then(Value::as_str)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    {
        normalized["reset_at"] = Value::from(iso.timestamp());
    } else if let Some(after) = window.get("reset_after_seconds").and_then(Value::as_i64) {
        normalized["reset_after_seconds"] = Value::from(after);
    }
    Some(normalized)
}

fn read_auth() -> Option<Auth> {
    read_auth_at(&auth_path())
}

fn read_auth_at(path: &Path) -> Option<Auth> {
    let raw = std::fs::read_to_string(path).ok()?;
    let file: AuthFile = serde_json::from_str(&raw).ok()?;
    if file.tokens.access_token.trim().is_empty() {
        return None;
    }
    Some(Auth {
        access_token: file.tokens.access_token,
        refresh_token: file.tokens.refresh_token,
        id_token: file.tokens.id_token,
        account_id: file.tokens.account_id,
        expires_at: file.tokens.expires_at,
        extra: file.extra,
        tokens_extra: file.tokens.extra,
    })
}

fn expires_at_secs(auth: &Auth) -> i64 {
    auth.expires_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .or_else(|| jwt::exp_secs(&auth.access_token))
        .or_else(|| jwt::exp_secs(&auth.id_token))
        .unwrap_or(0)
}

fn needs_refresh(auth: &Auth) -> bool {
    expires_at_secs(auth) - Utc::now().timestamp() < REFRESH_BUFFER
}

fn plan_from_id(id_token: &str) -> Option<String> {
    let claims = jwt::claims(id_token)?;
    claims
        .get("https://api.openai.com/auth")
        .and_then(|v| v.get("chatgpt_plan_type"))
        .and_then(|v| v.as_str())
        .map(|s| pretty_plan(s))
}

fn pretty_plan(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "plus" => "Plus".into(),
        "pro" => "Pro".into(),
        "free" => "Free".into(),
        "team" => "Team".into(),
        "self_serve_business_prolite" => "Business Premium".into(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => "ChatGPT".into(),
            }
        }
    }
}

fn refresh_tokens(auth: &mut Auth) -> Result<(), FetchError> {
    let body = http::post_form(
        TOKEN_URL,
        &[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", &auth.refresh_token),
            ("scope", SCOPE),
        ],
    )?;
    let access = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FetchError::Parse("openai: refresh sin access_token".into()))?;
    auth.access_token = access.to_string();
    if let Some(rt) = body.get("refresh_token").and_then(|v| v.as_str()) {
        if !rt.is_empty() {
            auth.refresh_token = rt.to_string();
        }
    }
    if let Some(id) = body.get("id_token").and_then(|v| v.as_str()) {
        if !id.is_empty() {
            auth.id_token = id.to_string();
        }
    }
    if let Some(secs) = body.get("expires_in").and_then(|v| v.as_u64()) {
        if let Some(dt) = chrono::DateTime::from_timestamp(Utc::now().timestamp() + secs as i64, 0)
        {
            auth.expires_at = Some(dt.to_rfc3339());
        }
    }
    write_back_at(&auth_path(), auth)?;
    Ok(())
}

fn write_back_at(path: &Path, auth: &Auth) -> Result<(), FetchError> {
    let mut tokens = serde_json::json!({
        "access_token": auth.access_token,
        "refresh_token": auth.refresh_token,
        "id_token": auth.id_token,
    });
    if let Some(aid) = &auth.account_id {
        tokens["account_id"] = Value::String(aid.clone());
    }
    if let Some(exp) = &auth.expires_at {
        tokens["expires_at"] = Value::String(exp.clone());
    }
    if let Some(obj) = tokens.as_object_mut() {
        for (k, v) in &auth.tokens_extra {
            obj.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    let mut root = Value::Object(auth.extra.clone());
    root["tokens"] = tokens;
    let bytes = serde_json::to_vec_pretty(&root)
        .map_err(|e| FetchError::Parse(format!("serializar auth.json: {e}")))?;
    // `rename(temp, existing)` does not replace the target on Windows. Reuse
    // the config writer's backup-and-replace algorithm so a refreshed OAuth
    // token survives there without truncating or failing on an existing file.
    crate::config::atomic_write(path, &bytes)
        .map_err(|e| FetchError::Network(format!("escritura auth.json: {e}")))
}

fn fetch_usage(auth: &Auth) -> Result<Value, FetchError> {
    let authz = format!("Bearer {}", auth.access_token);
    let mut owned = vec![
        ("Authorization", authz),
        ("User-Agent", "codex-cli".to_string()),
    ];
    if let Some(aid) = &auth.account_id {
        owned.push(("ChatGPT-Account-Id", aid.clone()));
    }
    let refs: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    http::get_json(USAGE_URL, &refs)
}

fn normalized_used_percent(v: &Value) -> Option<f64> {
    if let Some(used) = v
        .get("used_percent")
        .or_else(|| v.get("usedPercent"))
        .and_then(|x| x.as_f64())
    {
        return Some(used.clamp(0.0, 100.0));
    }
    v.get("remaining_percent")
        .or_else(|| v.get("remainingPercent"))
        .or_else(|| v.get("percent_remaining"))
        .and_then(|x| x.as_f64())
        .map(|remaining| (100.0 - remaining).clamp(0.0, 100.0))
}

fn window_from(v: &Value) -> Option<(f64, Option<String>, i64)> {
    let pct = normalized_used_percent(v)?;
    let secs = v
        .get("limit_window_seconds")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let reset = if let Some(ts) = v.get("reset_at").and_then(|x| x.as_i64()) {
        resets_from_unix(ts).0
    } else if let Some(after) = v.get("reset_after_seconds").and_then(|x| x.as_i64()) {
        resets_from_unix(Utc::now().timestamp() + after).0
    } else {
        None
    };
    Some((pct, reset, secs))
}

fn classify(secs: i64) -> (&'static str, &'static str) {
    if secs > 0 && secs <= 6 * 3600 {
        ("5h", "5 horas")
    } else if secs >= 6 * 24 * 3600 {
        ("weekly", "Semanal")
    } else {
        ("window", "Ventana")
    }
}

pub fn snapshot_from_json(body: &Value, plan_hint: Option<&str>) -> ProviderSnapshot {
    let plan = body
        .get("plan_type")
        .and_then(|v| v.as_str())
        .map(pretty_plan)
        .or_else(|| plan_hint.map(|s| s.to_string()))
        .unwrap_or_else(|| "ChatGPT".into());
    let mut lines = Vec::new();
    if let Some(rl) = body.get("rate_limit") {
        let mut windows = Vec::new();
        if let Some(w) = rl.get("primary_window").and_then(window_from) {
            windows.push(w);
        }
        if let Some(w) = rl.get("secondary_window").and_then(window_from) {
            windows.push(w);
        }
        windows.sort_by_key(|w| w.2);
        for (i, (pct, reset, secs)) in windows.into_iter().enumerate() {
            let (id, label) = if windows_len_hint(rl) == 1 && secs >= 6 * 24 * 3600 {
                ("weekly", "Semanal")
            } else {
                classify(secs)
            };
            let id = match (i, id) {
                (0, "window") => "session",
                (_, "window") => "secondary",
                _ => id,
            };
            lines.push(progress_pct(id, label, pct, reset, secs, "always"));
        }
    }
    if let Some(credits) = body.get("credits") {
        if credits.get("unlimited").and_then(|v| v.as_bool()) == Some(true) {
            lines.push(values_line("credits", "Créditos", "Ilimitados", "demand"));
        } else if let Some(balance) = credits.get("balance") {
            let parsed = balance
                .as_f64()
                .or_else(|| balance.as_str().and_then(|value| value.parse::<f64>().ok()));
            if let Some(value) = parsed {
                lines.push(values_line(
                    "credits",
                    "Créditos",
                    &format!("{:.0}", value.floor()),
                    "demand",
                ));
            }
        }
    }
    if let Some(count) = body
        .pointer("/rate_limit_reset_credits/available_count")
        .and_then(|v| v.as_u64())
    {
        lines.push(values_line(
            "resets",
            "Resets disponibles",
            &format!("{count}"),
            "demand",
        ));
    }
    if let Some(arr) = body
        .get("additional_rate_limits")
        .and_then(|v| v.as_array())
    {
        for extra in arr {
            let name = extra
                .get("limit_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Extra");
            if let Some(rl) = extra.get("rate_limit") {
                if let Some((pct, reset, secs)) = rl.get("primary_window").and_then(window_from) {
                    lines.push(progress_pct(
                        &format!("extra_{}", name.to_ascii_lowercase().replace(' ', "_")),
                        name,
                        pct,
                        reset,
                        secs,
                        "demand",
                    ));
                }
            }
        }
    }
    snapshot_ok(VendorId::Openai, &plan, lines)
}

fn windows_len_hint(rl: &Value) -> usize {
    let mut n = 0;
    if rl.get("primary_window").is_some() {
        n += 1;
    }
    if rl.get("secondary_window").is_some() {
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("codex-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // F-H1: an unknown key inside `tokens` (or at the root) must survive a
    // read -> write_back round trip instead of being silently dropped.
    #[test]
    fn write_back_preserves_unknown_fields() {
        let dir = scratch_dir("roundtrip");
        let path = dir.join("auth.json");
        std::fs::write(
            &path,
            r#"{"OPENAI_API_KEY":"root-key","last_refresh":"2020-01-01",
                "tokens":{"access_token":"a","refresh_token":"r","id_token":"i","custom_claim":"keepme"}}"#,
        )
        .unwrap();

        let auth = read_auth_at(&path).expect("read");
        write_back_at(&path, &auth).expect("write");

        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["tokens"]["custom_claim"], "keepme");
        assert_eq!(v["last_refresh"], "2020-01-01");
        assert_eq!(v["OPENAI_API_KEY"], "root-key");
        assert_eq!(v["tokens"]["access_token"], "a");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_back_replaces_an_existing_auth_file() {
        let dir = scratch_dir("replace-existing");
        let path = dir.join("auth.json");
        std::fs::write(
            &path,
            r#"{"tokens":{"access_token":"old","refresh_token":"old"}}"#,
        )
        .unwrap();
        let auth = Auth {
            access_token: "new-access".into(),
            refresh_token: "new-refresh".into(),
            id_token: String::new(),
            account_id: None,
            expires_at: None,
            extra: serde_json::Map::new(),
            tokens_extra: serde_json::Map::new(),
        };
        write_back_at(&path, &auth).expect("replace existing auth.json");
        let body: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(body["tokens"]["access_token"], "new-access");
        assert_eq!(body["tokens"]["refresh_token"], "new-refresh");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    // The shared atomic writer creates the credential directory when Codex
    // has not written auth.json yet.
    #[test]
    fn write_back_creates_missing_directory_without_partial_file() {
        let path = std::env::temp_dir()
            .join(format!("codex-test-{}-nodir", std::process::id()))
            .join("nested")
            .join("auth.json");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let auth = Auth {
            access_token: "a".into(),
            refresh_token: "r".into(),
            id_token: "i".into(),
            account_id: None,
            expires_at: None,
            extra: serde_json::Map::new(),
            tokens_extra: serde_json::Map::new(),
        };
        write_back_at(&path, &auth).expect("write into a new Codex directory");
        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn remaining_percent_is_converted_once_in_adapter() {
        let window = serde_json::json!({
            "remaining_percent": 71.0,
            "limit_window_seconds": 604800,
            "reset_at": 4103049600i64
        });
        let (used, _, _) = window_from(&window).expect("window");
        assert_eq!(used, 29.0);
        let body = serde_json::json!({
            "rate_limit": {"primary_window": window}
        });
        let snapshot = snapshot_from_json(&body, Some("Plus"));
        assert_eq!(snapshot.quotas[0].used_percent, Some(29.0));
        assert_eq!(snapshot.quotas[0].remaining_percent, Some(71.0));
    }

    #[test]
    fn credits_are_rounded_down_and_reset_count_is_preserved() {
        let body = serde_json::json!({
            "plan_type": "plus",
            "rate_limit": {
                "primary_window": {
                    "used_percent": 0.0,
                    "limit_window_seconds": 18000,
                    "reset_at": 4102444800i64
                }
            },
            "credits": {"balance": "478.0394270000"},
            "rate_limit_reset_credits": {"available_count": 1}
        });
        let snapshot = snapshot_from_json(&body, None);
        let credits = snapshot.credits.expect("credits");
        assert_eq!(credits.remaining, 478.0);
        assert_eq!(credits.resets_available, Some(1));
        assert_eq!(
            snapshot.quotas[0].window_type,
            crate::model::WindowType::FiveHour
        );
    }

    #[test]
    fn session_rate_limits_use_the_same_normalized_snapshot() {
        let rate_limits = serde_json::json!({
            "primary": { "used_percent": 21.0, "window_minutes": 300, "resets_at": 4102444800i64 },
            "secondary": { "remaining_percent": 85.0, "window_minutes": 10080, "resets_at": 4103049600i64 }
        });
        let body = session_rate_limits_body(&rate_limits).expect("normaliza rate_limits");
        let snapshot = snapshot_from_json(&body, Some("Plus"));
        assert_eq!(snapshot.quotas.len(), 2);
        assert_eq!(snapshot.quotas[0].used_percent, Some(21.0));
        assert_eq!(snapshot.quotas[1].used_percent, Some(15.0));
    }
}
