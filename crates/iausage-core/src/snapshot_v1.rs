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
    UsageCost, UsageQuota, UsageSource, VendorId, VendorInfo,
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
}

pub fn parse_id(s: &str) -> Option<VendorId> {
    VendorId::all().iter().copied().find(|id| id.slug() == s)
}

pub fn build(
    snapshots: &HashMap<String, ProviderSnapshot>,
    catalog: &[VendorInfo],
    generated_at: String,
    app_version: Option<String>,
) -> DashboardSnapshotV1 {
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
                quotas: snap.map(|s| s.quotas.clone()).unwrap_or_default(),
                credits: snap.and_then(|s| s.credits.clone()),
                cost: snap.and_then(|s| s.cost.clone()),
                accounts: snap
                    .map(|_| vec![AccountV1 { id: "default".into() }])
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
            vec![progress_pct("session", "Sesión", 10.0, None, 18_000, "always")],
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
}
