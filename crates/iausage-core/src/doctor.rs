//! `iausage doctor`: diagnóstico estructurado por proveedor.
//!
//! Checks baratos y sin efectos secundarios sobre estado ya cargado
//! (config + caché). El probing de red en vivo es trabajo de `refresh`;
//! `doctor` dice qué falta, no lo reintenta.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::model::{ProviderSnapshot, VendorId};
use crate::providers;
use crate::snapshot_v1::parse_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub code: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDiagnosis {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

fn check(code: &str, ok: bool, detail: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        code: code.into(),
        ok,
        detail: detail.into(),
    }
}

pub fn diagnose_provider(
    id: VendorId,
    cfg: &AppConfig,
    snapshots: &HashMap<String, ProviderSnapshot>,
) -> ProviderDiagnosis {
    let enabled = cfg.is_enabled(id);
    let mut checks = Vec::new();

    checks.push(check(
        "enabled",
        enabled,
        if enabled {
            "provider activo"
        } else {
            "provider desactivado (no se consulta)"
        },
    ));

    let has_cred = providers::has_local_credentials(id, cfg);
    checks.push(check(
        "credential",
        has_cred,
        if has_cred {
            "credencial detectable (keyring/env/login local)"
        } else {
            id.login_hint()
        },
    ));

    match snapshots.get(id.slug()) {
        None => checks.push(check("data", false, "sin snapshot en caché todavía")),
        Some(snap) => {
            checks.push(check(
                "connection",
                snap.is_connected(),
                match (snap.status, snap.status_reason) {
                    (crate::model::ProviderStatus::Connected, _) => "conectado",
                    (s, r) => {
                        // Préstamo corto: el detalle se construye abajo.
                        let _ = (s, r);
                        "ver status/statusReason"
                    }
                },
            ));
            if snap.stale {
                checks.push(check(
                    "freshness",
                    true,
                    "datos antiguos (stale-while-refresh)",
                ));
            }
        }
    }

    let ok = checks.iter().all(|c| c.ok) && enabled;
    ProviderDiagnosis {
        id: id.slug().into(),
        name: id.display_name().into(),
        enabled,
        ok,
        checks,
    }
}

pub fn diagnose_all(
    cfg: &AppConfig,
    snapshots: &HashMap<String, ProviderSnapshot>,
) -> Vec<ProviderDiagnosis> {
    VendorId::all()
        .iter()
        .copied()
        .map(|id| diagnose_provider(id, cfg, snapshots))
        .collect()
}

pub fn diagnose_one(
    slug: &str,
    cfg: &AppConfig,
    snapshots: &HashMap<String, ProviderSnapshot>,
) -> Option<ProviderDiagnosis> {
    parse_id(slug).map(|id| diagnose_provider(id, cfg, snapshots))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_provider_reports_not_ok_with_hint() {
        let cfg = AppConfig::default();
        let diags = diagnose_all(&cfg, &HashMap::new());
        assert_eq!(diags.len(), VendorId::all().len());
        let claude = diags.iter().find(|d| d.id == "anthropic").unwrap();
        assert!(!claude.enabled);
        assert!(!claude.ok);
        assert!(claude.checks.iter().any(|c| c.code == "credential"));
    }

    #[test]
    fn unknown_slug_returns_none() {
        let cfg = AppConfig::default();
        assert!(diagnose_one("nope", &cfg, &HashMap::new()).is_none());
    }
}
