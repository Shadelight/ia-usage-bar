//! OpenAI Admin API — costos de organización (CodexBar).

use chrono::{Datelike, Utc};
use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    json_f64, snapshot_needs_auth, snapshot_ok, values_line, ProviderSnapshot, VendorId,
};

use super::Provider;

pub struct OpenaiAdmin;

impl Provider for OpenaiAdmin {
    fn id(&self) -> VendorId {
        VendorId::OpenaiAdmin
    }

    fn has_local_credentials(&self, cfg: &AppConfig) -> bool {
        cfg.api_key(VendorId::OpenaiAdmin)
            .filter(|s| !s.trim().is_empty())
            .is_some()
    }

    fn refresh(&self, cfg: &AppConfig) -> ProviderSnapshot {
        let key = match cfg.api_key(VendorId::OpenaiAdmin) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                return snapshot_needs_auth(
                    VendorId::OpenaiAdmin,
                    VendorId::OpenaiAdmin.login_hint(),
                )
            }
        };
        match fetch_costs(&key) {
            Ok(snap) => snap,
            // A permission response is authoritative. Retrying a legacy
            // endpoint with the same key hides the actionable api.usage.read
            // diagnosis behind a second, unrelated failure.
            Err(error) if should_try_legacy(&error) => match fetch_legacy_grants(&key) {
                Ok(snap) => snap,
                Err(_) => super::map_fetch_err(VendorId::OpenaiAdmin, error),
            },
            Err(error) => super::map_fetch_err(VendorId::OpenaiAdmin, error),
        }
    }
}

fn should_try_legacy(error: &FetchError) -> bool {
    matches!(error, FetchError::Http(404 | 410, _))
}

fn fetch_costs(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let now = Utc::now();
    let start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc().timestamp())
        .unwrap_or(0);
    let url =
        format!("https://api.openai.com/v1/organization/costs?start_time={start}&bucket_width=1d");
    let body = http::get_json(
        &url,
        &[
            ("Authorization", &format!("Bearer {key}")),
            ("Content-Type", "application/json"),
        ],
    )?;
    let mut total = 0.0;
    if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
        for bucket in arr {
            if let Some(results) = bucket.get("results").and_then(|v| v.as_array()) {
                for row in results {
                    total += json_f64(row, &["amount", "value"]).unwrap_or(0.0);
                    if let Some(amt) = row.get("amount") {
                        total += json_f64(amt, &["value"]).unwrap_or(0.0);
                    }
                }
            } else {
                total += json_f64(bucket, &["amount", "cost"]).unwrap_or(0.0);
            }
        }
    }
    Ok(snapshot_ok(
        VendorId::OpenaiAdmin,
        "Admin API",
        vec![values_line(
            "mtd",
            "Gasto del mes",
            &format!("${total:.2}"),
            "always",
        )],
    ))
}

fn fetch_legacy_grants(key: &str) -> Result<ProviderSnapshot, FetchError> {
    let body = http::get_json(
        "https://api.openai.com/v1/dashboard/billing/credit_grants",
        &[("Authorization", &format!("Bearer {key}"))],
    )?;
    let remaining = pick_money(&body, &["total_available", "totalAvailable"]);
    let used = pick_money(&body, &["total_used", "totalUsed"]);
    let mut lines = Vec::new();
    if let Some(r) = remaining {
        lines.push(values_line(
            "available",
            "Disponible",
            &format!("${r:.2}"),
            "always",
        ));
    }
    if let Some(u) = used {
        lines.push(values_line("used", "Usado", &format!("${u:.2}"), "always"));
    }
    if lines.is_empty() {
        return Err(FetchError::Parse("OpenAI: sin datos de créditos".into()));
    }
    Ok(snapshot_ok(VendorId::OpenaiAdmin, "API", lines))
}

fn pick_money(v: &Value, keys: &[&str]) -> Option<f64> {
    json_f64(v, keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_removed_or_missing_cost_endpoints_use_legacy_fallback() {
        assert!(should_try_legacy(&FetchError::Http(404, String::new())));
        assert!(should_try_legacy(&FetchError::Http(410, String::new())));
        assert!(!should_try_legacy(&FetchError::Http(403, String::new())));
        assert!(!should_try_legacy(&FetchError::Http(401, String::new())));
    }
}
