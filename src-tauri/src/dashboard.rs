//! Construccion del dashboard, ciclo de refresh y notificaciones.

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

use crate::commands::parse_id;
use crate::model::{
    monthly_spend, most_headroom, now_iso, Dashboard, ProviderSnapshot, ProviderStatus,
    ProviderStatusReason, SpendRow, VendorId,
};
use crate::providers;
use crate::state::{claim_refresh, lock_or_recover, AppState, NotifyState};
use crate::tray::update_tray_from_dashboard;

pub(crate) fn build_dashboard(app: &AppHandle, state: &AppState) -> Dashboard {
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
    let idle_secs = lock_or_recover(&state.last_activity).elapsed().as_secs();
    let interval = crate::refresh_policy::next_interval_secs(&crate::refresh_policy::PolicyInput {
        adaptive: cfg.refresh_adaptive,
        manual_minutes: cfg.refresh_minutes,
        secs_since_activity: idle_secs,
        // TODO(0.3.x): leer el modo de ahorro de batería real de Windows.
        battery_saver: false,
    });
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
        catalog: lock_or_recover(&state.catalog).clone(),
        refresh_minutes: cfg.refresh_minutes,
        refresh_adaptive: cfg.refresh_adaptive,
        primary: cfg.primary.clone(),
        notifications: cfg.notifications,
        notify_thresholds: cfg.notify_thresholds.clone(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        always_on_top: cfg.always_on_top,
        compact_mode: cfg.compact_mode,
        app_bootstrapping: state.app_bootstrapping.load(Ordering::Acquire),
        refreshing: state.refreshing.load(Ordering::Acquire),
        loading_providers: lock_or_recover(&state.loading_providers)
            .iter()
            .cloned()
            .collect(),
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

fn take_usage_alerts(
    util: f64,
    ns: &mut NotifyState,
    window_reset: bool,
    thresholds: &[u8],
) -> Vec<u8> {
    if window_reset {
        ns.notified.clear();
    }
    let previous = if window_reset {
        0.0
    } else {
        ns.previous_utilization.unwrap_or(0.0)
    };
    let mut out = Vec::new();
    for threshold in thresholds.iter().copied() {
        let threshold_value = f64::from(threshold);
        if previous < threshold_value && util >= threshold_value && ns.notified.insert(threshold) {
            out.push(threshold);
        }
    }
    ns.previous_utilization = Some(util);
    out
}

fn check_notifications(app: &AppHandle, snap: &ProviderSnapshot) {
    let state = app.state::<AppState>();
    if !state.notifications_enabled.load(Ordering::Relaxed) {
        return;
    }
    if !snap.is_connected() {
        return;
    }
    let Some(util) = snap.primary_utilization else {
        return;
    };
    let mut map = lock_or_recover(&state.notify);
    let thresholds = lock_or_recover(&state.config).notify_thresholds.clone();
    let ns = map.entry(snap.id.clone()).or_default();
    let cur = snap.quotas.iter().find_map(|quota| quota.reset_at.clone());
    if ns.initialized {
        let reset = reset_happened(&ns.prev_resets, &cur);
        if reset {
            send_notification(
                app,
                &format!("{} reiniciado", snap.name),
                "Tu límite se reinició. Listo para seguir.",
            );
        }
        for threshold in take_usage_alerts(util, ns, reset, &thresholds) {
            let title = if threshold >= 95 {
                format!("Límite {}", snap.name)
            } else {
                format!("{} al {threshold}%", snap.name)
            };
            let body = format!("Llegaste al {:.0}% de tu cuota.", util);
            send_notification(app, &title, &body);
        }
    }
    ns.prev_resets = cur;
    ns.previous_utilization = Some(util);
    ns.initialized = true;
}

pub(crate) fn do_refresh(app: &AppHandle, only: Option<String>) {
    let a = app.clone();
    std::thread::spawn(move || {
        refresh_sync(&a, only.as_deref());
    });
}

/// Aplica una medición nacida de un archivo local (por ejemplo, la última
/// línea `rate_limits` de Codex). No entra por `refresh_sync`: hacerlo allí
/// convertiría cada pulsación de Codex en una petición HTTP.
pub(crate) fn apply_local_snapshot(app: &AppHandle, incoming: ProviderSnapshot) {
    let state = app.state::<AppState>();
    let stored = {
        let mut snapshots = lock_or_recover(&state.snapshots);
        let merged = merge_snapshot(snapshots.get(&incoming.id), incoming);
        snapshots.insert(merged.id.clone(), merged.clone());
        merged
    };
    if stored.is_connected() {
        if let Err(error) = crate::cache::save_valid(&stored) {
            eprintln!(
                "local snapshot cache could not be updated for {}: {error}",
                stored.id
            );
        }
    }
    check_notifications(app, &stored);
    emit_dashboard(app);
}

/// Watcher de fuentes locales de alta frecuencia. El ciclo normal de refresh
/// sigue siendo responsable de APIs y de su backoff; aquí solo se reconstruye
/// Codex desde datos que ya escribió localmente.
pub(crate) fn start_local_watch(app: AppHandle) {
    let root = crate::watch::codex_sessions_dir();
    if !root.is_dir() {
        return;
    }
    std::thread::spawn(move || {
        let result = crate::watch::run_codex_session_watch(
            &root,
            crate::watch::DEFAULT_DEBOUNCE,
            crate::watch::DEFAULT_REMOTE_POLL,
            |tick| {
                if tick == crate::watch::WatchTick::CodexSessionChanged {
                    if let Some(snapshot) = crate::providers::codex_snapshot_from_sessions() {
                        apply_local_snapshot(&app, snapshot);
                    }
                }
            },
        );
        if let Err(error) = result {
            eprintln!("local usage watcher stopped: {error}");
        }
    });
}

pub(crate) fn refresh_sync(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    // F-H3: run at most one refresh fan-out at a time. A request that lands
    // while one is in flight sets a rerun flag instead of spawning a competing
    // fan-out (which could drive concurrent writes to the same auth.json — F-H1).
    if !claim_refresh(&state.refreshing) {
        // A provider-only settings change that lands during a global refresh
        // still needs a follow-up pass: the in-flight pass captured the old
        // config. Coalesce it into one full rerun instead of dropping it.
        state.rerun_requested.store(true, Ordering::Release);
        return;
    }
    state.rerun_requested.store(false, Ordering::Release);

    // F-H4: a panic on this thread must not kill refreshing permanently.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        refresh_once(app, only);
    }));
    state.refreshing.store(false, Ordering::Release);
    state.app_bootstrapping.store(false, Ordering::Release);

    if outcome.is_err() {
        eprintln!("refresh_sync: refresh panicked; marking data stale and continuing");
        lock_or_recover(&state.loading_providers).clear();
        for snap in lock_or_recover(&state.snapshots).values_mut() {
            snap.mark_stale();
        }
        emit_dashboard(app);
    }
    emit_dashboard(app);

    // Coalesced rerun for whatever asked while we were busy.
    if state.rerun_requested.swap(false, Ordering::AcqRel) {
        do_refresh(app, None);
    }
}

fn refresh_once(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    let cfg = lock_or_recover(&state.config).clone();
    let ids: Vec<VendorId> = if let Some(only) = only {
        parse_id(only).into_iter().collect()
    } else {
        cfg.enabled_ids()
    };
    let now = Instant::now();
    let backoff = lock_or_recover(&state.backoff_until).clone();
    let ids: Vec<VendorId> = ids
        .into_iter()
        .filter(|id| backoff.get(id.slug()).is_none_or(|until| now >= *until))
        .collect();
    let attempted_at = now_iso();
    {
        let mut snapshots = lock_or_recover(&state.snapshots);
        for id in &ids {
            if let Some(snapshot) = snapshots.get_mut(id.slug()) {
                snapshot.last_attempt_at = Some(attempted_at.clone());
            }
        }
    }
    {
        let mut loading = lock_or_recover(&state.loading_providers);
        loading.extend(ids.iter().map(|id| id.slug().to_string()));
    }
    emit_dashboard(app);

    let (sender, receiver) = mpsc::channel();
    for id in &ids {
        let id = *id;
        let cfg = cfg.clone();
        let sender = sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send((id, providers::refresh(id, &cfg)));
        });
    }
    drop(sender);

    let started = Instant::now();
    let deadline = started + Duration::from_secs(12);
    let mut pending: HashSet<String> = ids.iter().map(|id| id.slug().to_string()).collect();
    while !pending.is_empty() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Ok((id, snap)) = receiver.recv_timeout(remaining) else {
            break;
        };
        if pending.remove(id.slug()) {
            let rate_limited = snap.status_reason == Some(ProviderStatusReason::RateLimited);
            if rate_limited {
                lock_or_recover(&state.backoff_until).insert(
                    id.slug().to_string(),
                    Instant::now() + Duration::from_secs(300),
                );
            }
            let stored = {
                let mut snaps = lock_or_recover(&state.snapshots);
                let merged = merge_snapshot(snaps.get(id.slug()), snap);
                snaps.insert(id.slug().to_string(), merged.clone());
                merged
            };
            lock_or_recover(&state.loading_providers).remove(id.slug());
            if stored.is_connected() {
                if let Err(error) = crate::cache::save_valid(&stored) {
                    eprintln!(
                        "snapshot cache could not be updated for {}: {error}",
                        stored.id
                    );
                }
            }
            if !stored.is_connected() {
                crate::logfile::append(&format!(
                    "{} status={:?} reason={:?}",
                    stored.id, stored.status, stored.status_reason
                ));
            }
            check_notifications(app, &stored);
            #[cfg(debug_assertions)]
            eprintln!(
                "provider {} completed: {} ms",
                id.slug(),
                started.elapsed().as_millis()
            );
            emit_dashboard(app);
        }
    }

    if !pending.is_empty() {
        for id in pending {
            lock_or_recover(&state.loading_providers).remove(&id);
            if let Some(vendor_id) = parse_id(&id) {
                let timeout = providers::map_fetch_err(
                    vendor_id,
                    crate::http::FetchError::Network("timeout after 12s".into()),
                );
                let mut snapshots = lock_or_recover(&state.snapshots);
                let merged = merge_snapshot(snapshots.get(&id), timeout);
                snapshots.insert(id.clone(), merged);
            }
            crate::logfile::append(&format!("{id} status=timeout after 12s"));
        }
    }
    *lock_or_recover(&state.last_refresh) = Some(Instant::now());
    state.app_bootstrapping.store(false, Ordering::Release);
    emit_dashboard(app);

    // M5: export best-effort del blob cifrado tras cada refresh completo
    // (los parciales por provider no reescriben el snapshot entero).
    if only.is_none() {
        let cfg = lock_or_recover(&state.config).clone();
        if cfg.sync_enabled {
            let snaps = lock_or_recover(&state.snapshots).clone();
            let version = app.package_info().version.to_string();
            match crate::sync::export_current_snapshot(&cfg, &snaps, Some(version)) {
                Ok(_) => {}
                Err(error) => eprintln!("sync auto-export skipped: {error}"),
            }
        }
    }
}

fn merge_snapshot(
    previous: Option<&ProviderSnapshot>,
    mut incoming: ProviderSnapshot,
) -> ProviderSnapshot {
    if incoming.is_connected() {
        return incoming;
    }

    if let Some(previous) = previous.filter(|snapshot| {
        snapshot.is_connected() && (!snapshot.lines.is_empty() || !snapshot.quotas.is_empty())
    }) {
        let rate_limited = incoming.status_reason == Some(ProviderStatusReason::RateLimited);
        let mut retained = previous.clone();
        retained.mark_stale();
        retained.last_attempt_at = incoming.last_attempt_at.take();
        retained.status_reason = incoming.status_reason;
        retained.error = incoming.error.take();
        if !rate_limited {
            retained.status = incoming.status;
            retained.hint = incoming.hint;
        }
        return retained;
    }

    if incoming.status_reason == Some(ProviderStatusReason::RateLimited) {
        incoming.status = ProviderStatus::Error;
        incoming.status_reason = Some(ProviderStatusReason::Unknown);
        incoming.stale = false;
    }
    incoming
}

pub(crate) fn run_loop(app: AppHandle) {
    loop {
        refresh_sync(&app, None);
        let state = app.state::<AppState>();
        let (adaptive, manual, idle) = {
            let cfg = lock_or_recover(&state.config);
            let idle = lock_or_recover(&state.last_activity).elapsed().as_secs();
            (cfg.refresh_adaptive, cfg.refresh_minutes, idle)
        };
        let wait = crate::refresh_policy::next_interval_secs(&crate::refresh_policy::PolicyInput {
            adaptive,
            manual_minutes: manual,
            secs_since_activity: idle,
            // TODO(0.3.x): leer el modo de ahorro de batería real de Windows.
            battery_saver: false,
        })
        .max(60);
        std::thread::sleep(Duration::from_secs(wait));
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
            take_usage_alerts(76.0, &mut ns, false, &[75, 90, 95]),
            vec![75]
        );
        assert!(take_usage_alerts(80.0, &mut ns, false, &[75, 90, 95]).is_empty());
        assert_eq!(
            take_usage_alerts(91.0, &mut ns, false, &[75, 90, 95]),
            vec![90]
        );
        assert_eq!(
            take_usage_alerts(96.0, &mut ns, false, &[75, 90, 95]),
            vec![95]
        );
        assert!(take_usage_alerts(50.0, &mut ns, false, &[75, 90, 95]).is_empty());
        assert!(ns.notified.contains(&75));
        assert_eq!(
            take_usage_alerts(92.0, &mut ns, true, &[75, 90, 95]),
            vec![75, 90]
        );
    }

    #[test]
    fn custom_thresholds_fire_once_per_window() {
        let mut ns = NotifyState::default();
        assert_eq!(take_usage_alerts(85.0, &mut ns, false, &[80, 95]), vec![80]);
        assert!(take_usage_alerts(90.0, &mut ns, false, &[80, 95]).is_empty());
        assert_eq!(take_usage_alerts(96.0, &mut ns, false, &[80, 95]), vec![95]);
    }

    #[test]
    fn transient_failure_retains_metrics_but_exposes_failure_status() {
        let previous = crate::model::snapshot_ok(
            VendorId::Anthropic,
            "Pro",
            vec![crate::model::progress_pct(
                "session", "Sesión", 25.0, None, 18_000, "always",
            )],
        );
        let incoming = providers::map_fetch_err(
            VendorId::Anthropic,
            crate::http::FetchError::Network("offline".into()),
        );
        let merged = merge_snapshot(Some(&previous), incoming);
        assert_eq!(merged.status, ProviderStatus::Unavailable);
        assert!(merged.stale);
        assert_eq!(merged.lines.len(), 1);
    }

    #[test]
    fn rate_limit_retains_previous_good_snapshot_as_stale() {
        let previous = crate::model::snapshot_ok(
            VendorId::Anthropic,
            "Pro",
            vec![crate::model::progress_pct(
                "session", "Sesión", 25.0, None, 18_000, "always",
            )],
        );
        let incoming =
            providers::map_fetch_err(VendorId::Anthropic, crate::http::FetchError::RateLimited);
        let merged = merge_snapshot(Some(&previous), incoming);

        assert_eq!(merged.status, ProviderStatus::Connected);
        assert_eq!(
            merged.status_reason,
            Some(ProviderStatusReason::RateLimited)
        );
        assert!(merged.stale);
        assert_eq!(merged.lines.len(), 1);
    }

    #[test]
    fn rate_limit_without_previous_data_is_generic_error() {
        let incoming =
            providers::map_fetch_err(VendorId::Anthropic, crate::http::FetchError::RateLimited);
        let merged = merge_snapshot(None, incoming);

        assert_eq!(merged.status, ProviderStatus::Error);
        assert_eq!(merged.status_reason, Some(ProviderStatusReason::Unknown));
        assert!(!merged.stale);
    }
}
