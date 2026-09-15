//! Tipos que se serializan al frontend (camelCase).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VendorId {
    Anthropic,
    AnthropicApi,
    Openai,
    OpenaiAdmin,
    Copilot,
    Zai,
    Openrouter,
    Deepseek,
    Kimi,
    Kilo,
    Novita,
    Moonshot,
    Grok,
    Supergrok,
    Antigravity,
    Cursor,
    Minimax,
    Kiro,
    Nous,
    #[serde(rename = "opencode_go")]
    OpenCodeGo,
    CommandCode,
    Groq,
    Windsurf,
}

impl VendorId {
    pub fn slug(self) -> &'static str {
        match self {
            VendorId::Anthropic => "anthropic",
            VendorId::AnthropicApi => "anthropic_api",
            VendorId::Openai => "openai",
            VendorId::OpenaiAdmin => "openai_admin",
            VendorId::Copilot => "copilot",
            VendorId::Zai => "zai",
            VendorId::Openrouter => "openrouter",
            VendorId::Deepseek => "deepseek",
            VendorId::Kimi => "kimi",
            VendorId::Kilo => "kilo",
            VendorId::Novita => "novita",
            VendorId::Moonshot => "moonshot",
            VendorId::Grok => "grok",
            VendorId::Supergrok => "supergrok",
            VendorId::Antigravity => "antigravity",
            VendorId::Cursor => "cursor",
            VendorId::Minimax => "minimax",
            VendorId::Kiro => "kiro",
            VendorId::Nous => "nous",
            VendorId::OpenCodeGo => "opencode_go",
            VendorId::CommandCode => "commandcode",
            VendorId::Groq => "groq",
            VendorId::Windsurf => "windsurf",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            VendorId::Anthropic => "Claude Code",
            VendorId::AnthropicApi => "Anthropic API",
            VendorId::Openai => "Codex / ChatGPT",
            VendorId::OpenaiAdmin => "OpenAI API · Admin",
            VendorId::Copilot => "GitHub Copilot",
            VendorId::Zai => "Z.AI / GLM",
            VendorId::Openrouter => "OpenRouter",
            VendorId::Deepseek => "DeepSeek",
            VendorId::Kimi => "Kimi",
            VendorId::Kilo => "Kilo",
            VendorId::Novita => "Novita",
            VendorId::Moonshot => "Moonshot",
            VendorId::Grok => "Grok (xAI)",
            VendorId::Supergrok => "SuperGrok",
            VendorId::Antigravity => "Antigravity",
            VendorId::Cursor => "Cursor",
            VendorId::Minimax => "MiniMax",
            VendorId::Kiro => "Kiro",
            VendorId::Nous => "Nous Research",
            VendorId::OpenCodeGo => "OpenCode Go",
            VendorId::CommandCode => "Command Code",
            VendorId::Groq => "Groq",
            VendorId::Windsurf => "Windsurf",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            VendorId::Anthropic => "CLD",
            VendorId::AnthropicApi => "ANT",
            VendorId::Openai => "CDX",
            VendorId::OpenaiAdmin => "OAI",
            VendorId::Copilot => "COP",
            VendorId::Zai => "ZAI",
            VendorId::Openrouter => "OR",
            VendorId::Deepseek => "DSK",
            VendorId::Kimi => "KMI",
            VendorId::Kilo => "KLO",
            VendorId::Novita => "NOV",
            VendorId::Moonshot => "MSH",
            VendorId::Grok => "GRK",
            VendorId::Supergrok => "SGK",
            VendorId::Antigravity => "AGY",
            VendorId::Cursor => "CUR",
            VendorId::Minimax => "MMX",
            VendorId::Kiro => "KRO",
            VendorId::Nous => "NUS",
            VendorId::OpenCodeGo => "OCG",
            VendorId::CommandCode => "CMD",
            VendorId::Groq => "GRQ",
            VendorId::Windsurf => "WND",
        }
    }

    pub fn all() -> &'static [VendorId] {
        &[
            VendorId::Anthropic,
            VendorId::Openai,
            VendorId::Cursor,
            VendorId::Antigravity,
            VendorId::OpenaiAdmin,
            VendorId::Copilot,
            VendorId::Openrouter,
            VendorId::Zai,
            VendorId::Deepseek,
            VendorId::Grok,
            VendorId::Supergrok,
            VendorId::Kimi,
            VendorId::Kilo,
            VendorId::Novita,
            VendorId::Moonshot,
            VendorId::Minimax,
            VendorId::Kiro,
            VendorId::Nous,
            VendorId::OpenCodeGo,
            VendorId::CommandCode,
            VendorId::AnthropicApi,
            VendorId::Groq,
            VendorId::Windsurf,
        ]
    }

    pub fn env_key(self) -> Option<&'static str> {
        match self {
            VendorId::Zai => Some("ZAI_API_KEY"),
            VendorId::Openrouter => Some("OPENROUTER_API_KEY"),
            VendorId::Deepseek => Some("DEEPSEEK_API_KEY"),
            VendorId::Kimi => Some("KIMI_API_KEY"),
            VendorId::Kilo => Some("KILO_API_KEY"),
            VendorId::Novita => Some("NOVITA_API_KEY"),
            VendorId::Moonshot => Some("MOONSHOT_API_KEY"),
            VendorId::Grok => Some("XAI_MANAGEMENT_KEY"),
            VendorId::Minimax => Some("MINIMAX_API_KEY"),
            VendorId::AnthropicApi => Some("ANTHROPIC_ADMIN_KEY"),
            VendorId::OpenaiAdmin => Some("OPENAI_ADMIN_KEY"),
            VendorId::OpenCodeGo => Some("OPENCODE_GO_API_KEY"),
            VendorId::CommandCode => Some("COMMANDCODE_API_KEY"),
            VendorId::Copilot => Some("GITHUB_COPILOT_TOKEN"),
            VendorId::Groq => Some("GROQ_API_KEY"),
            _ => None,
        }
    }

    pub fn auth_kind(self) -> AuthKind {
        match self {
            VendorId::Anthropic | VendorId::Openai | VendorId::Nous => AuthKind::Oauth,
            VendorId::Cursor
            | VendorId::Antigravity
            | VendorId::Kiro
            | VendorId::Supergrok
            | VendorId::CommandCode
            | VendorId::Windsurf => AuthKind::Local,
            VendorId::Copilot | VendorId::Kimi => AuthKind::Mixed,
            _ => AuthKind::ApiKey,
        }
    }

    pub fn login_hint(self) -> &'static str {
        match self {
            VendorId::Anthropic => "Ejecuta `claude` e inicia sesión.",
            VendorId::Openai => "Ejecuta `codex login`.",
            VendorId::Cursor => "Abre Cursor e inicia sesión.",
            VendorId::Antigravity => "Abre Antigravity e inicia sesión.",
            VendorId::Copilot => "Ejecuta `gh auth login --web` o define GITHUB_COPILOT_TOKEN.",
            VendorId::OpenaiAdmin => {
                "Pega una Admin API key de platform.openai.com (mide gasto de la organización)."
            }
            VendorId::Kiro => "Ejecuta `kiro-cli login`.",
            VendorId::Supergrok => "Ejecuta `grok login` (en Windows la CLI está en %USERPROFILE%\\.grok\\bin\\grok.exe).",
            VendorId::CommandCode => "Inicia sesión con `commandcode` o pi.",
            VendorId::Nous => "Guarda las credenciales OAuth de Nous.",
            VendorId::Kimi => "Define KIMI_API_KEY o inicia sesión con `kimi`.",
            VendorId::Groq => "Pega GROQ_API_KEY en Ajustes.",
            VendorId::Windsurf => "Abre Windsurf e inicia sesión.",
            _ => "Pega una API key en Ajustes o define la variable de entorno.",
        }
    }

    pub fn needs_api_key_ui(self) -> bool {
        matches!(self.auth_kind(), AuthKind::ApiKey | AuthKind::Mixed) && self.env_key().is_some()
    }

    /// Only well-known, verifiable official URLs. Every provider not listed
    /// here gets `None` for all fields — never a guessed link.
    pub fn links(self) -> VendorLinks {
        match self {
            VendorId::Anthropic => VendorLinks {
                usage_url: Some("https://claude.ai/settings/usage".into()),
                billing_url: Some("https://claude.ai/settings/billing".into()),
                status_url: Some("https://status.anthropic.com".into()),
                docs_url: Some(
                    "https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/overview"
                        .into(),
                ),
                app_url: Some("https://claude.ai".into()),
                api_key_url: None,
                signup_url: None,
            },
            VendorId::AnthropicApi => VendorLinks {
                usage_url: Some("https://console.anthropic.com/settings/usage".into()),
                billing_url: Some("https://console.anthropic.com/settings/plans".into()),
                status_url: Some("https://status.anthropic.com".into()),
                docs_url: Some("https://docs.anthropic.com".into()),
                app_url: Some("https://console.anthropic.com".into()),
                api_key_url: Some("https://console.anthropic.com/settings/keys".into()),
                signup_url: Some("https://console.anthropic.com/".into()),
            },
            VendorId::Openai => VendorLinks {
                usage_url: Some("https://chatgpt.com/#settings".into()),
                billing_url: Some("https://chatgpt.com/#settings".into()),
                status_url: Some("https://status.openai.com".into()),
                docs_url: Some("https://help.openai.com".into()),
                app_url: Some("https://chatgpt.com".into()),
                api_key_url: None,
                signup_url: None,
            },
            VendorId::OpenaiAdmin => VendorLinks {
                usage_url: Some("https://platform.openai.com/usage".into()),
                billing_url: Some(
                    "https://platform.openai.com/settings/organization/billing".into(),
                ),
                status_url: Some("https://status.openai.com".into()),
                docs_url: Some("https://platform.openai.com/docs".into()),
                app_url: Some("https://platform.openai.com".into()),
                api_key_url: Some("https://platform.openai.com/api-keys".into()),
                signup_url: Some("https://platform.openai.com/signup".into()),
            },
            VendorId::Copilot => VendorLinks {
                usage_url: Some("https://github.com/settings/copilot".into()),
                billing_url: Some("https://github.com/settings/billing".into()),
                status_url: Some("https://www.githubstatus.com".into()),
                docs_url: Some("https://docs.github.com/copilot".into()),
                app_url: Some("https://github.com/copilot".into()),
                api_key_url: Some("https://github.com/settings/tokens".into()),
                signup_url: Some("https://github.com/signup".into()),
            },
            VendorId::Cursor => VendorLinks {
                usage_url: Some("https://cursor.com/dashboard".into()),
                billing_url: Some("https://cursor.com/settings".into()),
                status_url: None,
                docs_url: Some("https://docs.cursor.com".into()),
                app_url: Some("https://cursor.com".into()),
                api_key_url: None,
                signup_url: None,
            },
            VendorId::Openrouter => VendorLinks {
                usage_url: Some("https://openrouter.ai/activity".into()),
                billing_url: Some("https://openrouter.ai/credits".into()),
                status_url: None,
                docs_url: Some("https://openrouter.ai/docs".into()),
                app_url: Some("https://openrouter.ai".into()),
                api_key_url: Some("https://openrouter.ai/settings/keys".into()),
                signup_url: Some("https://openrouter.ai/auth".into()),
            },
            VendorId::Deepseek => VendorLinks {
                usage_url: Some("https://platform.deepseek.com/usage".into()),
                billing_url: Some("https://platform.deepseek.com/top_up".into()),
                status_url: None,
                docs_url: Some("https://api-docs.deepseek.com".into()),
                app_url: Some("https://chat.deepseek.com".into()),
                api_key_url: Some("https://platform.deepseek.com/api_keys".into()),
                signup_url: Some("https://platform.deepseek.com/sign_up".into()),
            },
            VendorId::Groq => VendorLinks {
                usage_url: Some("https://console.groq.com/dashboard/usage".into()),
                billing_url: Some("https://console.groq.com/settings/billing".into()),
                status_url: Some("https://status.groq.com".into()),
                docs_url: Some("https://console.groq.com/docs".into()),
                app_url: Some("https://console.groq.com".into()),
                api_key_url: Some("https://console.groq.com/keys".into()),
                signup_url: Some("https://console.groq.com/".into()),
            },
            VendorId::Windsurf => VendorLinks {
                usage_url: Some("https://codeium.com/profile".into()),
                billing_url: Some("https://codeium.com/subscription".into()),
                status_url: Some("https://status.codeium.com".into()),
                docs_url: Some("https://docs.codeium.com/windsurf".into()),
                app_url: Some("https://codeium.com/windsurf".into()),
                api_key_url: None,
                signup_url: None,
            },
            VendorId::Zai => VendorLinks {
                usage_url: Some("https://open.bigmodel.cn/usercenter/apikeys".into()),
                billing_url: Some("https://open.bigmodel.cn/usercenter/billing".into()),
                status_url: None,
                docs_url: Some("https://open.bigmodel.cn/dev/api".into()),
                app_url: None,
                api_key_url: Some("https://open.bigmodel.cn/usercenter/apikeys".into()),
                signup_url: Some("https://open.bigmodel.cn/".into()),
            },
            // No verified standalone API-key page: keep None rather than guess.
            VendorId::Minimax => VendorLinks {
                usage_url: Some(
                    "https://platform.minimaxi.com/user-center/basic-information".into(),
                ),
                billing_url: None,
                status_url: None,
                docs_url: Some("https://platform.minimaxi.com/document/guides".into()),
                app_url: None,
                api_key_url: None,
                signup_url: None,
            },
            VendorId::Kimi => VendorLinks {
                usage_url: Some("https://platform.moonshot.cn/console/info".into()),
                billing_url: Some("https://platform.moonshot.cn/console/pay".into()),
                status_url: None,
                docs_url: Some("https://platform.moonshot.cn/docs".into()),
                app_url: Some("https://kimi.moonshot.cn".into()),
                api_key_url: Some("https://platform.moonshot.cn/console/api-keys".into()),
                signup_url: Some("https://platform.moonshot.cn/".into()),
            },
            VendorId::Moonshot => VendorLinks {
                usage_url: Some("https://platform.moonshot.cn/console/info".into()),
                billing_url: Some("https://platform.moonshot.cn/console/pay".into()),
                status_url: None,
                docs_url: Some("https://platform.moonshot.cn/docs".into()),
                app_url: None,
                api_key_url: Some("https://platform.moonshot.ai/console/api-keys".into()),
                signup_url: Some("https://platform.moonshot.ai/".into()),
            },
            VendorId::Novita => VendorLinks {
                usage_url: Some("https://novita.ai/dashboard".into()),
                billing_url: Some("https://novita.ai/dashboard/billing".into()),
                status_url: None,
                docs_url: Some("https://novita.ai/docs".into()),
                app_url: None,
                api_key_url: Some("https://novita.ai/settings/key-management".into()),
                signup_url: Some("https://novita.ai/".into()),
            },
            VendorId::Grok => VendorLinks {
                usage_url: Some("https://console.x.ai/".into()),
                billing_url: Some("https://console.x.ai/billing".into()),
                status_url: None,
                docs_url: Some("https://docs.x.ai/".into()),
                app_url: Some("https://x.ai/grok".into()),
                api_key_url: Some("https://console.x.ai/team/api-keys".into()),
                signup_url: Some("https://console.x.ai/".into()),
            },
            // Local login: no API-key page by definition.
            VendorId::Supergrok => VendorLinks {
                usage_url: Some("https://console.x.ai/".into()),
                billing_url: Some("https://console.x.ai/billing".into()),
                status_url: None,
                docs_url: Some("https://docs.x.ai/".into()),
                app_url: Some("https://x.ai/grok".into()),
                api_key_url: None,
                signup_url: None,
            },
            // Keys are issued from the OpenCode console auth page
            // (per https://opencode.ai/docs/go/ and /docs/providers/).
            VendorId::OpenCodeGo => VendorLinks {
                usage_url: None,
                billing_url: None,
                status_url: None,
                docs_url: Some("https://opencode.ai/docs/go/".into()),
                app_url: Some("https://opencode.ai/".into()),
                api_key_url: Some("https://opencode.ai/auth".into()),
                signup_url: Some("https://opencode.ai/".into()),
            },
            _ => VendorLinks::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VendorLinks {
    pub usage_url: Option<String>,
    pub billing_url: Option<String>,
    pub status_url: Option<String>,
    pub docs_url: Option<String>,
    pub app_url: Option<String>,
    /// Official page where an API key is created/copied. `None` for
    /// OAuth/local vendors and for key vendors without a verified page —
    /// never a guessed link.
    #[serde(default)]
    pub api_key_url: Option<String>,
    /// Official signup/login page. Same no-guessing rule as above.
    #[serde(default)]
    pub signup_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    Oauth,
    ApiKey,
    Local,
    Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowType {
    Session,
    #[serde(rename = "5h")]
    FiveHour,
    Daily,
    Weekly,
    Monthly,
    Credits,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageUnit {
    Percent,
    Credits,
    Usd,
    Requests,
    Tokens,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResetStatus {
    Known,
    NotProvided,
    NotApplicable,
    FetchFailed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    #[serde(alias = "Available", alias = "available")]
    Available,
    #[serde(alias = "PartialLimited", alias = "partial_limited", alias = "partialLimited")]
    PartialLimited,
    #[serde(alias = "Blocked", alias = "blocked")]
    Blocked,
}

impl Availability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::PartialLimited => "partial_limited",
            Self::Blocked => "blocked",
        }
    }
}

impl Default for Availability {
    fn default() -> Self {
        Self::Available
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Connected,
    NeedsAuth,
    NeedsPermission,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatusReason {
    MissingCredential,
    InvalidCredential,
    /// An OAuth session was valid but can no longer be refreshed by this app.
    /// This is distinct from a mistyped API key because the right recovery is
    /// to reauthenticate with the provider's client.
    // snake_case convertiría "OAuth" en "o_auth_expired"; el frontend espera
    // "oauth_expired". El alias lee snapshots cacheados con el nombre viejo.
    #[serde(rename = "oauth_expired", alias = "o_auth_expired")]
    OAuthExpired,
    MissingPermission,
    LocalServiceUnavailable,
    NetworkUnavailable,
    RateLimited,
    ParseFailed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UsageSource {
    /// Preferencia de selección, nunca una fuente activa observada.
    Auto,
    Cli,
    Oauth,
    Api,
    LocalSession,
    WebSession,
    Web,
    Local,
}

/// De dónde sale el número y cuánto fiarse de él. Un porcentaje OAuth es
/// `Exact`; un coste derivado de logs locales es `Estimated` y la UI debe
/// etiquetarlo como tal, nunca como factura.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataConfidence {
    Exact,
    Estimated,
    PercentOnly,
    #[default]
    Unknown,
}

/// Salud del servicio del proveedor (Statuspage/feeds), independiente de
/// `ProviderStatus`, que describe nuestra conexión con la fuente.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceHealth {
    Operational,
    Degraded,
    Outage,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageQuota {
    pub id: String,
    pub label: String,
    pub window_type: WindowType,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub used_amount: Option<f64>,
    pub limit_amount: Option<f64>,
    pub unit: Option<UsageUnit>,
    pub reset_at: Option<String>,
    pub reset_in_seconds: Option<i64>,
    pub reset_status: ResetStatus,
    pub temporary_multiplier: Option<f64>,
    pub temporary_expires_at: Option<String>,
    pub source: UsageSource,
    pub fetched_at: String,
    pub stale: bool,
    /// `#[serde(default)]` mantiene legibles los cachés escritos por 0.2.0.
    #[serde(default)]
    pub confidence: DataConfidence,
    /// Proyección calculada al publicar un snapshot. Los snapshots internos
    /// del proveedor conservan `None`, para no confundir observación con dato
    /// derivado y para mantener legibles los cachés anteriores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace: Option<crate::pace::UsagePace>,
    /// Familia de modelos que comparte esta ventana. Aditivo: caches v1 sin
    /// el campo siguen siendo legibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_label: Option<String>,
    /// Metadata débil de display. Nunca entra en scoring ni disponibilidad.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    /// `always` | `details` | `diagnostic`. Vacío se trata como `always`.
    #[serde(default = "default_visible_always")]
    pub visible: String,
}

fn default_visible_always() -> String {
    "always".into()
}

pub fn is_summary_visible(visible: &str) -> bool {
    visible.is_empty() || visible == "always"
}

/// Un grupo de cuotas de primera clase (Gemini Models, Cursor Models, …).
#[derive(Debug, Clone, PartialEq)]
pub struct QuotaGroupView {
    pub id: String,
    pub label: String,
    pub models: Vec<String>,
    pub quotas: Vec<UsageQuota>,
}

impl QuotaGroupView {
    /// Used más restrictivo de las ventanas del grupo.
    pub fn used_percent(&self) -> Option<f64> {
        self.quotas
            .iter()
            .filter_map(|quota| quota.used_percent)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

/// Agrupa ventanas `always`. Cuotas `details`/`diagnostic` no inventan grupos.
pub fn groups_from_quotas(quotas: &[UsageQuota]) -> Vec<QuotaGroupView> {
    let mut groups: Vec<QuotaGroupView> = Vec::new();
    for quota in quotas {
        if !is_summary_visible(&quota.visible) || quota.id == "total" {
            continue;
        }
        let id = quota
            .group_id
            .clone()
            .unwrap_or_else(|| quota.id.clone());
        if let Some(existing) = groups.iter_mut().find(|group| group.id == id) {
            if existing.label.is_empty() {
                if let Some(label) = &quota.group_label {
                    existing.label = label.clone();
                }
            }
            if existing.models.is_empty() && !quota.models.is_empty() {
                existing.models = quota.models.clone();
            }
            existing.quotas.push(quota.clone());
            continue;
        }
        groups.push(QuotaGroupView {
            id,
            label: quota
                .group_label
                .clone()
                .unwrap_or_else(|| quota.label.clone()),
            models: quota.models.clone(),
            quotas: vec![quota.clone()],
        });
    }
    groups
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreditsSummary {
    pub remaining: f64,
    pub resets_available: Option<u64>,
    /// Credits are observed by the remote endpoint, not by Codex JSONL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<UsageSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    #[serde(default)]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_fetched_at: Option<String>,
    #[serde(default)]
    pub resets_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProductUsage {
    pub name: String,
    pub used_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_quota_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
}

impl ProductUsage {
    pub fn new(name: impl Into<String>, used_percent: f64) -> Self {
        Self {
            name: name.into(),
            used_percent,
            parent_quota_id: None,
            group_id: None,
        }
    }

    pub fn with_parent_quota(
        name: impl Into<String>,
        used_percent: f64,
        parent_quota_id: impl Into<String>,
    ) -> Self {
        let parent = parent_quota_id.into();
        Self {
            name: name.into(),
            used_percent,
            parent_quota_id: Some(parent.clone()),
            group_id: Some(parent),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageCost {
    pub today: Option<f64>,
    pub week: Option<f64>,
    pub thirty_days: Option<f64>,
    pub month: Option<f64>,
    /// `Estimated` cuando el número sale de logs locales (no es factura).
    #[serde(default)]
    pub confidence: DataConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MetricLine {
    Progress {
        id: String,
        label: String,
        used: f64,
        remaining: f64,
        limit: f64,
        format: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        resets_at: Option<String>,
        resets_in_label: String,
        window_secs: i64,
        visible: String,
    },
    Values {
        id: String,
        label: String,
        text: String,
        visible: String,
    },
    Badge {
        id: String,
        label: String,
        text: String,
        visible: String,
    },
}

impl MetricLine {
    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        match self {
            MetricLine::Progress { id, .. }
            | MetricLine::Values { id, .. }
            | MetricLine::Badge { id, .. } => id,
        }
    }

    pub fn utilization(&self) -> Option<f64> {
        match self {
            MetricLine::Progress { used, format, .. } if format == "percent" => Some(*used),
            MetricLine::Progress { used, limit, .. } if *limit > 0.0 => {
                Some((*used / *limit) * 100.0)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub id: String,
    pub name: String,
    pub short: String,
    pub plan: String,
    pub status: ProviderStatus,
    pub status_reason: Option<ProviderStatusReason>,
    #[serde(default)]
    pub availability: Availability,
    /// Salud del servicio del proveedor. Nunca mezclar con `status`.
    #[serde(default)]
    pub service: ServiceHealth,
    /// Fuente que produjo el snapshot actual (`None` = sin datos).
    #[serde(default)]
    pub active_source: Option<UsageSource>,
    pub stale: bool,
    pub error: Option<String>,
    pub hint: Option<String>,
    /// Most recent refresh attempt, successful or not. `updated_at` remains
    /// the timestamp of the last valid data, so stale cache is never passed
    /// off as a successful refresh.
    #[serde(default)]
    pub last_attempt_at: Option<String>,
    pub updated_at: String,
    pub quotas: Vec<UsageQuota>,
    pub credits: Option<CreditsSummary>,
    pub product_breakdown: Vec<ProductUsage>,
    pub cost: Option<UsageCost>,
    pub lines: Vec<MetricLine>,
    pub primary_utilization: Option<f64>,
}

impl ProviderUsage {
    pub fn is_connected(&self) -> bool {
        self.status == ProviderStatus::Connected
    }

    pub fn compute_availability(&self) -> Availability {
        if !self.is_connected() {
            return Availability::Blocked;
        }
        let relevant_quotas: Vec<&UsageQuota> = self
            .quotas
            .iter()
            .filter(|q| q.id != "total")
            .filter(|q| q.used_percent.is_some())
            .collect();
        if relevant_quotas.is_empty() {
            return Availability::Available;
        }
        let all_blocked = relevant_quotas
            .iter()
            .all(|q| q.used_percent.unwrap_or(0.0) >= 100.0);
        if all_blocked {
            return Availability::Blocked;
        }
        let any_blocked = relevant_quotas
            .iter()
            .any(|q| q.used_percent.unwrap_or(0.0) >= 100.0);
        if any_blocked {
            return Availability::PartialLimited;
        }
        Availability::Available
    }

    pub fn mark_stale(&mut self) {
        self.stale = true;
        for quota in &mut self.quotas {
            quota.stale = true;
        }
        if let Some(credits) = &mut self.credits {
            credits.stale = true;
            credits.resets_stale = credits.resets_available.is_some();
        }
    }

    /// Records the transport that actually produced this snapshot. This must
    /// be changed together with quota sources; otherwise the detail view and
    /// individual quota rows would contradict each other after a fallback.
    pub fn set_active_source(&mut self, source: UsageSource) {
        self.active_source = Some(source);
        for quota in &mut self.quotas {
            quota.source = source;
        }
    }
}

pub type ProviderSnapshot = ProviderUsage;

/// Where an API credential was found. This is deliberately metadata only:
/// serializing it must never expose the credential value itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialSource {
    Environment,
    Keyring,
    Legacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorInfo {
    pub id: String,
    pub name: String,
    pub short: String,
    pub auth_kind: AuthKind,
    pub env_key: Option<String>,
    pub hint: String,
    pub needs_key: bool,
    pub enabled: bool,
    pub detected: bool,
    /// True when the app holds a usable credential for this vendor right
    /// now (env var, OS keyring entry, or detected local login). This is a
    /// boolean only — the secret itself is never exposed. The UI derives
    /// "configured" from this; `enabled` stays fully independent.
    pub has_credential: bool,
    /// Origin of an API credential, when one is available. This makes an
    /// environment override visible without exposing the secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_source: Option<CredentialSource>,
    pub links: VendorLinks,
    /// Estrategias declaradas en orden de preferencia (`descriptor`).
    #[serde(default)]
    pub strategies: Vec<String>,
    /// Preferencia explícita (`None` = Automática).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub providers: Vec<ProviderSnapshot>,
    pub catalog: Vec<VendorInfo>,
    pub refresh_minutes: u64,
    #[serde(default = "default_adaptive")]
    pub refresh_adaptive: bool,
    pub primary: String,
    pub notifications: bool,
    pub notify_thresholds: Vec<u8>,
    pub autostart: bool,
    pub always_on_top: bool,
    pub compact_mode: bool,
    #[serde(default)]
    pub percentage_mode: crate::config::PercentageMode,
    pub app_bootstrapping: bool,
    pub refreshing: bool,
    pub loading_providers: Vec<String>,
    pub next_update_in_secs: u64,
    pub spend_month_usd: f64,
    pub spend: Vec<SpendRow>,
    pub recommend_id: Option<String>,
    pub recommend_name: Option<String>,
    pub recommend_left: Option<f64>,
    /// Motor contextual (`recommend::recommend`). Siempre presente en v0.3+;
    /// `#[serde(default)]` mantiene legibles cachés/payloads antiguos.
    #[serde(default)]
    pub recommend_action: String,
    #[serde(default)]
    pub recommend_reason: String,
    #[serde(default)]
    pub recommend_severity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommend_limiting_quota: Option<crate::recommend::LimitingQuota>,
    #[serde(default)]
    pub recommend_confidence: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommend_from: Option<String>,
    #[serde(default)]
    pub recommend_scores: Vec<crate::recommend::CandidateScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendRow {
    pub id: String,
    pub name: String,
    pub label: String,
    pub usd: f64,
}

pub fn parse_usd(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_prefix('$').unwrap_or(t).replace(',', "");
    let n: f64 = t.parse().ok()?;
    n.is_finite().then_some(n)
}

pub fn monthly_spend(p: &ProviderSnapshot) -> Option<(String, f64)> {
    if let Some(cost) = &p.cost {
        if let Some(value) = cost.month.or(cost.thirty_days).or(cost.week) {
            let label = if cost.month.is_some() {
                "Mes"
            } else if cost.thirty_days.is_some() {
                "30 días"
            } else {
                "Semana"
            };
            return Some((label.into(), value));
        }
    }
    let mut best: Option<(u8, String, f64)> = None;
    for line in &p.lines {
        let MetricLine::Values {
            id, label, text, ..
        } = line
        else {
            continue;
        };
        let rank = match id.as_str() {
            "cost_month" => 0,
            "cost_30" | "mtd" => 1,
            "cost_week" => 2,
            _ => continue,
        };
        let Some(usd) = parse_usd(text) else { continue };
        if best.as_ref().map(|(r, _, _)| rank < *r).unwrap_or(true) {
            best = Some((rank, label.clone(), usd));
        }
    }
    best.map(|(_, label, usd)| (label, usd))
}

pub fn now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

fn default_adaptive() -> bool {
    true
}

fn source_for(id: VendorId) -> UsageSource {
    match id {
        VendorId::Anthropic | VendorId::Openai | VendorId::Nous => UsageSource::Oauth,
        VendorId::Cursor
        | VendorId::Antigravity
        | VendorId::Kiro
        | VendorId::Supergrok
        | VendorId::CommandCode
        | VendorId::Windsurf => UsageSource::LocalSession,
        VendorId::Copilot => UsageSource::Cli,
        _ => UsageSource::Api,
    }
}

fn window_type(id: &str, window_secs: i64) -> WindowType {
    match id.to_ascii_lowercase().as_str() {
        "session" => WindowType::Session,
        "5h" | "five_hour" => WindowType::FiveHour,
        "daily" => WindowType::Daily,
        "weekly" => WindowType::Weekly,
        "monthly" => WindowType::Monthly,
        "credits" => WindowType::Credits,
        _ if window_secs > 0 && window_secs <= 6 * 3_600 => WindowType::FiveHour,
        _ if window_secs > 0 && window_secs <= 36 * 3_600 => WindowType::Daily,
        _ if window_secs >= 20 * 86_400 => WindowType::Monthly,
        _ if window_secs >= 6 * 86_400 => WindowType::Weekly,
        _ => WindowType::Custom,
    }
}

fn seconds_until(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso).ok().map(|dt| {
        (dt.with_timezone(&chrono::Utc) - chrono::Utc::now())
            .num_seconds()
            .max(0)
    })
}

fn quota_from_line(line: &MetricLine, source: UsageSource, fetched_at: &str) -> Option<UsageQuota> {
    let MetricLine::Progress {
        id,
        label,
        used,
        remaining,
        resets_at,
        window_secs,
        visible,
        ..
    } = line
    else {
        return None;
    };
    let reset_in_seconds = resets_at.as_deref().and_then(seconds_until);
    let reset_status = match (resets_at.is_some(), reset_in_seconds.is_some()) {
        (true, true) => ResetStatus::Known,
        (true, false) => ResetStatus::FetchFailed,
        (false, _) => ResetStatus::NotProvided,
    };
    Some(UsageQuota {
        id: id.clone(),
        label: label.clone(),
        window_type: window_type(id, *window_secs),
        used_percent: Some(*used),
        remaining_percent: Some(*remaining),
        used_amount: None,
        limit_amount: None,
        unit: Some(UsageUnit::Percent),
        reset_at: resets_at.clone(),
        reset_in_seconds,
        reset_status,
        temporary_multiplier: None,
        temporary_expires_at: None,
        source,
        fetched_at: fetched_at.to_string(),
        stale: false,
        confidence: DataConfidence::Exact,
        pace: None,
        group_id: None,
        group_label: None,
        models: Vec::new(),
        visible: if visible == "demand" {
            "details".into()
        } else if visible.is_empty() {
            "always".into()
        } else {
            visible.clone()
        },
    })
}

fn number_from_text(text: &str) -> Option<f64> {
    let normalized: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
        .collect();
    normalized.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn credits_from_lines(
    lines: &[MetricLine],
    source: UsageSource,
    fetched_at: &str,
) -> Option<CreditsSummary> {
    let remaining = lines.iter().find_map(|line| match line {
        MetricLine::Values { id, text, .. } if id == "credits" => number_from_text(text),
        _ => None,
    })?;
    let resets_available = lines.iter().find_map(|line| match line {
        MetricLine::Values { id, text, .. } if id == "resets" => {
            number_from_text(text).map(|n| n.max(0.0).floor() as u64)
        }
        _ => None,
    });
    Some(CreditsSummary {
        remaining: remaining.max(0.0).floor(),
        resets_available,
        source: Some(source),
        fetched_at: Some(fetched_at.to_string()),
        stale: false,
        resets_fetched_at: resets_available.map(|_| fetched_at.to_string()),
        resets_stale: false,
    })
}

fn cost_from_lines(lines: &[MetricLine]) -> Option<UsageCost> {
    let mut cost = UsageCost::default();
    for line in lines {
        let MetricLine::Values { id, text, .. } = line else {
            continue;
        };
        let value = parse_usd(text);
        match id.as_str() {
            "cost_today" => cost.today = value,
            "cost_week" => cost.week = value,
            "cost_30" => cost.thirty_days = value,
            "cost_month" | "mtd" => cost.month = value,
            _ => {}
        }
    }
    if cost.today.is_some()
        || cost.week.is_some()
        || cost.thirty_days.is_some()
        || cost.month.is_some()
    {
        // Los costes que llegan por líneas de la API del proveedor se toman
        // como exactos; `claude.rs::append_cost` los rebaja a `Estimated`
        // porque salen de logs locales.
        cost.confidence = DataConfidence::Exact;
        Some(cost)
    } else {
        None
    }
}

pub fn resets_in_label(iso: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(iso);
    let target = match parsed {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => return String::new(),
    };
    let secs = (target - chrono::Utc::now()).num_seconds();
    if secs <= 0 {
        return "ahora".to_string();
    }
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

pub fn resets_from_unix(ts: i64) -> (Option<String>, String, i64) {
    let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) else {
        return (None, String::new(), 0);
    };
    let iso = dt.to_rfc3339();
    let label = resets_in_label(&iso);
    (
        Some(iso),
        label,
        (dt - chrono::Utc::now()).num_seconds().max(0),
    )
}

pub fn progress_pct(
    id: &str,
    label: &str,
    pct: f64,
    resets_at: Option<String>,
    window_secs: i64,
    visible: &str,
) -> MetricLine {
    let resets_in_label = resets_at
        .as_deref()
        .map(resets_in_label)
        .unwrap_or_default();
    MetricLine::Progress {
        id: id.to_string(),
        label: label.to_string(),
        used: pct.clamp(0.0, 100.0),
        remaining: (100.0 - pct).clamp(0.0, 100.0),
        limit: 100.0,
        format: "percent".into(),
        resets_at,
        resets_in_label,
        window_secs,
        visible: visible.to_string(),
    }
}

pub fn values_line(id: &str, label: &str, text: &str, visible: &str) -> MetricLine {
    MetricLine::Values {
        id: id.to_string(),
        label: label.to_string(),
        text: text.to_string(),
        visible: visible.to_string(),
    }
}

pub fn badge_line(id: &str, label: &str, text: &str) -> MetricLine {
    MetricLine::Badge {
        id: id.to_string(),
        label: label.to_string(),
        text: text.to_string(),
        visible: "demand".into(),
    }
}

pub fn snapshot_ok(id: VendorId, plan: &str, lines: Vec<MetricLine>) -> ProviderSnapshot {
    let primary_utilization = lines.iter().find_map(|l| l.utilization());
    let updated_at = now_iso();
    let source = source_for(id);
    let quotas = lines
        .iter()
        .filter_map(|line| quota_from_line(line, source, &updated_at))
        .collect();
    let credits = credits_from_lines(&lines, source, &updated_at);
    let cost = cost_from_lines(&lines);
    let mut snapshot = ProviderSnapshot {
        id: id.slug().to_string(),
        name: id.display_name().to_string(),
        short: id.short().to_string(),
        plan: if plan.is_empty() {
            id.display_name().to_string()
        } else {
            plan.to_string()
        },
        status: ProviderStatus::Connected,
        status_reason: None,
        availability: Availability::Available,
        service: ServiceHealth::Unknown,
        active_source: Some(source),
        stale: false,
        error: None,
        hint: None,
        last_attempt_at: Some(updated_at.clone()),
        updated_at,
        quotas,
        credits,
        product_breakdown: Vec::new(),
        cost,
        lines,
        primary_utilization,
    };
    snapshot.availability = snapshot.compute_availability();
    snapshot
}

pub fn snapshot_with_status(
    id: VendorId,
    status: ProviderStatus,
    status_reason: ProviderStatusReason,
    error: &str,
) -> ProviderSnapshot {
    ProviderSnapshot {
        id: id.slug().to_string(),
        name: id.display_name().to_string(),
        short: id.short().to_string(),
        plan: String::new(),
        status,
        status_reason: Some(status_reason),
        availability: Availability::Blocked,
        service: ServiceHealth::Unknown,
        active_source: None,
        stale: false,
        error: Some(error.to_string()),
        hint: Some(id.login_hint().to_string()),
        last_attempt_at: Some(now_iso()),
        updated_at: now_iso(),
        quotas: vec![],
        credits: None,
        product_breakdown: vec![],
        cost: None,
        lines: vec![],
        primary_utilization: None,
    }
}

pub fn json_f64(v: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for k in keys {
        if let Some(n) = v.get(*k).and_then(|x| x.as_f64()) {
            return Some(n);
        }
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if let Ok(n) = s.parse::<f64>() {
                return Some(n);
            }
        }
    }
    None
}

pub fn json_str(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_percent_maps_utilization() {
        let line = progress_pct("session", "Sesión", 42.4, None, 18_000, "always");
        assert_eq!(line.utilization().unwrap().round(), 42.0);
    }

    #[test]
    fn vendor_links_never_guessed_for_unlisted_providers() {
        let kiro = VendorId::Kiro.links();
        assert!(kiro.usage_url.is_none());
        assert!(kiro.docs_url.is_none());
        let nous = VendorId::Nous.links();
        assert!(nous.status_url.is_none());
        assert!(nous.app_url.is_none());
        let cmd = VendorId::CommandCode.links();
        assert!(cmd.billing_url.is_none());
    }

    #[test]
    fn known_vendor_links_are_https() {
        let known = [
            VendorId::Anthropic,
            VendorId::AnthropicApi,
            VendorId::Openai,
            VendorId::OpenaiAdmin,
            VendorId::Copilot,
            VendorId::Cursor,
            VendorId::Openrouter,
            VendorId::Deepseek,
            VendorId::Groq,
            VendorId::Windsurf,
            VendorId::Zai,
            VendorId::Minimax,
            VendorId::Kimi,
            VendorId::Moonshot,
            VendorId::Novita,
            VendorId::Grok,
            VendorId::Supergrok,
        ];
        for id in known {
            let links = id.links();
            let urls = [
                links.usage_url,
                links.billing_url,
                links.status_url,
                links.docs_url,
                links.app_url,
            ];
            assert!(
                urls.iter().any(Option::is_some),
                "{} should have at least one link",
                id.slug()
            );
            for url in urls.into_iter().flatten() {
                assert!(url.starts_with("https://"), "{} -> {url}", id.slug());
            }
        }
    }

    #[test]
    fn vendor_slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for id in VendorId::all() {
            assert!(seen.insert(id.slug()), "duplicate {}", id.slug());
        }
    }

    #[test]
    fn parse_usd_strips_dollar() {
        assert_eq!(parse_usd("$12.50"), Some(12.5));
        assert_eq!(parse_usd("1,200.00"), Some(1200.0));
    }

    #[test]
    fn monthly_spend_prefers_cost_month() {
        let snap = snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![
                values_line("cost_today", "Hoy", "$1.00", "always"),
                values_line("cost_month", "Mes", "$40.00", "always"),
            ],
        );
        assert_eq!(monthly_spend(&snap), Some(("Mes".into(), 40.0)));
    }

    #[test]
    fn monthly_spend_ignores_lifetime_used() {
        let snap = snapshot_ok(
            VendorId::Openrouter,
            "OpenRouter",
            vec![values_line("used", "Usado", "$88.00", "always")],
        );
        assert_eq!(monthly_spend(&snap), None);
    }

    #[test]
    fn oauth_expired_reason_uses_the_frontend_name() {
        assert_eq!(
            serde_json::to_string(&ProviderStatusReason::OAuthExpired).unwrap(),
            "\"oauth_expired\""
        );
        let cached: ProviderStatusReason = serde_json::from_str("\"o_auth_expired\"").unwrap();
        assert_eq!(cached, ProviderStatusReason::OAuthExpired);
    }

    #[test]
    fn unknown_reset_has_explicit_status() {
        let snapshot = snapshot_ok(
            VendorId::Anthropic,
            "Pro",
            vec![progress_pct(
                "session", "Sesión", 3.0, None, 18_000, "always",
            )],
        );
        assert_eq!(snapshot.quotas[0].reset_status, ResetStatus::NotProvided);
        assert_eq!(snapshot.quotas[0].reset_at, None);
    }

    #[test]
    fn stale_snapshot_marks_every_quota_stale() {
        let mut snapshot = snapshot_ok(
            VendorId::Openai,
            "Plus",
            vec![progress_pct("5h", "5 horas", 0.0, None, 18_000, "always")],
        );
        snapshot.mark_stale();
        assert!(snapshot.stale);
        assert!(snapshot.quotas.iter().all(|quota| quota.stale));
    }

    #[test]
    fn active_source_tracks_the_source_that_produced_the_snapshot() {
        let mut snapshot = snapshot_ok(
            VendorId::Antigravity,
            "Pro",
            vec![progress_pct("5h", "5h", 10.0, None, 18_000, "always")],
        );
        snapshot.set_active_source(UsageSource::Oauth);
        assert_eq!(snapshot.active_source, Some(UsageSource::Oauth));
        assert!(snapshot
            .quotas
            .iter()
            .all(|quota| quota.source == UsageSource::Oauth));
    }

    #[test]
    fn groups_from_quotas_skips_details_and_total() {
        let mut snap = snapshot_ok(
            VendorId::Cursor,
            "Pro",
            vec![
                progress_pct("cursor_models", "Cursor Models", 38.0, None, 2_592_000, "always"),
                progress_pct("other_models", "Other Models", 100.0, None, 2_592_000, "always"),
                progress_pct("total", "Uso incluido total", 50.0, None, 2_592_000, "details"),
            ],
        );
        snap.quotas[0].group_id = Some("cursor_models".into());
        snap.quotas[0].group_label = Some("Cursor Models".into());
        snap.quotas[1].group_id = Some("other_models".into());
        snap.quotas[1].group_label = Some("Other Models".into());
        let groups = groups_from_quotas(&snap.quotas);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[1].used_percent(), Some(100.0));
    }

    #[test]
    fn old_quota_json_without_group_fields_still_parses() {
        let quota: UsageQuota = serde_json::from_str(
            r#"{"id":"session","label":"Sesión","windowType":"session","usedPercent":21.0,"remainingPercent":79.0,"usedAmount":null,"limitAmount":null,"unit":"percent","resetAt":null,"resetInSeconds":null,"resetStatus":"not_provided","temporaryMultiplier":null,"temporaryExpiresAt":null,"source":"oauth","fetchedAt":"2026-09-14T00:00:00Z","stale":false}"#,
        )
        .unwrap();
        assert_eq!(quota.group_id, None);
        assert!(quota.models.is_empty());
        assert_eq!(quota.visible, "always");
    }
}

pub fn snapshot_needs_auth(id: VendorId, error: &str) -> ProviderSnapshot {
    snapshot_with_status(
        id,
        ProviderStatus::NeedsAuth,
        ProviderStatusReason::MissingCredential,
        error,
    )
}
