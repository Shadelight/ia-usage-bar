//! Estado compartido de la app y helpers de bloqueo.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use tauri::{
    menu::{CheckMenuItem, MenuItem, Submenu},
    Wry,
};

use crate::config::AppConfig;
use crate::model::{ProviderSnapshot, VendorInfo};

pub(crate) struct AppState {
    pub(crate) snapshots: Mutex<HashMap<String, ProviderSnapshot>>,
    pub(crate) config: Mutex<AppConfig>,
    /// Credential detection is intentionally cached: building the dashboard
    /// must be side-effect free and must not probe every installed CLI.
    pub(crate) catalog: Mutex<Vec<VendorInfo>>,
    pub(crate) notify: Mutex<HashMap<String, NotifyState>>,
    pub(crate) notifications_enabled: AtomicBool,
    pub(crate) allow_exit: AtomicBool,
    pub(crate) last_refresh: Mutex<Option<Instant>>,
    /// Última interacción del usuario (dashboard/refresh/tray). Alimenta la
    /// política de refresh adaptativo (`refresh_policy`).
    pub(crate) last_activity: Mutex<Instant>,
    pub(crate) loading_providers: Mutex<HashSet<String>>,
    pub(crate) app_bootstrapping: AtomicBool,
    /// True while a refresh fan-out is in flight (F-H3 overlap guard).
    pub(crate) refreshing: AtomicBool,
    /// Set when a refresh was requested while `refreshing` was held; consumed
    /// as a single coalesced rerun when the in-flight refresh finishes.
    pub(crate) rerun_requested: AtomicBool,
    pub(crate) backoff_until: Mutex<HashMap<String, Instant>>,
}

pub(crate) struct TrayMenuState {
    pub(crate) header: MenuItem<Wry>,
    pub(crate) autostart: CheckMenuItem<Wry>,
    pub(crate) always_on_top: CheckMenuItem<Wry>,
    pub(crate) compact_mode: CheckMenuItem<Wry>,
    pub(crate) primary_submenu: Submenu<Wry>,
}

#[derive(Default, Clone)]
pub(crate) struct NotifyState {
    pub(crate) initialized: bool,
    pub(crate) prev_resets: Option<String>,
    pub(crate) previous_utilization: Option<f64>,
    pub(crate) notified: HashSet<u8>,
}

/// Lock a mutex, tolerating poisoning instead of cascading the panic. A single
/// transient panic while a lock is held must not permanently kill refreshing.
pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// F-H3 overlap guard: returns `true` for exactly one caller at a time. The
/// winner must `flag.store(false, …)` when its refresh finishes.
pub(crate) fn claim_refresh(flag: &AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // F-H3: two threads racing the refresh flag — exactly one wins the claim,
    // the loser is turned away (it will set the rerun flag instead).
    #[test]
    fn claim_refresh_admits_exactly_one() {
        let flag = Arc::new(AtomicBool::new(false));
        let a = flag.clone();
        let b = flag.clone();
        let ha = std::thread::spawn(move || claim_refresh(&a));
        let hb = std::thread::spawn(move || claim_refresh(&b));
        let (ra, rb) = (ha.join().unwrap(), hb.join().unwrap());
        assert!(ra ^ rb, "exactly one thread claims the refresh");
        assert!(!claim_refresh(&flag), "flag stays held until released");
        flag.store(false, Ordering::Release);
        assert!(claim_refresh(&flag));
    }

    // F-H4: a mutex poisoned by a panicking thread must still be usable.
    #[test]
    fn lock_or_recover_tolerates_poison() {
        let m = Arc::new(Mutex::new(7));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("poison the mutex");
        })
        .join();
        assert!(m.lock().is_err(), "mutex is poisoned");
        assert_eq!(*lock_or_recover(&m), 7);
    }
}
