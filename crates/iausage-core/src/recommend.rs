//! Motor de recomendación contextual.
//!
//! Sustituye a `most_headroom` (que solo miraba `100 - primary_utilization`)
//! por una puntuación que combina margen corto/largo, sostenibilidad
//! (¿llega al próximo reset al ritmo actual?), ventaja de reset, preferencia
//! del proveedor actual y confianza de datos, con histéresis para evitar
//! recomendaciones neuróticas.

use serde::{Deserialize, Serialize};

use crate::model::{
    Availability, DataConfidence, ProviderSnapshot, ProviderStatus, ProviderStatusReason,
    ServiceHealth, UsageQuota, WindowType,
};
use crate::pace::{compute_pace, window_secs_for};

/// Margen mínimo (puntos) para recomendar un cambio. Evita el flapping
/// `Claude 74 / Codex 76 → cambia` que nadie necesita.
pub const SWITCH_MARGIN: f64 = 15.0;
/// Umbral de margen corto por debajo del cual el proveedor actual "va justo".
pub const AT_RISK_SHORT_HEADROOM: f64 = 25.0;
/// Ventana para considerar un agotamiento como crítico (1h).
pub const CRITICAL_EXHAUST_SECS: i64 = 3_600;
pub const CRITICAL_USED_PERCENT: f64 = 95.0;
pub const PROJECTED_MIN_USED_PERCENT: f64 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecSeverity {
    Healthy,
    Warning,
    Critical,
}

impl RecSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitingQuota {
    pub id: String,
    pub label: String,
    pub window_type: WindowType,
    pub used_percent: f64,
    pub available_percent: f64,
    pub reset_at: Option<String>,
    pub reset_in_seconds: Option<i64>,
    pub expected_used_percent: Option<f64>,
    pub delta_percent: Option<f64>,
    pub estimated_exhausted_at: Option<String>,
    pub exhausts_before_reset_seconds: Option<i64>,
}

#[derive(Debug, Clone)]
struct ProviderSignal {
    severity: RecSeverity,
    reason: &'static str,
    quota: Option<LimitingQuota>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecAction {
    Stay,
    Switch,
    Balanced,
    InsufficientData,
}

impl RecAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecAction::Stay => "stay",
            RecAction::Switch => "switch",
            RecAction::Balanced => "balanced",
            RecAction::InsufficientData => "insufficient_data",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateScore {
    pub id: String,
    pub name: String,
    pub score: f64,
    pub short_headroom: Option<f64>,
    pub long_headroom: Option<f64>,
    /// Margen mostrado en el banner (% disponible de la ventana corta).
    pub display_left: Option<f64>,
    pub sustainable: Option<bool>,
    pub exhaust_in_secs: Option<i64>,
    pub reset_in_secs: Option<i64>,
    pub has_data: bool,
    pub is_reserve: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded: Option<String>,
    pub severity: String,
    pub signal_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limiting_quota: Option<LimitingQuota>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub action: String,
    pub from_id: String,
    pub to_id: Option<String>,
    pub to_name: Option<String>,
    /// Compat: margen del destino (lo que antes era `recommend_left`).
    pub left: Option<f64>,
    pub confidence: f64,
    /// Código de motivo para i18n, nunca texto libre.
    pub reason: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limiting_quota: Option<LimitingQuota>,
    pub candidates: Vec<CandidateScore>,
}

fn quota_used_percent(q: &crate::model::UsageQuota) -> Option<f64> {
    if let Some(u) = q.used_percent {
        return Some(u.clamp(0.0, 100.0));
    }
    match (q.used_amount, q.limit_amount) {
        (Some(u), Some(l)) if l > 0.0 => Some(((u / l) * 100.0).clamp(0.0, 100.0)),
        _ => None,
    }
}

fn quota_remaining(q: &crate::model::UsageQuota) -> Option<f64> {
    if let Some(r) = q.remaining_percent {
        return Some(r.clamp(0.0, 100.0));
    }
    quota_used_percent(q).map(|u| (100.0 - u).max(0.0))
}

fn window_type_str(w: WindowType) -> &'static str {
    match w {
        WindowType::Session => "session",
        WindowType::FiveHour => "5h",
        WindowType::Daily => "daily",
        WindowType::Weekly => "weekly",
        WindowType::Monthly => "monthly",
        WindowType::Credits => "credits",
        WindowType::Custom => "custom",
    }
}

fn is_short_window(w: WindowType) -> bool {
    matches!(
        w,
        WindowType::Session | WindowType::FiveHour | WindowType::Daily
    )
}

fn is_long_window(w: WindowType) -> bool {
    matches!(
        w,
        WindowType::Weekly | WindowType::Monthly | WindowType::Credits
    )
}

fn parse_unix(s: &Option<String>) -> Option<i64> {
    s.as_ref()
        .and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok())
        .map(|d| d.timestamp())
}

fn confidence_score(c: DataConfidence) -> f64 {
    match c {
        DataConfidence::Exact => 100.0,
        DataConfidence::Estimated => 70.0,
        DataConfidence::PercentOnly => 40.0,
        DataConfidence::Unknown => 20.0,
    }
}

fn reset_advantage(reset_in_secs: Option<i64>, sustainable: Option<bool>) -> f64 {
    match (reset_in_secs, sustainable) {
        // Sin reset conocido: neutral.
        (None, _) => 50.0,
        // Si llega al reset, la distancia importa poco.
        (Some(_), Some(true)) => 70.0,
        // Si se agota antes, un reset cercano es una ventaja (recupera antes).
        (Some(s), Some(false)) => {
            const H: i64 = 3_600;
            if s <= 6 * H {
                90.0
            } else if s <= 24 * H {
                75.0
            } else if s <= 72 * H {
                55.0
            } else if s <= 168 * H {
                35.0
            } else {
                15.0
            }
        }
        // Sin proyección: neutral.
        (Some(_), None) => 50.0,
    }
}

fn sustainability_of(
    snap: &ProviderSnapshot,
    now_unix: i64,
) -> (Option<bool>, Option<i64>, Option<i64>) {
    // Every canonical window participates. Presentation chooses the limiting
    // quota by severity, so long windows can no longer be hidden by sessions.
    let all = verdict_for(snap, now_unix, false);
    (all.0, all.1, all.2)
}

/// Proyección sobre quotas con reset conocido y ventana canónica.
/// `short_only` restringe a Session/FiveHour/Daily. Devuelve además si hubo
/// alguna quota proyectable. Un `will_last=false` con delta < 5 se considera
/// ruido (igual que la UI lo silencia) y no marca agotamiento.
fn verdict_for(
    snap: &ProviderSnapshot,
    now_unix: i64,
    short_only: bool,
) -> (Option<bool>, Option<i64>, Option<i64>, bool) {
    let mut worst_last: Option<bool> = None;
    let mut worst_exhaust: Option<i64> = None;
    let mut worst_reset: Option<i64> = None;
    let mut any_projectable = false;

    for q in &snap.quotas {
        if short_only && !is_short_window(q.window_type) {
            continue;
        }
        let Some(used) = quota_used_percent(q) else {
            continue;
        };
        let reset_unix = parse_unix(&q.reset_at);
        let window_secs = window_secs_for(window_type_str(q.window_type));
        // Solo quotas con reset conocido y ventana canónica proyectan.
        let (Some(reset), Some(window_secs)) = (reset_unix, window_secs) else {
            continue;
        };
        let Some(pace) = compute_pace(used, window_secs, reset, now_unix) else {
            continue;
        };
        any_projectable = true;
        match pace.will_last_to_reset {
            Some(false) if pace.delta_percent >= 5.0 => {
                worst_last = Some(false);
                if let Some(at) = pace.estimated_exhausted_at {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&at) {
                        let exhaust_in = dt.timestamp() - now_unix;
                        worst_exhaust = Some(match worst_exhaust {
                            Some(prev) => prev.min(exhaust_in),
                            None => exhaust_in,
                        });
                    }
                }
                let reset_in = reset - now_unix;
                worst_reset = Some(match worst_reset {
                    Some(prev) => prev.min(reset_in),
                    None => reset_in,
                });
            }
            Some(true) => {
                if worst_last.is_none() {
                    worst_last = Some(true);
                    worst_reset = Some(reset - now_unix);
                }
            }
            // `Some(false)` con delta < 5 o `None`: ruido o sin proyección.
            // Cuenta como proyectable pero no marca agotamiento.
            _ => {
                if worst_reset.is_none() {
                    worst_reset = Some(reset - now_unix);
                }
            }
        }
    }

    if !any_projectable {
        return (None, None, reset_fallback(snap, now_unix), false);
    }
    // Si ninguna quota computable se agota, llega.
    let sustainable = worst_last.or(Some(true));
    let reset_in = worst_reset.or_else(|| reset_fallback(snap, now_unix));
    (sustainable, worst_exhaust, reset_in, true)
}

fn reset_fallback(snap: &ProviderSnapshot, now_unix: i64) -> Option<i64> {
    snap.quotas
        .iter()
        .filter_map(|q| parse_unix(&q.reset_at).map(|r| r - now_unix))
        .filter(|d| *d >= 0)
        .min()
}

fn headroom_for(snap: &ProviderSnapshot, short: bool) -> Option<f64> {
    let mut best: Option<f64> = None;
    for q in &snap.quotas {
        if q.id == "total" {
            continue;
        }
        let matches = if short {
            is_short_window(q.window_type)
        } else {
            is_long_window(q.window_type)
        };
        if !matches {
            continue;
        }
        if q.window_type == WindowType::Credits && quota_remaining(q).is_none() {
            continue;
        }
        if let Some(r) = quota_remaining(q) {
            if snap.availability == Availability::PartialLimited && r <= 0.0 {
                continue;
            }
            best = Some(match best {
                Some(prev) => prev.min(r),
                None => r,
            });
        }
    }
    best
}

fn avg_confidence(snap: &ProviderSnapshot) -> Option<f64> {
    let mut sum = 0.0;
    let mut n = 0usize;
    for q in &snap.quotas {
        if quota_used_percent(q).is_none() && quota_remaining(q).is_none() {
            continue;
        }
        sum += confidence_score(q.confidence);
        n += 1;
    }
    if n == 0 {
        return None;
    }
    Some(sum / n as f64)
}

fn exclusion_reason(snap: &ProviderSnapshot, now_unix: i64) -> Option<String> {
    if !snap.is_connected() {
        return Some(match snap.status_reason {
            Some(ProviderStatusReason::InvalidCredential | ProviderStatusReason::OAuthExpired) => {
                "auth".to_string()
            }
            _ => "offline".to_string(),
        });
    }
    // Stale grave: marcado stale, sin quota usable y último intento viejo.
    // El stale leve (merge_snapshot retiene métricas válidas) sigue puntuando.
    let has_usable = snap
        .quotas
        .iter()
        .any(|q| quota_used_percent(q).is_some() || quota_remaining(q).is_some())
        || snap.primary_utilization.is_some();
    if snap.stale && !has_usable {
        return Some("stale".to_string());
    }
    if snap.stale {
        if let Some(last) = parse_unix(&snap.last_attempt_at) {
            if now_unix - last > 30 * 60 {
                // Stale de más de 30 min con datos: no se excluye, se penaliza.
                return None;
            }
        }
    }
    None
}

fn limiting_quota(quota: &crate::model::UsageQuota, used: f64, now_unix: i64) -> LimitingQuota {
    let reset_unix = parse_unix(&quota.reset_at);
    let reset_in_seconds = reset_unix
        .map(|reset| (reset - now_unix).max(0))
        .or(quota.reset_in_seconds);
    let pace = if quota.stale {
        None
    } else {
        reset_unix
            .zip(window_secs_for(window_type_str(quota.window_type)))
            .and_then(|(reset, window)| compute_pace(used, window, reset, now_unix))
    };
    let exhausted_unix = pace
        .as_ref()
        .and_then(|value| value.estimated_exhausted_at.as_ref())
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp());
    let exhausts_before_reset_seconds = reset_unix
        .zip(exhausted_unix)
        .map(|(reset, exhausted)| (reset - exhausted).max(0));

    LimitingQuota {
        id: quota.id.clone(),
        label: quota.label.clone(),
        window_type: quota.window_type,
        used_percent: used,
        available_percent: (100.0 - used).clamp(0.0, 100.0),
        reset_at: quota.reset_at.clone(),
        reset_in_seconds,
        expected_used_percent: pace.as_ref().map(|value| value.expected_used_percent),
        delta_percent: pace.as_ref().map(|value| value.delta_percent),
        estimated_exhausted_at: pace
            .as_ref()
            .and_then(|value| value.estimated_exhausted_at.clone()),
        exhausts_before_reset_seconds,
    }
}

/// Chooses the provider's most restrictive signal. Severity always wins over
/// window length: a weekly quota at 99% outranks a merely fast session.
fn provider_signal(snap: &ProviderSnapshot, now_unix: i64) -> ProviderSignal {
    if snap.availability == Availability::PartialLimited {
        let limiting = snap
            .quotas
            .iter()
            .filter(|q| q.id != "total")
            .filter_map(|q| quota_used_percent(q).map(|u| (q, u)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(q, u)| limiting_quota(q, u, now_unix));

        return ProviderSignal {
            severity: RecSeverity::Warning,
            reason: "partial_limited",
            quota: limiting,
        };
    }

    let mut critical: Option<LimitingQuota> = None;
    for quota in &snap.quotas {
        if quota.id == "total" {
            continue;
        }
        let Some(used) = quota_used_percent(quota) else {
            continue;
        };
        if used < CRITICAL_USED_PERCENT {
            continue;
        }
        let candidate = limiting_quota(quota, used, now_unix);
        let replace = critical
            .as_ref()
            .map(|current| candidate.used_percent > current.used_percent)
            .unwrap_or(true);
        if replace {
            critical = Some(candidate);
        }
    }
    if let Some(quota) = critical {
        let reason = if quota.used_percent >= 100.0 {
            "quota_exhausted"
        } else {
            "quota_near_exhaustion"
        };
        return ProviderSignal {
            severity: RecSeverity::Critical,
            reason,
            quota: Some(quota),
        };
    }

    if !snap.stale {
        let mut projected: Option<LimitingQuota> = None;
        for quota in &snap.quotas {
            if quota.id == "total" || quota.stale {
                continue;
            }
            let Some(used) = quota_used_percent(quota) else {
                continue;
            };
            if used < PROJECTED_MIN_USED_PERCENT {
                continue;
            }
            let candidate = limiting_quota(quota, used, now_unix);
            if candidate.delta_percent.unwrap_or(0.0) < 5.0
                || candidate.exhausts_before_reset_seconds.unwrap_or(0) <= 0
            {
                continue;
            }
            let replace = projected
                .as_ref()
                .map(|current| {
                    candidate.exhausts_before_reset_seconds.unwrap_or(0)
                        > current.exhausts_before_reset_seconds.unwrap_or(0)
                        || (candidate.exhausts_before_reset_seconds
                            == current.exhausts_before_reset_seconds
                            && candidate.used_percent > current.used_percent)
                })
                .unwrap_or(true);
            if replace {
                projected = Some(candidate);
            }
        }
        if let Some(quota) = projected {
            return ProviderSignal {
                severity: RecSeverity::Warning,
                reason: "projected_exhaustion",
                quota: Some(quota),
            };
        }
    }

    match (snap.status, snap.service) {
        (_, ServiceHealth::Outage) | (ProviderStatus::Unavailable | ProviderStatus::Error, _) => {
            ProviderSignal {
                severity: RecSeverity::Critical,
                reason: "service_outage",
                quota: None,
            }
        }
        (_, ServiceHealth::Degraded) => ProviderSignal {
            severity: RecSeverity::Warning,
            reason: "service_degraded",
            quota: None,
        },
        (ProviderStatus::NeedsAuth | ProviderStatus::NeedsPermission, _) => ProviderSignal {
            severity: RecSeverity::Warning,
            reason: "auth_problem",
            quota: None,
        },
        _ => ProviderSignal {
            severity: RecSeverity::Healthy,
            reason: "sustainable",
            quota: None,
        },
    }
}

fn score_candidate(snap: &ProviderSnapshot, current_id: &str, now_unix: i64) -> CandidateScore {
    let excluded = exclusion_reason(snap, now_unix);
    let signal = provider_signal(snap, now_unix);

    let mut short_headroom = headroom_for(snap, true);
    let mut long_headroom = headroom_for(snap, false);
    // Fallback a primary_utilization cuando no hay quotas con %.
    if short_headroom.is_none() && long_headroom.is_none() {
        if let Some(u) = snap.primary_utilization {
            let left = (100.0 - u).max(0.0);
            short_headroom = Some(left);
            long_headroom = Some(left);
        }
    }
    let display_left = short_headroom
        .or(long_headroom)
        .or_else(|| snap.primary_utilization.map(|u| (100.0 - u).max(0.0)));

    let has_data = display_left.is_some() && !snap.quotas.is_empty()
        || short_headroom.is_some()
        || long_headroom.is_some()
        || (snap.primary_utilization.is_some() && !snap.quotas.is_empty());
    // Conectado pero sin quotas ni primary: 100% vacío (pago por uso / sin historial).
    let has_data = if snap.quotas.is_empty() && snap.primary_utilization.is_none() {
        false
    } else {
        has_data
    };
    let is_reserve = !has_data && snap.is_connected();

    let (sustainable, exhaust_in_secs, reset_in_secs) = if snap.stale {
        (None, None, reset_fallback(snap, now_unix))
    } else {
        sustainability_of(snap, now_unix)
    };

    let short = short_headroom.unwrap_or(50.0);
    let long = long_headroom.unwrap_or(50.0);
    let sust_num = match sustainable {
        Some(true) => 100.0,
        Some(false) => 0.0,
        None => 50.0,
    };
    let reset_num = reset_advantage(reset_in_secs, sustainable);
    let pref_num = if snap.id == current_id { 100.0 } else { 0.0 };
    let mut conf_num = avg_confidence(snap).unwrap_or(20.0);
    if snap.stale {
        conf_num = (conf_num - 30.0).max(0.0);
    }

    let mut score = 0.30 * short
        + 0.20 * long
        + 0.25 * sust_num
        + 0.10 * reset_num
        + 0.10 * pref_num
        + 0.05 * conf_num;

    // Penalizaciones (ver plan): el stale leve y el 100% vacío no excluyen,
    // pero nunca ganan a un current sostenible.
    if snap.stale {
        score -= 15.0;
    }
    if sustainable == Some(false) {
        score -= 10.0;
    }
    if snap.availability == Availability::PartialLimited {
        score -= 20.0;
    }
    if !has_data {
        score -= 20.0;
        score = score.min(50.0);
    }
    if excluded.is_some() {
        score = 0.0;
    }
    score = score.clamp(0.0, 100.0);

    CandidateScore {
        id: snap.id.clone(),
        name: snap.name.clone(),
        score,
        short_headroom,
        long_headroom,
        display_left,
        sustainable,
        exhaust_in_secs,
        reset_in_secs,
        has_data,
        is_reserve,
        excluded,
        severity: signal.severity.as_str().to_string(),
        signal_reason: signal.reason.to_string(),
        limiting_quota: signal.quota,
    }
}

fn confidence_for(
    action: &RecAction,
    from: Option<&CandidateScore>,
    to: Option<&CandidateScore>,
    any_stale: bool,
) -> f64 {
    let mut c: f64 = match action {
        RecAction::InsufficientData => 0.3,
        RecAction::Balanced => 0.7,
        RecAction::Stay | RecAction::Switch => 0.9,
    };
    let unknown_side = from.map(|f| f.sustainable.is_none()).unwrap_or(true)
        || to.map(|t| t.sustainable.is_none()).unwrap_or(true);
    if unknown_side {
        c -= 0.2;
    }
    if to.map(|t| t.is_reserve).unwrap_or(false) {
        c -= 0.3;
    }
    if any_stale {
        c -= 0.1;
    }
    c.clamp(0.3, 0.95)
}

/// Calcula la recomendación contextual sobre los snapshots habilitados.
///
/// `current_id` es el proveedor actual (`cfg.primary` en el backend; el
/// frontend lo interpreta frente al tab seleccionado con la misma regla de
/// histéresis).
pub fn recommend(
    providers: &[ProviderSnapshot],
    current_id: &str,
    now_unix: i64,
) -> Recommendation {
    let mut candidates: Vec<CandidateScore> = providers
        .iter()
        .map(|s| score_candidate(s, current_id, now_unix))
        .collect();
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let valid: Vec<&CandidateScore> = candidates.iter().filter(|c| c.excluded.is_none()).collect();
    let with_data: Vec<&CandidateScore> = valid.iter().filter(|c| c.has_data).copied().collect();
    let current_any = candidates
        .iter()
        .find(|candidate| candidate.id == current_id);
    let any_stale = providers.iter().any(|p| p.stale);

    if with_data.is_empty() {
        return Recommendation {
            action: RecAction::InsufficientData.as_str().to_string(),
            from_id: current_id.to_string(),
            to_id: None,
            to_name: None,
            left: None,
            confidence: confidence_for(&RecAction::InsufficientData, None, None, any_stale),
            reason: "insufficient_data".to_string(),
            severity: RecSeverity::Healthy.as_str().to_string(),
            limiting_quota: None,
            candidates: top_candidates(candidates),
        };
    }
    // Un solo candidato con datos: quedarse (o señalar la reserva).
    if with_data.len() == 1 {
        let only = with_data[0];
        if only.id != current_id {
            return Recommendation {
                action: RecAction::Switch.as_str().to_string(),
                from_id: current_id.to_string(),
                to_id: Some(only.id.clone()),
                to_name: Some(only.name.clone()),
                left: only.display_left,
                confidence: confidence_for(&RecAction::Switch, current_any, Some(only), any_stale),
                reason: current_any
                    .map(|candidate| candidate.signal_reason.clone())
                    .unwrap_or_else(|| "current_unavailable".to_string()),
                severity: current_any
                    .map(|candidate| candidate.severity.clone())
                    .unwrap_or_else(|| RecSeverity::Warning.as_str().to_string()),
                limiting_quota: current_any.and_then(|candidate| candidate.limiting_quota.clone()),
                candidates: top_candidates(candidates),
            };
        }
        let has_reserve = valid.iter().any(|c| c.is_reserve);
        let reason = if only.severity != RecSeverity::Healthy.as_str() {
            only.signal_reason.as_str()
        } else if has_reserve && only.id == current_id {
            // Hay un 100% vacío al acecho: el destino sigue siendo el
            // actual (acción Stay); el frontend nombra la reserva desde
            // `candidates` para la copia informativa.
            "reserve_no_history"
        } else {
            "sustainable"
        };
        let to_id = Some(only.id.clone());
        let to_name = Some(only.name.clone());
        let left = only.display_left;
        return Recommendation {
            action: RecAction::Stay.as_str().to_string(),
            from_id: current_id.to_string(),
            to_id,
            to_name,
            left,
            confidence: confidence_for(&RecAction::Stay, Some(only), Some(only), any_stale),
            reason: reason.to_string(),
            severity: only.severity.clone(),
            limiting_quota: only.limiting_quota.clone(),
            candidates: top_candidates(candidates),
        };
    }

    let current = valid.iter().find(|c| c.id == current_id).copied();
    let best = with_data
        .iter()
        .max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied();

    // Current no válido (offline/auth): cambiar al mejor con datos.
    if current.is_none() {
        if let Some(best) = best {
            return Recommendation {
                action: RecAction::Switch.as_str().to_string(),
                from_id: current_id.to_string(),
                to_id: Some(best.id.clone()),
                to_name: Some(best.name.clone()),
                left: best.display_left,
                confidence: confidence_for(&RecAction::Switch, None, Some(best), any_stale),
                reason: current_any
                    .map(|candidate| candidate.signal_reason.clone())
                    .unwrap_or_else(|| "current_unavailable".to_string()),
                severity: current_any
                    .map(|candidate| candidate.severity.clone())
                    .unwrap_or_else(|| RecSeverity::Warning.as_str().to_string()),
                limiting_quota: current_any.and_then(|candidate| candidate.limiting_quota.clone()),
                candidates: top_candidates(candidates),
            };
        }
    }

    if let (Some(cur), Some(best)) = (current, best) {
        // The limiting signal drives the diagnosis. Candidate scoring only
        // chooses an alternative and cannot replace a severe weekly quota
        // with a less severe short-window prediction.
        if cur.severity != RecSeverity::Healthy.as_str() {
            let alt = with_data
                .iter()
                .copied()
                .filter(|candidate| {
                    candidate.id != cur.id
                        && candidate.display_left.is_some_and(|left| left > 0.0)
                        && candidate.severity != RecSeverity::Critical.as_str()
                        && (cur.severity == RecSeverity::Critical.as_str()
                            || candidate.severity == RecSeverity::Healthy.as_str()
                                && (candidate.score >= cur.score + SWITCH_MARGIN
                                    || cur.sustainable == Some(false)
                                        && candidate.sustainable != Some(false))
                            || cur.sustainable == Some(false)
                                && candidate.exhaust_in_secs.unwrap_or(0)
                                    >= (2 * cur.exhaust_in_secs.unwrap_or(0))
                                        .max(cur.exhaust_in_secs.unwrap_or(0) + 3_600))
                })
                .max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            let action = if alt.is_some() {
                RecAction::Switch
            } else {
                RecAction::Stay
            };
            let target = alt.unwrap_or(cur);
            return Recommendation {
                action: action.as_str().to_string(),
                from_id: current_id.to_string(),
                to_id: Some(target.id.clone()),
                to_name: Some(target.name.clone()),
                left: target.display_left,
                confidence: confidence_for(&action, Some(cur), Some(target), any_stale),
                reason: cur.signal_reason.clone(),
                severity: cur.severity.clone(),
                limiting_quota: cur.limiting_quota.clone(),
                candidates: top_candidates(candidates),
            };
        }
        // El actual se agota antes de su reset: SWITCH_MARGIN y el bonus del
        // actual evitan el flapping entre proveedores sanos, pero no pueden
        // retener al usuario en el que se acaba primero (tampoco la
        // penalización por stale leve de la alternativa). Gana la alternativa
        // con mejor puntuación entre las que aguantan claramente más.
        if cur.sustainable == Some(false) {
            let cur_exhaust = cur.exhaust_in_secs.unwrap_or(0);
            let lasts_longer = |c: &&CandidateScore| {
                c.id != cur.id
                    && c.display_left.is_some_and(|left| left > 0.0)
                    && (c.sustainable != Some(false)
                        || c.exhaust_in_secs.unwrap_or(0)
                            >= (2 * cur_exhaust).max(cur_exhaust + 3_600))
            };
            let alt = with_data
                .iter()
                .copied()
                .filter(lasts_longer)
                .max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            if let Some(alt) = alt {
                return Recommendation {
                    action: RecAction::Switch.as_str().to_string(),
                    from_id: current_id.to_string(),
                    to_id: Some(alt.id.clone()),
                    to_name: Some(alt.name.clone()),
                    left: alt.display_left,
                    confidence: confidence_for(&RecAction::Switch, Some(cur), Some(alt), any_stale),
                    reason: reserves_or_critical(cur).to_string(),
                    severity: cur.severity.clone(),
                    limiting_quota: cur.limiting_quota.clone(),
                    candidates: top_candidates(candidates),
                };
            }
        }

        // El mejor nunca es una reserva: with_data ya las excluye.
        if best.id != cur.id
            && best.score >= cur.score + SWITCH_MARGIN
            && cur.severity != RecSeverity::Healthy.as_str()
        {
            // Solo cambiar a un destino que no se agote (salvo que el
            // actual tampoco llegue y el destino aguante más).
            let dest_ok = best.sustainable != Some(false)
                || (cur.sustainable == Some(false)
                    && best.exhaust_in_secs.unwrap_or(i64::MAX) > cur.exhaust_in_secs.unwrap_or(0));
            if dest_ok {
                let reason = if cur.sustainable == Some(false) {
                    if cur.exhaust_in_secs.unwrap_or(i64::MAX) <= CRITICAL_EXHAUST_SECS {
                        "critical_short"
                    } else {
                        "exhausts_before_reset"
                    }
                } else if short_is_low(cur) {
                    "at_risk"
                } else {
                    "sustainable"
                };
                return Recommendation {
                    action: RecAction::Switch.as_str().to_string(),
                    from_id: current_id.to_string(),
                    to_id: Some(best.id.clone()),
                    to_name: Some(best.name.clone()),
                    left: best.display_left,
                    confidence: confidence_for(
                        &RecAction::Switch,
                        Some(cur),
                        Some(best),
                        any_stale,
                    ),
                    reason: reason.to_string(),
                    severity: cur.severity.clone(),
                    limiting_quota: cur.limiting_quota.clone(),
                    candidates: top_candidates(candidates),
                };
            }
        }

        // Sin mejora suficiente: quedarse. Distinguir equilibrado / riesgo.
        if cur.sustainable == Some(false) {
            let reason = reserves_or_critical(cur);
            return Recommendation {
                action: RecAction::Stay.as_str().to_string(),
                from_id: current_id.to_string(),
                to_id: Some(cur.id.clone()),
                to_name: Some(cur.name.clone()),
                left: cur.display_left,
                confidence: confidence_for(&RecAction::Stay, Some(cur), Some(cur), any_stale),
                reason: reason.to_string(),
                severity: cur.severity.clone(),
                limiting_quota: cur.limiting_quota.clone(),
                candidates: top_candidates(candidates),
            };
        }
        // Equilibrado: top2 cerca y ambos sostenibles.
        let mut sorted_data = with_data.clone();
        sorted_data.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if sorted_data.len() >= 2 {
            let (first, second) = (sorted_data[0], sorted_data[1]);
            if (first.score - second.score).abs() <= 10.0
                && first.sustainable != Some(false)
                && second.sustainable != Some(false)
            {
                return Recommendation {
                    action: RecAction::Balanced.as_str().to_string(),
                    from_id: current_id.to_string(),
                    to_id: Some(cur.id.clone()),
                    to_name: Some(cur.name.clone()),
                    left: cur.display_left,
                    confidence: confidence_for(
                        &RecAction::Balanced,
                        Some(cur),
                        Some(cur),
                        any_stale,
                    ),
                    reason: "balanced".to_string(),
                    severity: RecSeverity::Healthy.as_str().to_string(),
                    limiting_quota: None,
                    candidates: top_candidates(candidates),
                };
            }
        }
        if short_is_low(cur) && cur.severity != RecSeverity::Healthy.as_str() {
            // Va justo pero sin alternativa ≥15 pts: avisar sin ordenar cambio.
            let alt = if best.id != cur.id { Some(best) } else { None };
            return Recommendation {
                action: RecAction::Stay.as_str().to_string(),
                from_id: current_id.to_string(),
                to_id: Some(alt.map(|b| b.id.clone()).unwrap_or(cur.id.clone())),
                to_name: Some(alt.map(|b| b.name.clone()).unwrap_or(cur.name.clone())),
                left: alt.map(|b| b.display_left).unwrap_or(cur.display_left),
                confidence: confidence_for(&RecAction::Stay, Some(cur), Some(cur), any_stale),
                reason: "at_risk".to_string(),
                severity: RecSeverity::Warning.as_str().to_string(),
                limiting_quota: cur.limiting_quota.clone(),
                candidates: top_candidates(candidates),
            };
        }
        return Recommendation {
            action: RecAction::Stay.as_str().to_string(),
            from_id: current_id.to_string(),
            to_id: Some(cur.id.clone()),
            to_name: Some(cur.name.clone()),
            left: cur.display_left,
            confidence: confidence_for(&RecAction::Stay, Some(cur), Some(cur), any_stale),
            reason: "sustainable".to_string(),
            severity: RecSeverity::Healthy.as_str().to_string(),
            limiting_quota: None,
            candidates: top_candidates(candidates),
        };
    }

    Recommendation {
        action: RecAction::InsufficientData.as_str().to_string(),
        from_id: current_id.to_string(),
        to_id: None,
        to_name: None,
        left: None,
        confidence: confidence_for(&RecAction::InsufficientData, None, None, any_stale),
        reason: "insufficient_data".to_string(),
        severity: RecSeverity::Healthy.as_str().to_string(),
        limiting_quota: None,
        candidates: top_candidates(candidates),
    }
}

fn short_is_low(c: &CandidateScore) -> bool {
    c.short_headroom.unwrap_or(50.0) < AT_RISK_SHORT_HEADROOM
}

fn reserves_or_critical(cur: &CandidateScore) -> &'static str {
    if cur.exhaust_in_secs.unwrap_or(i64::MAX) <= CRITICAL_EXHAUST_SECS {
        "critical_short"
    } else {
        "exhausts_before_reset"
    }
}

fn top_candidates(mut all: Vec<CandidateScore>) -> Vec<CandidateScore> {
    all.truncate(3);
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok, snapshot_with_status, VendorId};
    use crate::model::{ProviderStatus, ProviderStatusReason};

    fn now() -> i64 {
        1_700_000_000
    }

    fn rfc(ts: i64) -> String {
        chrono::DateTime::from_timestamp(ts, 0)
            .map(|d| d.to_rfc3339())
            .unwrap()
    }

    /// Snapshot conectado con quotas de % sobre ventanas cortas y largas.
    /// `short_used`/`long_used` en %; `reset_in` en segundos para la corta.
    fn snap_window(
        id: VendorId,
        short_used: f64,
        long_used: f64,
        reset_in_secs: i64,
    ) -> ProviderSnapshot {
        let n = now();
        let mut s = snapshot_ok(
            id,
            "Pro",
            vec![
                progress_pct("session", "Sesión", short_used, None, 18_000, "always"),
                progress_pct("weekly", "Semanal", long_used, None, 604_800, "always"),
            ],
        );
        // Fijar resets conocidos: corta en `reset_in_secs`, larga en 6 días.
        s.quotas[0].reset_at = Some(rfc(n + reset_in_secs));
        s.quotas[0].reset_in_seconds = Some(reset_in_secs);
        s.quotas[1].reset_at = Some(rfc(n + 6 * 86_400));
        s.quotas[1].reset_in_seconds = Some(6 * 86_400);
        s.updated_at = rfc(n);
        s.last_attempt_at = Some(rfc(n));
        s
    }

    fn snap_empty_connected(id: VendorId) -> ProviderSnapshot {
        let n = now();
        let mut s = snapshot_ok(id, "Go", vec![]);
        s.quotas = vec![];
        s.primary_utilization = None;
        s.lines = vec![];
        s.updated_at = rfc(n);
        s.last_attempt_at = Some(rfc(n));
        s
    }

    #[test]
    fn sustainable_current_beats_empty_hundred_percent() {
        // Claude 65% usado (35% disponible) con ritmo sano; OpenCode sin
        // quotas (100% vacío) nunca debe ganar.
        let mut claude = snap_window(VendorId::Anthropic, 30.0, 20.0, 3 * 3_600);
        claude.id = "anthropic".into();
        claude.name = "Claude Code".into();
        let mut go = snap_empty_connected(VendorId::Openrouter);
        go.id = "opencode".into();
        go.name = "OpenCode Go".into();
        let rec = recommend(&[claude, go], "anthropic", now());
        assert_eq!(rec.action, "stay");
        assert_eq!(rec.to_id.as_deref(), Some("anthropic"));
        // Señala la reserva 100% vacía sin recomendar cambiar a ella.
        assert_eq!(rec.reason, "reserve_no_history");
    }

    #[test]
    fn exhausting_current_switches_to_healthy_alternative() {
        // Claude 88% con 72% de la ventana transcurrida → se agota antes;
        // Codex sano con mucho margen → switch.
        let window = 18_000i64;
        let n = now();
        let elapsed = (0.72 * window as f64) as i64;
        let reset_in = window - elapsed;
        let mut claude = snap_window(VendorId::Anthropic, 88.0, 60.0, reset_in);
        claude.id = "anthropic".into();
        claude.name = "Claude Code".into();
        let mut codex = snap_window(VendorId::Openai, 20.0, 15.0, 5 * 3_600);
        codex.id = "openai".into();
        codex.name = "Codex".into();
        let rec = recommend(&[claude, codex], "anthropic", n);
        assert_eq!(rec.action, "switch", "candidates: {:?}", rec.candidates);
        assert_eq!(rec.to_id.as_deref(), Some("openai"));
        assert_eq!(rec.reason, "projected_exhaustion");
    }

    #[test]
    fn exhausting_current_switches_to_an_alternative_that_lasts_longer() {
        // Caso real: Claude 51% de sesión a mitad de ventana y 91% semanal
        // (se agota en ~1h); Codex con la sesión libre y 59% semanal también
        // proyecta agotarse antes del reset semanal, pero días más tarde. La
        // histéresis (bonus del actual + SWITCH_MARGIN) no puede dejar al
        // usuario en el proveedor que se acaba primero.
        // Reset de sesión en ~3,6 h de 5 h: 51% gastado en 1,4 h proyecta
        // agotarse ~1h antes del reset (mismo cuadro que el caché real).
        let mut claude = snap_window(VendorId::Anthropic, 51.0, 91.0, 13_000);
        claude.id = "anthropic".into();
        claude.name = "Claude Code".into();
        let mut codex = snap_window(VendorId::Openai, 0.0, 59.0, 5 * 3_600);
        codex.id = "openai".into();
        codex.name = "Codex".into();
        let rec = recommend(&[claude, codex], "anthropic", now());
        assert_eq!(rec.action, "switch", "candidates: {:?}", rec.candidates);
        assert_eq!(rec.to_id.as_deref(), Some("openai"));
    }

    #[test]
    fn critical_weekly_quota_outranks_session_projection() {
        let mut claude = snap_window(VendorId::Anthropic, 44.0, 99.0, 3_180);
        claude.id = "anthropic".into();
        claude.name = "Claude Code".into();
        // Match the reported case: weekly reset is still more than a day away.
        claude.quotas[1].reset_at = Some(rfc(now() + 35 * 3_600));
        claude.quotas[1].reset_in_seconds = Some(35 * 3_600);

        let mut codex = snap_window(VendorId::Openai, 20.0, 40.0, 4 * 3_600);
        codex.id = "openai".into();
        codex.name = "Codex".into();

        let rec = recommend(&[claude, codex], "anthropic", now());
        assert_eq!(rec.severity, "critical");
        assert_eq!(rec.reason, "quota_near_exhaustion");
        assert_eq!(rec.action, "switch");
        assert_eq!(rec.to_id.as_deref(), Some("openai"));
        let limiting = rec.limiting_quota.expect("limiting quota");
        assert_eq!(limiting.id, "weekly");
        assert_eq!(limiting.used_percent, 99.0);
        assert_eq!(limiting.available_percent, 1.0);
    }

    #[test]
    fn hysteresis_prevents_flip_on_tie() {
        // 74 vs 76 sin riesgo: quedarse.
        let mut a = snap_window(VendorId::Anthropic, 26.0, 20.0, 3 * 3_600);
        a.id = "anthropic".into();
        a.name = "Claude Code".into();
        let mut b = snap_window(VendorId::Openai, 24.0, 20.0, 3 * 3_600);
        b.id = "openai".into();
        b.name = "Codex".into();
        let rec = recommend(&[a, b], "anthropic", now());
        assert_ne!(rec.action, "switch", "candidates: {:?}", rec.candidates);
    }

    #[test]
    fn big_gap_switches() {
        // Current 48 pts de score aprox vs alternativa muy libre → switch.
        let mut cur = snap_window(VendorId::Anthropic, 80.0, 70.0, 5 * 3_600);
        cur.id = "anthropic".into();
        cur.name = "Claude Code".into();
        let mut alt = snap_window(VendorId::Openai, 10.0, 10.0, 5 * 3_600);
        alt.id = "openai".into();
        alt.name = "Codex".into();
        let rec = recommend(&[cur, alt], "anthropic", now());
        assert_eq!(rec.action, "switch");
        assert_eq!(rec.to_id.as_deref(), Some("openai"));
    }

    #[test]
    fn offline_and_auth_excluded() {
        let mut ok = snap_window(VendorId::Anthropic, 50.0, 40.0, 3 * 3_600);
        ok.id = "anthropic".into();
        ok.name = "Claude Code".into();
        let mut bad = snapshot_with_status(
            VendorId::Openai,
            ProviderStatus::NeedsAuth,
            ProviderStatusReason::InvalidCredential,
            "bad key",
        );
        bad.id = "openai".into();
        bad.name = "Codex".into();
        let rec = recommend(&[ok, bad], "anthropic", now());
        assert_eq!(rec.action, "stay");
        let codex = rec.candidates.iter().find(|c| c.id == "openai").unwrap();
        assert_eq!(codex.excluded.as_deref(), Some("auth"));
    }

    #[test]
    fn current_auth_problem_recommends_a_usable_alternative() {
        let mut claude = snapshot_with_status(
            VendorId::Anthropic,
            ProviderStatus::NeedsAuth,
            ProviderStatusReason::OAuthExpired,
            "expired",
        );
        claude.id = "anthropic".into();
        claude.name = "Claude Code".into();
        let mut codex = snap_window(VendorId::Openai, 20.0, 20.0, 4 * 3_600);
        codex.id = "openai".into();
        codex.name = "Codex".into();

        let rec = recommend(&[claude, codex], "anthropic", now());
        assert_eq!(rec.action, "switch");
        assert_eq!(rec.reason, "auth_problem");
        assert_eq!(rec.severity, "warning");
        assert_eq!(rec.to_id.as_deref(), Some("openai"));
    }

    #[test]
    fn service_outage_is_critical_when_no_quota_signal_is_worse() {
        let mut claude = snap_window(VendorId::Anthropic, 20.0, 20.0, 4 * 3_600);
        claude.id = "anthropic".into();
        claude.name = "Claude Code".into();
        claude.service = ServiceHealth::Outage;
        let mut codex = snap_window(VendorId::Openai, 20.0, 20.0, 4 * 3_600);
        codex.id = "openai".into();
        codex.name = "Codex".into();

        let rec = recommend(&[claude, codex], "anthropic", now());
        assert_eq!(rec.reason, "service_outage");
        assert_eq!(rec.severity, "critical");
        assert_eq!(rec.action, "switch");
    }

    #[test]
    fn stale_quota_keeps_absolute_critical_but_not_a_new_projection() {
        let mut claude = snap_window(VendorId::Anthropic, 88.0, 20.0, 3_600);
        claude.id = "anthropic".into();
        claude.mark_stale();
        let rec = recommend(&[claude], "anthropic", now());
        assert_eq!(rec.severity, "healthy");
        assert_eq!(rec.reason, "sustainable");
        assert!(rec.limiting_quota.is_none());
    }

    #[test]
    fn no_valid_data_reports_insufficient() {
        let a = snapshot_with_status(
            VendorId::Anthropic,
            ProviderStatus::NeedsAuth,
            ProviderStatusReason::MissingCredential,
            "nope",
        );
        let b = snapshot_with_status(
            VendorId::Openai,
            ProviderStatus::Error,
            ProviderStatusReason::Unknown,
            "nope",
        );
        let rec = recommend(&[a, b], "anthropic", now());
        assert_eq!(rec.action, "insufficient_data");
    }

    #[test]
    fn partial_limited_provider_yields_warning_with_penalty_and_does_not_exclude() {
        let mut cursor = snap_window(VendorId::Cursor, 38.0, 38.0, 2_592_000);
        cursor.id = "cursor".into();
        cursor.name = "Cursor".into();
        cursor.availability = Availability::PartialLimited;
        cursor.quotas[0].id = "cursor_models".into();
        cursor.quotas[0].label = "Cursor Models".into();
        cursor.quotas.push(UsageQuota {
            id: "other_models".into(),
            label: "Other Models".into(),
            window_type: WindowType::Monthly,
            used_percent: Some(100.0),
            remaining_percent: Some(0.0),
            used_amount: None,
            limit_amount: None,
            unit: Some(crate::model::UsageUnit::Percent),
            reset_at: None,
            reset_in_seconds: None,
            reset_status: crate::model::ResetStatus::NotProvided,
            temporary_multiplier: None,
            temporary_expires_at: None,
            source: crate::model::UsageSource::LocalSession,
            fetched_at: String::new(),
            stale: false,
            confidence: DataConfidence::Exact,
            pace: None,
            group_id: Some("other_models".into()),
            group_label: Some("Other Models".into()),
            models: Vec::new(),
            visible: "always".into(),
        });

        let rec = recommend(&[cursor], "cursor", now());
        assert_eq!(rec.severity, "warning");
        assert_eq!(rec.reason, "partial_limited");
        let candidate = rec.candidates.iter().find(|c| c.id == "cursor").unwrap();
        assert!(candidate.excluded.is_none());
        assert_eq!(candidate.severity, "warning");
        assert_eq!(candidate.signal_reason, "partial_limited");
        assert!(candidate.score > 0.0);
    }
}
