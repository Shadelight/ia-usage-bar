//! Construccion del dashboard, ciclo de refresh y notificaciones.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::commands::parse_id;
use crate::model::{most_headroom, monthly_spend, Dashboard, ProviderSnapshot, SpendRow, VendorId};
use crate::providers;
use crate::state::{claim_refresh, lock_or_recover, AppState, NotifyState, UsageAlert};
use crate::tray::update_tray_from_dashboard;

pub(crate) fn build_dashboard(_app: &AppHandle, state: &AppState) -> Dashboard {
    let cfg = lock_or_recover(&state.config).clone();
    let snaps = lock_or_recover(&state.snapshots);
    let providers: Vec<ProviderSnapshot> = cfg
        .enabled_ids()
        .into_iter()
        .filter_map(|id| snaps.get(id.slug()).cloned())
        .collect();
    let elapsed = lock_or_recover(&state.last_refresh)
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);
    let interval = cfg.refresh_minutes.saturating_mul(60);
    let mut spend = Vec::new();
    let mut spend_month_usd = 0.0;
    for p in &providers {
        if let Some((label, usd)) = monthly_spend(p) {
            spend_month_usd += usd;
            spend.push(SpendRow {
                id: p.id.clone(),
                name: p.name.clone(),
                label,
                usd,
            });
        }
    }
    let (recommend_id, recommend_name, recommend_left) = match most_headroom(&providers) {
        Some((id, name, left)) => (Some(id), Some(name), Some(left)),
        None => (None, None, None),
    };
    Dashboard {
        providers,
        catalog: providers::catalog(&cfg),
        refresh_minutes: cfg.refresh_minutes,
        primary: cfg.primary.clone(),
        notifications: cfg.notifications,
        show_usage_as: cfg.show_usage_as.clone(),
        reset_times: cfg.reset_times.clone(),
        next_update_in_secs: interval.saturating_sub(elapsed),
        spend_month_usd,
        spend,
        recommend_id,
        recommend_name,
        recommend_left,
    }
}

pub(crate) fn emit_dashboard(app: &AppHandle) {
    let dash = build_dashboard(app, &app.state::<AppState>());
    update_tray_from_dashboard(app, &dash);
    let _ = app.emit("dashboard-updated", dash);
}

pub(crate) fn send_notification(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn parse_ts(s: &Option<String>) -> Option<i64> {
    s.as_ref()
        .and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok())
        .map(|d| d.timestamp())
}

fn reset_happened(prev: &Option<String>, cur: &Option<String>) -> bool {
    match (parse_ts(prev), parse_ts(cur)) {
        (Some(p), Some(c)) => (c - p) > 120,
        _ => false,
    }
}

fn take_usage_alerts(util: f64, ns: &mut NotifyState, window_reset: bool) -> Vec<UsageAlert> {
    if window_reset {
        ns.notified_75 = false;
        ns.notified_90 = false;
        ns.notified_limit = false;
    }
    if util < 70.0 {
        ns.notified_75 = false;
        ns.notified_90 = false;
        ns.notified_limit = false;
        return Vec::new();
    }
    let mut out = Vec::new();
    if util >= 75.0 && !ns.notified_75 {
        out.push(UsageAlert::At75);
        ns.notified_75 = true;
    }
    if util >= 90.0 && !ns.notified_90 {
        out.push(UsageAlert::At90);
        ns.notified_90 = true;
    }
    if util >= 95.0 && !ns.notified_limit {
        out.push(UsageAlert::At95);
        ns.notified_limit = true;
    }
    out
}

fn check_notifications(app: &AppHandle, snap: &ProviderSnapshot) {
    let state = app.state::<AppState>();
    if !state.notifications_enabled.load(Ordering::Relaxed) {
        return;
    }
    if !snap.connected {
        return;
    }
    let Some(util) = snap.primary_utilization else {
        return;
    };
    let mut map = lock_or_recover(&state.notify);
    let ns = map.entry(snap.id.clone()).or_default();
    let cur = snap.lines.iter().find_map(|l| l.resets_at().map(|s| s.to_string()));
    if ns.initialized {
        let reset = reset_happened(&ns.prev_resets, &cur);
        if reset {
            send_notification(
                app,
                &format!("{} reiniciado", snap.name),
                "Tu límite se reinició. Listo para seguir.",
            );
        }
        for alert in take_usage_alerts(util, ns, reset) {
            let (title, body) = match alert {
                UsageAlert::At75 => (
                    format!("{} al 75%", snap.name),
                    format!("Vas al {:.0}% de tu cuota. Queda margen, pero ya está cerca.", util),
                ),
                UsageAlert::At90 => (
                    format!("{} al 90%", snap.name),
                    format!("Llegaste al {:.0}%. Conviene cambiar de herramienta antes de cortarte.", util),
                ),
                UsageAlert::At95 => (
                    format!("Límite {}", snap.name),
                    format!("Llegaste al {:.0}% de tu cuota.", util),
                ),
            };
            send_notification(app, &title, &body);
        }
    }
    ns.prev_resets = cur;
    ns.initialized = true;
}

pub(crate) fn do_refresh(app: &AppHandle, only: Option<String>) {
    let a = app.clone();
    std::thread::spawn(move || {
        refresh_sync(&a, only.as_deref());
    });
}

pub(crate) fn refresh_sync(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    // F-H3: run at most one refresh fan-out at a time. A request that lands
    // while one is in flight sets a rerun flag instead of spawning a competing
    // fan-out (which could drive concurrent writes to the same auth.json — F-H1).
    if !claim_refresh(&state.refreshing) {
        if only.is_none() {
            state.rerun_requested.store(true, Ordering::Release);
        }
        return;
    }
    state.rerun_requested.store(false, Ordering::Release);

    // F-H4: a panic on this thread must not kill refreshing permanently.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        refresh_once(app, only);
    }));
    state.refreshing.store(false, Ordering::Release);

    if outcome.is_err() {
        eprintln!("refresh_sync: refresh panicked; marking data stale and continuing");
        for snap in lock_or_recover(&state.snapshots).values_mut() {
            snap.stale = true;
        }
        emit_dashboard(app);
    }

    // Coalesced rerun for whatever asked while we were busy.
    if only.is_none() && state.rerun_requested.swap(false, Ordering::AcqRel) {
        do_refresh(app, None);
    }
}

fn refresh_once(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    let cfg = lock_or_recover(&state.config).clone();
    let ids: Vec<VendorId> = if let Some(only) = only {
        parse_id(only).into_iter().collect()
    } else {
        let enabled = cfg.enabled_ids();
        if enabled.is_empty() {
            VendorId::all().to_vec()
        } else {
            enabled
        }
    };
    let now = Instant::now();
    let backoff = lock_or_recover(&state.backoff_until).clone();
    let mut handles = Vec::new();
    for id in ids {
        if let Some(until) = backoff.get(id.slug()) {
            if now < *until {
                continue;
            }
        }
        let cfg = cfg.clone();
        handles.push(std::thread::spawn(move || (id, providers::refresh(id, &cfg))));
    }
    for h in handles {
        if let Ok((id, snap)) = h.join() {
            let rate_limited = snap
                .error
                .as_deref()
                .is_some_and(|e| e.contains("Límite de peticiones"));
            if rate_limited {
                lock_or_recover(&state.backoff_until).insert(
                    id.slug().to_string(),
                    Instant::now() + Duration::from_secs(300),
                );
            }
            let stored = {
                let mut snaps = lock_or_recover(&state.snapshots);
                if snap.stale {
                    if let Some(prev) = snaps.get_mut(id.slug()) {
                        if !prev.lines.is_empty() {
                            prev.stale = true;
                            prev.error = snap.error.clone();
                            prev.updated_at = snap.updated_at.clone();
                            prev.clone()
                        } else {
                            snaps.insert(id.slug().to_string(), snap.clone());
                            snap
                        }
                    } else {
                        snaps.insert(id.slug().to_string(), snap.clone());
                        snap
                    }
                } else {
                    snaps.insert(id.slug().to_string(), snap.clone());
                    snap
                }
            };
            check_notifications(app, &stored);
        }
    }
    *lock_or_recover(&state.last_refresh) = Some(Instant::now());
    emit_dashboard(app);
}

pub(crate) fn run_loop(app: AppHandle) {
    loop {
        refresh_sync(&app, None);
        let mins = lock_or_recover(&app.state::<AppState>().config)
            .refresh_minutes
            .max(1);
        std::thread::sleep(Duration::from_secs(mins * 60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::NotifyState;

    #[test]
    fn reset_happened_ignores_microsecond_jitter() {
        let a = Some("2099-01-01T00:00:00Z".into());
        let b = Some("2099-01-01T00:00:01Z".into());
        assert!(!reset_happened(&a, &b));
    }

    #[test]
    fn reset_happened_detects_new_window() {
        let a = Some("2099-01-01T00:00:00Z".into());
        let b = Some("2099-01-08T00:00:00Z".into());
        assert!(reset_happened(&a, &b));
    }

    #[test]
    fn reset_happened_ignores_vanished_timestamp() {
        let a = Some("2099-01-01T00:00:00Z".into());
        assert!(!reset_happened(&a, &None));
    }

    #[test]
    fn usage_alerts_fire_at_75_and_90() {
        let mut ns = NotifyState::default();
        assert_eq!(
            take_usage_alerts(76.0, &mut ns, false),
            vec![UsageAlert::At75]
        );
        assert!(take_usage_alerts(80.0, &mut ns, false).is_empty());
        assert_eq!(
            take_usage_alerts(91.0, &mut ns, false),
            vec![UsageAlert::At90]
        );
        assert_eq!(
            take_usage_alerts(96.0, &mut ns, false),
            vec![UsageAlert::At95]
        );
        assert!(take_usage_alerts(50.0, &mut ns, false).is_empty());
        assert!(!ns.notified_75);
        assert_eq!(
            take_usage_alerts(92.0, &mut ns, true),
            vec![UsageAlert::At75, UsageAlert::At90]
        );
    }
}
