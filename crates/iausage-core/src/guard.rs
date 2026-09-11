//! `iausage guard`: puerta de orquestación de agentes.
//!
//! ```powershell
//! iausage guard --provider claude --min-remaining 15
//! if ($LASTEXITCODE -ne 0) { # mandar el trabajo a Codex }
//! ```
//!
//! Códigos estables: `0` suficiente, `1` bajo el límite, `64` argumentos
//! incorrectos, `69` provider no disponible. La GUI no usa esto; es API de
//! automatización y su contrato no se rompe.

use crate::model::{ProviderSnapshot, WindowType};

pub const EXIT_OK: i32 = 0;
/// Bajo el límite pedido.
pub const EXIT_BELOW: i32 = 1;
/// Argumentos incorrectos (estilo sysexits EX_USAGE).
pub const EXIT_USAGE: i32 = 64;
/// Provider no disponible / sin datos (estilo EX_UNAVAILABLE).
pub const EXIT_UNAVAILABLE: i32 = 69;

/// % restante para una ventana (`session`, `5h`, `daily`, `weekly`,
/// `monthly`, `credits`). `None` = sin ventana pedida → usa el primario
/// (menor restante entre quotas) o `primary_utilization`.
pub fn remaining_for(snapshot: &ProviderSnapshot, window: Option<&str>) -> Option<f64> {
    if let Some(w) = window.map(str::to_ascii_lowercase) {
        let want = match w.as_str() {
            "session" => Some(WindowType::Session),
            "5h" | "5h-window" | "five_hour" => Some(WindowType::FiveHour),
            "daily" => Some(WindowType::Daily),
            "weekly" => Some(WindowType::Weekly),
            "monthly" => Some(WindowType::Monthly),
            "credits" => Some(WindowType::Credits),
            _ => None,
        };
        if let Some(want) = want {
            let best = snapshot
                .quotas
                .iter()
                .filter(|q| q.window_type == want)
                .filter_map(|q| q.remaining_percent)
                .fold(None::<f64>, |acc, r| Some(acc.map_or(r, |a: f64| a.min(r))));
            if best.is_some() {
                return best;
            }
            if want == WindowType::Credits {
                return snapshot.credits.as_ref().map(|_| 100.0);
            }
            return None;
        }
        // Id de quota literal como fallback.
        if let Some(q) = snapshot.quotas.iter().find(|q| q.id == w) {
            return q.remaining_percent;
        }
        return None;
    }
    snapshot
        .quotas
        .iter()
        .filter_map(|q| q.remaining_percent)
        .fold(None::<f64>, |acc, r| {
            Some(acc.map_or(r, |a: f64| a.min(r)))
        })
        .or_else(|| snapshot.primary_utilization.map(|u| (100.0 - u).max(0.0)))
}

pub enum GuardVerdict {
    Ok { remaining: f64 },
    Below { remaining: f64, min: f64 },
    Unavailable { reason: &'static str },
}

/// Evalúa sin efectos secundarios; el binario traduce a exit codes.
pub fn evaluate(
    snapshot: Option<&ProviderSnapshot>,
    window: Option<&str>,
    min_remaining: f64,
) -> GuardVerdict {
    let Some(snap) = snapshot else {
        return GuardVerdict::Unavailable { reason: "no-data" };
    };
    if !snap.is_connected() {
        return GuardVerdict::Unavailable { reason: "not-connected" };
    }
    match remaining_for(snap, window) {
        None => GuardVerdict::Unavailable {
            reason: "no-window",
        },
        Some(remaining) if remaining >= min_remaining => GuardVerdict::Ok { remaining },
        Some(remaining) => GuardVerdict::Below {
            remaining,
            min: min_remaining,
        },
    }
}

pub fn exit_code(verdict: &GuardVerdict) -> i32 {
    match verdict {
        GuardVerdict::Ok { .. } => EXIT_OK,
        GuardVerdict::Below { .. } => EXIT_BELOW,
        GuardVerdict::Unavailable { .. } => EXIT_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok, VendorId};

    fn snap() -> ProviderSnapshot {
        snapshot_ok(
            VendorId::Anthropic,
            "Max",
            vec![
                progress_pct("session", "Sesión", 88.0, None, 18_000, "always"),
                progress_pct("weekly", "Semanal", 40.0, None, 604_800, "always"),
            ],
        )
    }

    #[test]
    fn picks_named_window() {
        let s = snap();
        assert_eq!(remaining_for(&s, Some("session")).unwrap().round(), 12.0);
        assert_eq!(remaining_for(&s, Some("weekly")).unwrap().round(), 60.0);
    }

    #[test]
    fn defaults_to_worst_remaining() {
        let s = snap();
        assert_eq!(remaining_for(&s, None).unwrap().round(), 12.0);
    }

    #[test]
    fn verdicts_map_to_stable_codes() {
        let s = snap();
        assert_eq!(
            exit_code(&evaluate(Some(&s), Some("session"), 15.0)),
            EXIT_BELOW
        );
        assert_eq!(exit_code(&evaluate(Some(&s), Some("session"), 10.0)), EXIT_OK);
        assert_eq!(exit_code(&evaluate(None, None, 10.0)), EXIT_UNAVAILABLE);
    }
}
