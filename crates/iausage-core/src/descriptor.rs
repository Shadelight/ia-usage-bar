//! `ProviderDescriptor`: fuente única de metadata por proveedor.
//!
//! Añadir un proveedor es `descriptor + strategies + parser + fixtures +
//! docs/providers/<slug>.md`. La GUI y el CLI consumen este mismo pipeline;
//! el frontend nunca ramifica por `if provider == "claude"`, solo lee
//! `capabilities / links / source / actions`.

use serde::{Deserialize, Serialize};

use crate::model::VendorId;

/// Estrategia de obtención de datos. Un proveedor declara varias en orden de
/// preferencia; `Auto` las prueba en orden y recuerda la activa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FetchStrategyKind {
    Oauth,
    Cli,
    Api,
    Web,
    Local,
}

impl FetchStrategyKind {
    pub fn label(self) -> &'static str {
        match self {
            FetchStrategyKind::Oauth => "OAuth",
            FetchStrategyKind::Cli => "CLI",
            FetchStrategyKind::Api => "API",
            FetchStrategyKind::Web => "Web",
            FetchStrategyKind::Local => "Local",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            FetchStrategyKind::Oauth => "oauth",
            FetchStrategyKind::Cli => "cli",
            FetchStrategyKind::Api => "api",
            FetchStrategyKind::Web => "web",
            FetchStrategyKind::Local => "local",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "oauth" => Some(FetchStrategyKind::Oauth),
            "cli" => Some(FetchStrategyKind::Cli),
            "api" => Some(FetchStrategyKind::Api),
            "web" | "web-session" | "websession" => Some(FetchStrategyKind::Web),
            "local" | "local-session" | "localsession" => Some(FetchStrategyKind::Local),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub quotas: bool,
    pub credits: bool,
    pub cost: bool,
    pub history: bool,
    pub service_status: bool,
    pub multiple_accounts: bool,
    pub dashboard: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderDescriptor {
    pub id: VendorId,
    /// Estrategias en orden de preferencia para `Auto`.
    pub strategies: &'static [FetchStrategyKind],
    pub default_strategy: FetchStrategyKind,
    pub capabilities: ProviderCapabilities,
}

const Q: ProviderCapabilities = ProviderCapabilities {
    quotas: true,
    credits: false,
    cost: false,
    history: false,
    service_status: false,
    multiple_accounts: false,
    dashboard: false,
};

macro_rules! desc {
    ($id:expr, $strategies:expr, $default:expr, $caps:expr) => {
        ProviderDescriptor {
            id: $id,
            strategies: $strategies,
            default_strategy: $default,
            capabilities: $caps,
        }
    };
}

use FetchStrategyKind::{Api, Cli, Local, Oauth};

/// Tabla curada a mano desde lo que cada fetcher ejecuta realmente
/// (ver `providers/*`). Una estrategia aquí es una promesa de ejecución, no
/// una fuente que el producto podría llegar a soportar. `service_status` y
/// `dashboard` derivan de que existan URLs oficiales verificadas
/// (`VendorId::links`), nunca adivinadas.
pub static DESCRIPTORS: &[ProviderDescriptor] = &[
    desc!(
        VendorId::Anthropic,
        &[Oauth],
        Oauth,
        ProviderCapabilities {
            quotas: true,
            cost: true,
            service_status: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(
        VendorId::AnthropicApi,
        &[Api],
        Api,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            cost: true,
            service_status: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(
        VendorId::Openai,
        &[Oauth],
        Oauth,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            service_status: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(
        VendorId::OpenaiAdmin,
        &[Api],
        Api,
        ProviderCapabilities {
            cost: true,
            service_status: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(
        VendorId::Copilot,
        &[Cli],
        Cli,
        ProviderCapabilities {
            quotas: true,
            service_status: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(VendorId::Zai, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        dashboard: true,
        ..Q
    }),
    desc!(VendorId::Openrouter, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        dashboard: true,
        ..Q
    }),
    desc!(VendorId::Deepseek, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        dashboard: true,
        ..Q
    }),
    desc!(
        VendorId::Kimi,
        &[Api],
        Api,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(VendorId::Kilo, &[Api], Api, Q),
    desc!(VendorId::Novita, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        dashboard: true,
        ..Q
    }),
    desc!(VendorId::Moonshot, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        dashboard: true,
        ..Q
    }),
    desc!(
        VendorId::Grok,
        &[Api],
        Api,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            cost: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(
        VendorId::Supergrok,
        &[Local],
        Local,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(VendorId::Antigravity, &[Local], Local, Q),
    desc!(
        VendorId::Cursor,
        &[Local],
        Local,
        ProviderCapabilities {
            quotas: true,
            dashboard: true,
            ..Q
        }
    ),
    desc!(VendorId::Minimax, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        dashboard: true,
        ..Q
    }),
    desc!(
        VendorId::Kiro,
        &[Local],
        Local,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            ..Q
        }
    ),
    desc!(
        VendorId::Nous,
        &[Oauth],
        Oauth,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            ..Q
        }
    ),
    desc!(VendorId::OpenCodeGo, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        ..Q
    }),
    desc!(
        VendorId::CommandCode,
        &[Local],
        Local,
        ProviderCapabilities {
            quotas: true,
            credits: true,
            ..Q
        }
    ),
    desc!(VendorId::Groq, &[Api], Api, ProviderCapabilities {
        quotas: true,
        credits: true,
        cost: true,
        service_status: true,
        dashboard: true,
        ..Q
    }),
    desc!(
        VendorId::Windsurf,
        &[Local],
        Local,
        ProviderCapabilities {
            quotas: true,
            service_status: true,
            dashboard: true,
            ..Q
        }
    ),
];

/// Resuelve la estrategia efectiva: preferencia explícita o `default`.
/// Las preferencias inválidas de versiones antiguas caen al valor implementado
/// para que una configuración guardada no pueda falsear la fuente activa.
pub fn resolve_strategy(
    descriptor: &ProviderDescriptor,
    preferred: Option<FetchStrategyKind>,
) -> FetchStrategyKind {
    match preferred {
        Some(p) if descriptor.strategies.contains(&p) => p,
        _ => descriptor.default_strategy,
    }
}

pub fn descriptor(id: VendorId) -> &'static ProviderDescriptor {
    DESCRIPTORS
        .iter()
        .find(|d| d.id == id)
        .expect("every VendorId must have a descriptor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_vendor_has_exactly_one_descriptor() {
        let mut seen = HashSet::new();
        for d in DESCRIPTORS {
            assert!(seen.insert(d.id.slug()), "duplicate {}", d.id.slug());
        }
        for id in VendorId::all() {
            assert!(seen.contains(id.slug()), "missing {}", id.slug());
        }
    }

    #[test]
    fn default_strategy_is_always_declared() {
        for d in DESCRIPTORS {
            assert!(
                d.strategies.contains(&d.default_strategy),
                "{} default not in strategies",
                d.id.slug()
            );
            assert!(!d.strategies.is_empty());
        }
    }

    #[test]
    fn service_and_dashboard_flags_match_verified_links() {
        for d in DESCRIPTORS {
            let links = d.id.links();
            if d.capabilities.service_status {
                assert!(
                    links.status_url.is_some(),
                    "{} claims service_status without a verified status URL",
                    d.id.slug()
                );
            }
            if d.capabilities.dashboard {
                assert!(
                    links.usage_url.is_some(),
                    "{} claims dashboard without a verified usage URL",
                    d.id.slug()
                );
            }
        }
    }

    #[test]
    fn strategy_parse_round_trips() {
        for s in [Oauth, Cli, Api, FetchStrategyKind::Web, Local] {
            assert_eq!(FetchStrategyKind::parse(s.slug()), Some(s));
        }
        assert_eq!(FetchStrategyKind::parse("auto"), None);
        assert_eq!(FetchStrategyKind::parse("bogus"), None);
    }

    #[test]
    fn resolve_prefers_explicit_when_supported() {
        let claude = descriptor(VendorId::Anthropic);
        assert_eq!(resolve_strategy(claude, None), Oauth);
        assert_eq!(resolve_strategy(claude, Some(FetchStrategyKind::Web)), Oauth);
        // Kimi no declara Web: cae al default.
        let kimi = descriptor(VendorId::Kimi);
        assert_eq!(resolve_strategy(kimi, Some(FetchStrategyKind::Web)), Api);
    }
}
