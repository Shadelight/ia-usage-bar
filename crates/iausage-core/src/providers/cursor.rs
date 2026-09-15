//! Cursor — token local de `state.vscdb` + `cursor.com/api/usage-summary`.

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::jwt;
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok, snapshot_with_status,
    values_line, ProductUsage, ProviderSnapshot, ProviderStatus, ProviderStatusReason, VendorId,
};

use super::Provider;

const TOKEN_KEY: &str = "cursorAuth/accessToken";
const USAGE_URL: &str = "https://cursor.com/api/usage-summary";
const SAND_USAGE_URL: &str =
    "https://api2.cursor.sh/aiserver.v1.DashboardService/GetSandUsageStatus";
const AGGREGATED_USAGE_URL: &str =
    "https://api2.cursor.sh/aiserver.v1.DashboardService/GetAggregatedUsageEvents";
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
                    if let Ok(agg) = fetch_aggregated_usage(&token) {
                        append_aggregated_usage(&mut snapshot, &agg);
                    }
                    snapshot.availability = snapshot.compute_availability();
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

fn fetch_aggregated_usage(token: &str) -> Result<Value, FetchError> {
    http::post_json(
        AGGREGATED_USAGE_URL,
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
        // Order is intentional:
        // 1. "cursor_models" first -> picked by snapshot_ok as primary_utilization, first main card
        // 2. "other_models" second -> second main card in UI grid
        // 3. "total" third -> secondary metric ("Uso incluido total"), visible in details
        for (id, label, field, visible) in [
            ("cursor_models", "Cursor Models", "autoPercentUsed", "always"),
            ("other_models", "Other Models", "apiPercentUsed", "always"),
            ("total", "Uso incluido total", "totalPercentUsed", "details"),
        ] {
            if let Some(pct) = json_f64(plan_obj, &[field]) {
                lines.push(progress_pct(
                    id,
                    label,
                    pct,
                    reset.clone(),
                    2_592_000,
                    visible,
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

    let on_demand_obj = body
        .pointer("/individualUsage/onDemand")
        .or_else(|| body.pointer("/teamUsage/onDemand"));
    let on_demand_enabled = on_demand_obj
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let no_usage_based = body
        .get("noUsageBasedAllowed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let on_demand_text = if !on_demand_enabled || no_usage_based {
        "Desactivado".to_string()
    } else if let Some(used) = on_demand_obj.and_then(|o| json_f64(o, &["used"])) {
        format!("${:.2} USD", used)
    } else {
        "Activado".to_string()
    };

    lines.push(values_line(
        "ondemand",
        "On-demand",
        &on_demand_text,
        "details",
    ));
    Ok(tag_cursor_groups(snapshot_ok(VendorId::Cursor, &plan, lines)))
}

fn tag_cursor_groups(mut snapshot: ProviderSnapshot) -> ProviderSnapshot {
    for quota in &mut snapshot.quotas {
        match quota.id.as_str() {
            "cursor_models" => {
                quota.group_id = Some("cursor_models".into());
                quota.group_label = Some("Cursor Models".into());
                quota.visible = "always".into();
            }
            "other_models" => {
                quota.group_id = Some("other_models".into());
                quota.group_label = Some("Other Models".into());
                quota.visible = "always".into();
            }
            "total" => quota.visible = "details".into(),
            _ => {}
        }
    }
    snapshot
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
    let line = progress_pct("grok_bot", "Grok Bot · semanal", used, reset, 604_800, "always");
    snapshot.quotas.extend(line_to_quota(&line, snapshot));
    snapshot.lines.push(line);
}

fn json_u64_or_str(v: &Value, key: &str) -> u64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(Value::String(s)) => s.parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

fn item_has_activity(item: &Value) -> bool {
    let total_cents = json_f64(item, &["totalCents"]).unwrap_or(0.0);
    let input_tokens = json_u64_or_str(item, "inputTokens");
    let output_tokens = json_u64_or_str(item, "outputTokens");
    let cache_write = json_u64_or_str(item, "cacheWriteTokens");
    let cache_read = json_u64_or_str(item, "cacheReadTokens");
    total_cents > 0.0 || input_tokens > 0 || output_tokens > 0 || cache_write > 0 || cache_read > 0
}

fn append_aggregated_usage(snapshot: &mut ProviderSnapshot, body: &Value) {
    let status = body.get("response").unwrap_or(body);
    let items = status
        .get("aggregations")
        .or_else(|| status.get("aggregates"))
        .and_then(|v| v.as_array());

    let Some(raw_items) = items else {
        return;
    };

    let auto_percent_used = snapshot
        .quotas
        .iter()
        .find(|q| q.id == "cursor_models")
        .and_then(|q| q.used_percent)
        .unwrap_or(0.0);

    let api_percent_used = snapshot
        .quotas
        .iter()
        .find(|q| q.id == "other_models")
        .and_then(|q| q.used_percent)
        .unwrap_or(0.0);

    struct ParsedModelItem {
        display_name: String,
        total_cents: f64,
    }

    let mut auto_items: Vec<ParsedModelItem> = Vec::new();
    let mut api_items: Vec<ParsedModelItem> = Vec::new();

    for item in raw_items {
        if !item_has_activity(item) {
            continue;
        }
        let model_intent = item
            .get("modelIntent")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tier = item.get("tier").and_then(|v| v.as_i64());
        let total_cents = json_f64(item, &["totalCents"]).unwrap_or(0.0);

        let is_auto = tier == Some(2) || (tier.is_none() && model_intent == "default");
        let is_api = model_intent != "default" && (tier == Some(1) || tier.is_none());

        if is_auto {
            let display_name = if model_intent == "default" {
                "auto".to_string()
            } else {
                model_intent.to_string()
            };
            auto_items.push(ParsedModelItem {
                display_name,
                total_cents,
            });
        } else if is_api {
            api_items.push(ParsedModelItem {
                display_name: model_intent.to_string(),
                total_cents,
            });
        }
    }

    auto_items.sort_by(|a, b| {
        b.total_cents
            .partial_cmp(&a.total_cents)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    api_items.sort_by(|a, b| {
        b.total_cents
            .partial_cmp(&a.total_cents)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let auto_cents_sum: f64 = auto_items.iter().map(|it| it.total_cents).sum();
    let api_cents_sum: f64 = api_items.iter().map(|it| it.total_cents).sum();

    let mut breakdown = Vec::new();

    for it in auto_items {
        let pct = if auto_cents_sum > 0.0 {
            (it.total_cents / auto_cents_sum) * auto_percent_used
        } else {
            0.0
        };
        let rounded_pct = (pct * 10.0).round() / 10.0;
        breakdown.push(ProductUsage::with_parent_quota(
            it.display_name,
            rounded_pct,
            "cursor_models",
        ));
    }

    for it in api_items {
        let pct = if api_cents_sum > 0.0 {
            (it.total_cents / api_cents_sum) * api_percent_used
        } else {
            0.0
        };
        let rounded_pct = (pct * 10.0).round() / 10.0;
        breakdown.push(ProductUsage::with_parent_quota(
            it.display_name,
            rounded_pct,
            "other_models",
        ));
    }

    snapshot.product_breakdown = breakdown;
    for quota in &mut snapshot.quotas {
        let Some(group_id) = quota.group_id.as_deref() else {
            continue;
        };
        quota.models = snapshot
            .product_breakdown
            .iter()
            .filter(|item| {
                item.group_id.as_deref() == Some(group_id)
                    || item.parent_quota_id.as_deref() == Some(group_id)
            })
            .map(|item| item.name.clone())
            .collect();
    }
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
        pace: None,
        group_id: Some("grok_bot".into()),
        group_label: Some("Grok Bot".into()),
        models: Vec::new(),
        visible: "always".into(),
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
    use crate::model::Availability;

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
        assert_eq!(quota.label, "Grok Bot · semanal");
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

    #[test]
    fn cursor_audit_fixture_regression_test() {
        let summary_json = serde_json::json!({
            "billingCycleStart": "2026-08-23T00:00:00.000Z",
            "billingCycleEnd": "2026-09-23T00:00:00.000Z",
            "membershipType": "pro",
            "limitType": "soft",
            "isUnlimited": false,
            "individualUsage": {
                "plan": {
                    "autoPercentUsed": 38.2711,
                    "apiPercentUsed": 100.0,
                    "totalPercentUsed": 44.0949
                },
                "onDemand": {
                    "enabled": false,
                    "used": 0,
                    "limit": null,
                    "remaining": null
                }
            }
        });

        let agg_json = serde_json::json!({
            "aggregations": [
                {
                    "modelIntent": "default",
                    "inputTokens": "8376982",
                    "outputTokens": "759098",
                    "cacheReadTokens": "133930837",
                    "totalCents": 9168.71588,
                    "tier": 2
                },
                {
                    "modelIntent": "cursor-grok-4.6-high-fast",
                    "inputTokens": "2921968",
                    "outputTokens": "316934",
                    "cacheReadTokens": "38141568",
                    "totalCents": 5327.11363,
                    "tier": 2
                },
                {
                    "modelIntent": "claude-opus-5-thinking-high",
                    "inputTokens": "94818",
                    "outputTokens": "257435",
                    "cacheWriteTokens": "3093675",
                    "cacheReadTokens": "44632069",
                    "totalCents": 4605.26609,
                    "tier": 1
                },
                {
                    "modelIntent": "cursor-grok-4.6-high",
                    "inputTokens": "1370357",
                    "outputTokens": "193456",
                    "cacheReadTokens": "22305280",
                    "totalCents": 1513.0719,
                    "tier": 2
                },
                {
                    "modelIntent": "composer-2.5-fast",
                    "inputTokens": "1056151",
                    "outputTokens": "157151",
                    "cacheReadTokens": "14140576",
                    "totalCents": 1259.6006,
                    "tier": 2
                },
                {
                    "modelIntent": "claude-haiku-4-5",
                    "tier": 1
                },
                {
                    "modelIntent": "claude-opus-5-thinking-max",
                    "inputTokens": "292",
                    "outputTokens": "101115",
                    "cacheWriteTokens": "940745",
                    "cacheReadTokens": "30088519",
                    "tier": 1
                },
                {
                    "modelIntent": "gpt-5.4-mini",
                    "tier": 1
                }
            ]
        });

        let sand_json = serde_json::json!({
            "usagePercent": 0.0,
            "nextResetTimestampUtc": "2026-09-20T00:00:00Z"
        });

        let mut snapshot = snapshot_from_json(&summary_json).unwrap();
        append_grok_bot(&mut snapshot, &sand_json);
        append_aggregated_usage(&mut snapshot, &agg_json);
        snapshot.availability = snapshot.compute_availability();

        // 1. Quotas verification
        assert_eq!(snapshot.quotas[0].id, "cursor_models");
        assert_eq!(snapshot.quotas[0].label, "Cursor Models");
        assert_eq!(snapshot.quotas[0].used_percent, Some(38.2711));

        assert_eq!(snapshot.quotas[1].id, "other_models");
        assert_eq!(snapshot.quotas[1].label, "Other Models");
        assert_eq!(snapshot.quotas[1].used_percent, Some(100.0));

        assert_eq!(snapshot.quotas[2].id, "total");
        assert_eq!(snapshot.quotas[2].label, "Uso incluido total");
        assert_eq!(snapshot.quotas[2].used_percent, Some(44.0949));

        assert_eq!(snapshot.quotas[3].id, "grok_bot");
        assert_eq!(snapshot.quotas[3].label, "Grok Bot · semanal");

        // 2. Primary utilization must be cursor_models (38.2711), NOT total (44.0949)
        assert_eq!(snapshot.primary_utilization, Some(38.2711));

        // 3. Status is Connected, availability is PartialLimited (because other_models == 100, cursor_models < 100)
        assert_eq!(snapshot.status, ProviderStatus::Connected);
        assert_eq!(snapshot.availability, Availability::PartialLimited);

        // 4. Product breakdown verification
        assert_eq!(snapshot.product_breakdown.len(), 6);
        // Cursor Models group
        assert_eq!(snapshot.product_breakdown[0].name, "auto");
        assert_eq!(snapshot.product_breakdown[0].used_percent, 20.3); // 9168.71588 / 17268.50201 * 38.2711 = 20.32
        assert_eq!(snapshot.product_breakdown[0].parent_quota_id, Some("cursor_models".into()));

        assert_eq!(snapshot.product_breakdown[1].name, "cursor-grok-4.6-high-fast");
        assert_eq!(snapshot.product_breakdown[1].used_percent, 11.8); // 5327.11363 / 17268.50201 * 38.2711 = 11.80
        assert_eq!(snapshot.product_breakdown[1].parent_quota_id, Some("cursor_models".into()));

        assert_eq!(snapshot.product_breakdown[2].name, "cursor-grok-4.6-high");
        assert_eq!(snapshot.product_breakdown[2].used_percent, 3.4); // 1513.0719 / 17268.50201 * 38.2711 = 3.35 -> 3.4
        assert_eq!(snapshot.product_breakdown[2].parent_quota_id, Some("cursor_models".into()));

        assert_eq!(snapshot.product_breakdown[3].name, "composer-2.5-fast");
        assert_eq!(snapshot.product_breakdown[3].used_percent, 2.8); // 1259.6006 / 17268.50201 * 38.2711 = 2.79 -> 2.8
        assert_eq!(snapshot.product_breakdown[3].parent_quota_id, Some("cursor_models".into()));

        // Other Models group
        assert_eq!(snapshot.product_breakdown[4].name, "claude-opus-5-thinking-high");
        assert_eq!(snapshot.product_breakdown[4].used_percent, 100.0);
        assert_eq!(snapshot.product_breakdown[4].parent_quota_id, Some("other_models".into()));

        assert_eq!(snapshot.product_breakdown[5].name, "claude-opus-5-thinking-max");
        assert_eq!(snapshot.product_breakdown[5].used_percent, 0.0);
        assert_eq!(snapshot.product_breakdown[5].parent_quota_id, Some("other_models".into()));
    }
}
