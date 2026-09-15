//! `DashboardSnapshotV1`: contrato estable de salida.
//!
//! La GUI, el CLI `--json`, la futura API y los widgets comparten este
//! modelo versionado. **Nunca** se expone `ProviderSnapshot` interno
//! directamente: cada refactor interno que rompiera scripts externos sería
//! un bug de compatibilidad.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::descriptor::FetchStrategyKind;
use crate::health;
use crate::model::{
    CreditsSummary, ProviderSnapshot, ProviderStatus, ProviderStatusReason, ServiceHealth,
    UsageCost, UsageQuota, UsageSource, VendorId, VendorInfo, WindowType,
};
use crate::SNAPSHOT_SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionV1 {
    pub status: ProviderStatus,
    pub reason: Option<ProviderStatusReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceV1 {
    pub status: ServiceHealth,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountV1 {
    /// Hoy siempre `"default"`: reserva la forma multi-account sin romper nada.
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderV1 {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub source: Option<UsageSource>,
    pub preferred_source: Option<FetchStrategyKind>,
    pub connection: Option<ConnectionV1>,
    pub service: ServiceV1,
    pub stale: bool,
    pub quotas: Vec<UsageQuota>,
    pub credits: Option<CreditsSummary>,
    pub cost: Option<UsageCost>,
    pub accounts: Vec<AccountV1>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshotV1 {
    pub schema_version: u32,
    pub generated_at: String,
    pub app_version: Option<String>,
    pub providers: Vec<ProviderV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<crate::recommend::Recommendation>,
}

pub fn parse_id(s: &str) -> Option<VendorId> {
    VendorId::all().iter().copied().find(|id| id.slug() == s)
}

fn window_type_key(window_type: WindowType) -> &'static str {
    match window_type {
        WindowType::Session => "session",
        WindowType::FiveHour => "5h",
        WindowType::Daily => "daily",
        WindowType::Weekly => "weekly",
        WindowType::Monthly => "monthly",
        WindowType::Credits | WindowType::Custom => "custom",
    }
}

fn quota_with_pace(mut quota: UsageQuota, now_unix: Option<i64>) -> UsageQuota {
    // Never carry an old derived value through a cache read. A public snapshot
    // is the sole place where pace is recomputed against its generatedAt.
    quota.pace = None;
    let Some(now_unix) = now_unix else {
        return quota;
    };
    if quota.stale {
        return quota;
    }
    let Some(used_percent) = quota.used_percent else {
        return quota;
    };
    let Some(reset_unix) = quota
        .reset_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp())
        .filter(|reset| *reset > now_unix)
    else {
        return quota;
    };
    let Some(window_secs) = crate::pace::window_secs_for(window_type_key(quota.window_type)) else {
        return quota;
    };
    quota.pace = crate::pace::compute_pace(used_percent, window_secs, reset_unix, now_unix);
    quota
}

pub fn build(
    snapshots: &HashMap<String, ProviderSnapshot>,
    catalog: &[VendorInfo],
    generated_at: String,
    app_version: Option<String>,
) -> DashboardSnapshotV1 {
    build_for_primary(snapshots, catalog, generated_at, app_version, None)
}

pub fn build_for_primary(
    snapshots: &HashMap<String, ProviderSnapshot>,
    catalog: &[VendorInfo],
    generated_at: String,
    app_version: Option<String>,
    primary: Option<&str>,
) -> DashboardSnapshotV1 {
    let generated_at_unix = chrono::DateTime::parse_from_rfc3339(&generated_at)
        .ok()
        .map(|value| value.timestamp());
    let recommendation = primary.and_then(|primary_id| {
        generated_at_unix.map(|now| {
            let enabled: Vec<ProviderSnapshot> = catalog
                .iter()
                .filter(|vendor| vendor.enabled)
                .filter_map(|vendor| snapshots.get(&vendor.id).cloned())
                .collect();
            crate::recommend::recommend(&enabled, primary_id, now)
        })
    });
    let providers = catalog
        .iter()
        .map(|vendor| {
            let snap = snapshots.get(&vendor.id);
            let vid = parse_id(&vendor.id);
            ProviderV1 {
                id: vendor.id.clone(),
                name: vendor.name.clone(),
                enabled: vendor.enabled,
                source: snap.and_then(|s| s.active_source),
                preferred_source: vendor
                    .source_preference
                    .as_deref()
                    .and_then(FetchStrategyKind::parse),
                connection: snap.map(|s| ConnectionV1 {
                    status: s.status,
                    reason: s.status_reason,
                }),
                service: ServiceV1 {
                    status: snap.map(|s| s.service).unwrap_or_default(),
                    url: vid.and_then(health::status_url),
                },
                stale: snap.map(|s| s.stale).unwrap_or(false),
                quotas: snap
                    .map(|s| {
                        s.quotas
                            .iter()
                            .cloned()
                            .map(|quota| quota_with_pace(quota, generated_at_unix))
                            .collect()
                    })
                    .unwrap_or_default(),
                credits: snap.and_then(|s| s.credits.clone()),
                cost: snap.and_then(|s| s.cost.clone()),
                accounts: snap
                    .map(|_| {
                        vec![AccountV1 {
                            id: "default".into(),
                        }]
                    })
                    .unwrap_or_default(),
                updated_at: snap.map(|s| s.updated_at.clone()),
            }
        })
        .collect();
    DashboardSnapshotV1 {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        generated_at,
        app_version,
        providers,
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok};

    fn catalog() -> Vec<VendorInfo> {
        vec![VendorInfo {
            id: "anthropic".into(),
            name: "Claude Code".into(),
            short: "CLD".into(),
            auth_kind: crate::model::AuthKind::Oauth,
            env_key: None,
            hint: String::new(),
            needs_key: false,
            enabled: true,
            detected: true,
            has_credential: true,
            credential_source: None,
            links: VendorId::Anthropic.links(),
            strategies: vec!["oauth".into()],
            source_preference: None,
        }]
    }

    #[test]
    fn v1_keeps_schema_version_and_single_default_account() {
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session", "Sesión", 10.0, None, 18_000, "always",
            )],
        );
        let snaps = HashMap::from([(snap.id.clone(), snap)]);
        let v1 = build(&snaps, &catalog(), "now".into(), Some("0.3.0".into()));
        assert_eq!(v1.schema_version, 1);
        assert_eq!(v1.providers.len(), 1);
        let p = &v1.providers[0];
        assert_eq!(p.accounts.len(), 1);
        assert_eq!(p.source, Some(UsageSource::Oauth));
        assert!(p.service.url.is_some());
        // Contrato JSON: claves estables.
        let json = serde_json::to_value(&v1).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert!(json["providers"][0].get("quotas").is_some());
    }

    #[test]
    fn vendor_without_data_has_empty_connection() {
        let v1 = build(&HashMap::new(), &catalog(), "now".into(), None);
        let p = &v1.providers[0];
        assert!(p.connection.is_none());
        assert!(p.quotas.is_empty());
        assert!(p.accounts.is_empty());
    }

    #[test]
    fn v1_adds_pace_only_when_the_window_is_fresh_and_projectable() {
        let reset = "2026-09-14T15:00:00Z".to_string();
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session",
                "Sesión",
                88.0,
                Some(reset),
                18_000,
                "always",
            )],
        );
        let snaps = HashMap::from([(snap.id.clone(), snap)]);
        // 72% de la ventana transcurrida: 88% no alcanzará el reset.
        let v1 = build(&snaps, &catalog(), "2026-09-14T13:36:00Z".into(), None);
        let quota = &v1.providers[0].quotas[0];
        let pace = quota.pace.as_ref().expect("pace should be present");
        assert_eq!(pace.will_last_to_reset, Some(false));
        assert!(pace.estimated_exhausted_at.is_some());

        let json = serde_json::to_value(&v1).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert!(json["providers"][0]["quotas"][0]["pace"].is_object());
        assert_eq!(json["providers"][0]["quotas"][0]["windowType"], "session");
    }

    #[test]
    fn v1_omits_pace_for_stale_or_unusable_windows() {
        let mut snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "session",
                "Sesión",
                88.0,
                Some("2026-09-14T15:00:00Z".into()),
                18_000,
                "always",
            )],
        );
        snap.mark_stale();
        let snaps = HashMap::from([(snap.id.clone(), snap)]);
        let v1 = build(&snaps, &catalog(), "2026-09-14T13:36:00Z".into(), None);
        assert!(v1.providers[0].quotas[0].pace.is_none());
        let json = serde_json::to_value(&v1).unwrap();
        assert!(json["providers"][0]["quotas"][0].get("pace").is_none());
    }

    #[test]
    fn v1_exposes_the_primary_providers_limiting_quota() {
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![progress_pct(
                "weekly",
                "Semanal",
                99.0,
                Some("2026-09-16T00:00:00Z".into()),
                604_800,
                "always",
            )],
        );
        let snaps = HashMap::from([(snap.id.clone(), snap)]);
        let v1 = build_for_primary(
            &snaps,
            &catalog(),
            "2026-09-14T13:00:00Z".into(),
            None,
            Some("anthropic"),
        );
        let recommendation = v1.recommendation.expect("recommendation");
        assert_eq!(recommendation.severity, "critical");
        assert_eq!(recommendation.reason, "quota_near_exhaustion");
        assert_eq!(recommendation.limiting_quota.expect("quota").id, "weekly");
    }
}
