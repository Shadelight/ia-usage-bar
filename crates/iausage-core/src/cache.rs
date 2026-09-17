//! Last-known-good provider snapshots used for an immediate, honest startup.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::atomic_write;
use crate::model::ProviderSnapshot;
use crate::paths::snapshot_cache_path;

fn load_from(path: &Path) -> HashMap<String, ProviderSnapshot> {
    let mut snapshots: HashMap<String, ProviderSnapshot> = fs::read_to_string(path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default();
    snapshots.retain(|_, snapshot| !snapshot.quotas.is_empty() || !snapshot.lines.is_empty());
    for snapshot in snapshots.values_mut() {
        snapshot.mark_stale();
    }
    snapshots
}

pub fn load() -> HashMap<String, ProviderSnapshot> {
    load_from(&snapshot_cache_path())
}

pub fn save_valid(snapshot: &ProviderSnapshot) -> Result<(), String> {
    if !snapshot.is_connected() || (snapshot.quotas.is_empty() && snapshot.lines.is_empty()) {
        return Ok(());
    }
    let path = snapshot_cache_path();
    let mut snapshots = load_from(&path);
    snapshots.insert(snapshot.id.clone(), snapshot.clone());
    let body = serde_json::to_vec(&snapshots).map_err(|error| error.to_string())?;
    atomic_write(&path, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{progress_pct, snapshot_ok, VendorId};

    #[test]
    fn cached_snapshot_is_loaded_as_stale() {
        let path = std::env::temp_dir().join(format!(
            "iausagebar-cache-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let snapshot = snapshot_ok(
            VendorId::Anthropic,
            "Pro",
            vec![progress_pct(
                "session", "Sesión", 12.0, None, 18_000, "always",
            )],
        );
        let body = serde_json::to_vec(&HashMap::from([(snapshot.id.clone(), snapshot)])).unwrap();
        fs::write(&path, body).unwrap();

        let loaded = load_from(&path);
        assert!(loaded["anthropic"].stale);
        assert!(loaded["anthropic"].quotas.iter().all(|quota| quota.stale));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn cache_is_namespaced_by_provider_id() {
        let path = std::env::temp_dir().join(format!(
            "iausagebar-cache-ns-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let antigravity = snapshot_ok(
            VendorId::Antigravity,
            "Free",
            vec![progress_pct("5h", "5h", 10.0, None, 18_000, "always")],
        );
        let cursor = snapshot_ok(
            VendorId::Cursor,
            "Pro",
            vec![progress_pct(
                "monthly", "Mensual", 50.0, None, 2_592_000, "always",
            )],
        );
        let body = serde_json::to_vec(&HashMap::from([
            (antigravity.id.clone(), antigravity),
            (cursor.id.clone(), cursor),
        ]))
        .unwrap();
        fs::write(&path, body).unwrap();

        let loaded = load_from(&path);
        assert_eq!(loaded["cursor"].id, "cursor");
        assert_eq!(loaded["cursor"].plan, "Pro");
        assert_eq!(loaded["antigravity"].id, "antigravity");
        assert_eq!(loaded["antigravity"].plan, "Free");
        let _ = fs::remove_file(path);
    }
}
