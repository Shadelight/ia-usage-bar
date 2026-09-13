//! Cursor — token local de `state.vscdb` + `cursor.com/api/usage-summary`.

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::jwt;
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok, snapshot_with_status,
    values_line, ProviderSnapshot, ProviderStatus, ProviderStatusReason, VendorId,
};

use super::Provider;

const TOKEN_KEY: &str = "cursorAuth/accessToken";
const USAGE_URL: &str = "https://cursor.com/api/usage-summary";
const SAND_USAGE_URL: &str =
    "https://api2.cursor.sh/aiserver.v1.DashboardService/GetSandUsageStatus";
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
            return snapshot_needs_auth(VendorId::Cursor, "Cursor no conectado");
        };
        let Some(cookie) = cookie_value(&token) else {
            return snapshot_with_status(
                VendorId::Cursor,
                ProviderStatus::NeedsAuth,
                ProviderStatusReason::InvalidCredential,
                "Token de Cursor ilegible. Vuelve a iniciar sesión en el IDE.",
            );
        };
        match fetch_summary(&cookie) {
            Ok(body) => snapshot_from_json(&body)
                .map(|mut snapshot| {
                    // Grok Bot has a separate weekly allowance. This endpoint is
                    // supplementary: failure must not discard the normal Cursor
                    // quota that was already fetched successfully.
                    if let Ok(sand) = fetch_sand_usage(&token) {
                        append_grok_bot(&mut snapshot, &sand);
                    }
                    snapshot
                })
                .unwrap_or_else(|e| {
                    snapshot_with_status(
                        VendorId::Cursor,
                        ProviderStatus::Error,
                        ProviderStatusReason::ParseFailed,
                        &e,
                    )
                }),
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

fn fetch_sand_usage(token: &str) -> Result<Value, FetchError> {
    http::post_json(
        SAND_USAGE_URL,
        &[
            ("Authorization", &format!("Bearer {token}")),
            ("Content-Type", "application/json"),
            ("Connect-Protocol-Version", "1"),
        ],
        &Value::Object(Default::default()),
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
        for (id, label, field) in [
            ("total", "Uso total", "totalPercentUsed"),
            ("cursor_models", "Cursor Models", "autoPercentUsed"),
            ("other_models", "Other Models", "apiPercentUsed"),
        ] {
            if let Some(pct) = json_f64(plan_obj, &[field]) {
                lines.push(progress_pct(
                    id,
                    label,
                    pct,
                    reset.clone(),
                    2_592_000,
                    "always",
                ));
            }
        }
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

/// Add Cursor's independent Grok Bot weekly allowance when the user has it.
/// `GetSandUsageStatus` has appeared both as a direct object and nested under
/// `response`, so tolerate both representations without guessing from HTML.
fn append_grok_bot(snapshot: &mut ProviderSnapshot, body: &Value) {
    let status = body.get("response").unwrap_or(body);
    let Some(used) = json_f64(status, &["usagePercent"]) else {
        return;
    };
    let reset = json_str(status, &["nextResetTimestampUtc"])
        .or_else(|| unix_reset(status.get("nextResetTimestampUtc")));
    let line = progress_pct("grok_bot", "Grok Bot", used, reset, 604_800, "always");
    snapshot.quotas.extend(line_to_quota(&line, snapshot));
    snapshot.lines.push(line);
}

fn unix_reset(value: Option<&Value>) -> Option<String> {
    let seconds = value?
        .as_i64()
        .or_else(|| value?.as_f64().map(|n| n as i64))?;
    let seconds = if seconds > 10_000_000_000 {
        seconds / 1_000
    } else {
        seconds
    };
    chrono::DateTime::from_timestamp(seconds, 0).map(|time| time.to_rfc3339())
}

// `snapshot_ok` normally performs this conversion. This small adapter keeps
// the Grok line consistent with the rest of a already-normalized snapshot.
fn line_to_quota(
    line: &crate::model::MetricLine,
    snapshot: &ProviderSnapshot,
) -> Vec<crate::model::UsageQuota> {
    let crate::model::MetricLine::Progress {
        id,
        label,
        used,
        remaining,
        resets_at,
        ..
    } = line
    else {
        return Vec::new();
    };
    let reset_in_seconds = resets_at.as_deref().and_then(|iso| {
        chrono::DateTime::parse_from_rfc3339(iso).ok().map(|time| {
            (time.with_timezone(&chrono::Utc) - chrono::Utc::now())
                .num_seconds()
                .max(0)
        })
    });
    vec![crate::model::UsageQuota {
        id: id.clone(),
        label: label.clone(),
        window_type: crate::model::WindowType::Weekly,
        used_percent: Some(*used),
        remaining_percent: Some(*remaining),
        used_amount: None,
        limit_amount: None,
        unit: Some(crate::model::UsageUnit::Percent),
        reset_at: resets_at.clone(),
        reset_in_seconds,
        reset_status: if resets_at.is_some() && reset_in_seconds.is_some() {
            crate::model::ResetStatus::Known
        } else {
            crate::model::ResetStatus::NotProvided
        },
        temporary_multiplier: None,
        temporary_expires_at: None,
        source: snapshot
            .active_source
            .unwrap_or(crate::model::UsageSource::LocalSession),
        fetched_at: snapshot.updated_at.clone(),
        stale: snapshot.stale,
        confidence: crate::model::DataConfidence::Exact,
    }]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_grok_bot_weekly_allowance() {
        let summary = serde_json::json!({
            "membershipType": "pro",
            "individualUsage": { "plan": { "totalPercentUsed": 8.0 } }
        });
        let mut snapshot = snapshot_from_json(&summary).unwrap();
        let sand = serde_json::json!({
            "usagePercent": 25.0,
            "nextResetTimestampUtc": "2099-01-08T00:00:00Z"
        });
        append_grok_bot(&mut snapshot, &sand);
        let quota = snapshot
            .quotas
            .iter()
            .find(|quota| quota.id == "grok_bot")
            .unwrap();
        assert_eq!(quota.window_type, crate::model::WindowType::Weekly);
        assert_eq!(quota.used_percent, Some(25.0));
    }

    #[test]
    fn only_emits_percentages_present_in_cursor_response() {
        let snapshot = snapshot_from_json(&serde_json::json!({
            "individualUsage": { "plan": { "totalPercentUsed": 8.0 } }
        }))
        .unwrap();
        assert_eq!(snapshot.quotas.len(), 1);
        assert_eq!(snapshot.quotas[0].id, "total");
        assert_eq!(snapshot.quotas[0].used_percent, Some(8.0));
    }
}
