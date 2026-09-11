//! Modelo formal de ritmo (pace/burn rate).
//!
//! Compara **% real usado** contra **% esperado según el tiempo
//! transcurrido de la ventana** y decide si el límite sobrevivirá al reset.
//! Misma fórmula que usa el frontend (`dash.ts`); aquí vive la versión
//! canónica en Rust para CLI/`guard`/tests.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePace {
    pub expected_used_percent: f64,
    pub actual_used_percent: f64,
    pub delta_percent: f64,
    /// `None` cuando no hay datos suficientes para proyectar.
    pub will_last_to_reset: Option<bool>,
    /// ISO-8601 estimado de agotamiento al ritmo actual (si aplica).
    pub estimated_exhausted_at: Option<String>,
}

/// Calcula el pace. `window_secs` es la duración de la ventana,
/// `reset_at_unix` el reset observado, `now_unix` el ahora.
/// Devuelve `None` si la ventana es degenerada o el uso es insignificante
/// (<8% igual que el umbral de la UI: evita ruido con datos recién abiertos).
pub fn compute_pace(
    used_percent: f64,
    window_secs: i64,
    reset_at_unix: i64,
    now_unix: i64,
) -> Option<UsagePace> {
    if window_secs <= 0 || !(0.0..=100.0).contains(&used_percent) {
        return None;
    }
    let elapsed = (window_secs - (reset_at_unix - now_unix)).max(0);
    let fraction = (elapsed as f64 / window_secs as f64).clamp(0.0, 1.0);
    if fraction < 0.05 || used_percent < 8.0 {
        return None;
    }
    let expected = fraction * 100.0;
    let delta = used_percent - expected;

    let elapsed = elapsed.max(1) as f64;
    let rate_per_sec = used_percent / elapsed;
    let (will_last, exhausted_at) = if rate_per_sec <= 0.0 {
        (None, None)
    } else {
        let remaining_secs = (100.0 - used_percent) / rate_per_sec;
        let exhaust_unix = now_unix + remaining_secs.round() as i64;
        let lasts = exhaust_unix >= reset_at_unix;
        let at = (!lasts).then(|| {
            chrono::DateTime::from_timestamp(exhaust_unix, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        });
        (Some(lasts), at.filter(|s| !s.is_empty()))
    };

    Some(UsagePace {
        expected_used_percent: expected,
        actual_used_percent: used_percent,
        delta_percent: delta,
        will_last_to_reset: will_last,
        estimated_exhausted_at: exhausted_at,
    })
}

/// Duración canónica por tipo de ventana (la UI usa la misma tabla).
pub fn window_secs_for(window_type: &str) -> Option<i64> {
    match window_type {
        "session" | "5h" => Some(5 * 3_600),
        "daily" => Some(86_400),
        "weekly" => Some(7 * 86_400),
        "monthly" => Some(30 * 86_400),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_track_usage_will_last() {
        // Ventana 5h, a mitad (2.5h transcurridas), 40% usado vs 50% esperado.
        let pace = compute_pace(40.0, 18_000, 1_000_000 + 9_000, 1_000_000).unwrap();
        assert!((pace.expected_used_percent - 50.0).abs() < 0.01);
        assert!((pace.delta_percent + 10.0).abs() < 0.01);
        assert_eq!(pace.will_last_to_reset, Some(true));
        assert!(pace.estimated_exhausted_at.is_none());
    }

    #[test]
    fn over_pace_projects_exhaustion_before_reset() {
        // 88% con solo 72% de la ventana transcurrida: no llega.
        let window = 18_000i64;
        let now = 1_000_000i64;
        let elapsed = (0.72 * window as f64) as i64;
        let reset = now + (window - elapsed);
        let pace = compute_pace(88.0, window, reset, now).unwrap();
        assert!((pace.expected_used_percent - 72.0).abs() < 1.0);
        assert!(pace.delta_percent > 10.0);
        assert_eq!(pace.will_last_to_reset, Some(false));
        assert!(pace.estimated_exhausted_at.is_some());
    }

    #[test]
    fn tiny_or_fresh_windows_are_silent() {
        assert!(compute_pace(3.0, 18_000, 1_018_000, 1_000_000).is_none());
        assert!(compute_pace(50.0, 0, 1_000_000, 1_000_000).is_none());
        assert!(compute_pace(120.0, 18_000, 1_018_000, 1_000_000).is_none());
    }
}
