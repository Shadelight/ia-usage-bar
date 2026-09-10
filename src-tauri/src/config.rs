//! Configuración persistente (`%APPDATA%\ia-usagebar\config.toml`).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::VendorId;
use crate::paths::{app_config_dir, config_path, detect_path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_refresh")]
    pub refresh_minutes: u64,
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default = "default_true")]
    pub notifications: bool,
    #[serde(default = "default_used")]
    pub show_usage_as: String,
    #[serde(default = "default_countdown")]
    pub reset_times: String,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// True when `load()` could not parse `config.toml` and fell back to
    /// defaults after backing the bad file aside. Never persisted; callers
    /// must not overwrite `config.toml` while this is set.
    #[serde(skip)]
    pub load_recovered: bool,
}

fn default_refresh() -> u64 {
    5
}
fn default_primary() -> String {
    "anthropic".into()
}
fn default_true() -> bool {
    true
}
fn default_used() -> String {
    "used".into()
}
fn default_countdown() -> String {
    "countdown".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_lines: Option<Vec<String>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_minutes: 5,
            primary: "anthropic".into(),
            notifications: true,
            show_usage_as: "used".into(),
            reset_times: "countdown".into(),
            providers: HashMap::new(),
            load_recovered: false,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    fn load_from(path: &Path) -> Self {
        let mut cfg: AppConfig = match Self::try_load(path) {
            Ok(Some(cfg)) => cfg,
            Ok(None) => AppConfig::default(),
            Err(_) => {
                // A config.toml that exists but will not parse is NOT an
                // invitation to overwrite it with defaults (that would destroy
                // every stored API key). Back it up and flag the recovery so
                // no caller re-saves over the backup.
                back_up_corrupt(path);
                AppConfig {
                    load_recovered: true,
                    ..AppConfig::default()
                }
            }
        };
        if cfg.refresh_minutes == 0 {
            cfg.refresh_minutes = 5;
        }
        cfg.refresh_minutes = match cfg.refresh_minutes {
            1 | 5 | 10 => cfg.refresh_minutes,
            n if n < 3 => 1,
            n if n < 8 => 5,
            _ => 10,
        };
        cfg
    }

    /// `Ok(None)` = file absent (a normal first run). `Ok(Some)` = parsed.
    /// `Err` = the file exists but is unreadable or unparseable.
    fn try_load(path: &Path) -> Result<Option<Self>, String> {
        let raw = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        toml::from_str(&raw).map(Some).map_err(|e| e.to_string())
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = app_config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let body = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(config_path(), body).map_err(|e| e.to_string())
    }

    pub fn provider(&self, id: VendorId) -> ProviderConfig {
        self.providers.get(id.slug()).cloned().unwrap_or_default()
    }

    pub fn is_enabled(&self, id: VendorId) -> bool {
        self.provider(id).enabled
    }

    pub fn set_enabled(&mut self, id: VendorId, enabled: bool) {
        self.providers.entry(id.slug().to_string()).or_default().enabled = enabled;
    }

    pub fn api_key(&self, id: VendorId) -> Option<String> {
        if let Some(env) = id.env_key() {
            if let Ok(v) = std::env::var(env) {
                if !v.trim().is_empty() {
                    return Some(v);
                }
            }
        }
        self.provider(id)
            .api_key
            .filter(|s| !s.trim().is_empty())
    }

    pub fn enabled_ids(&self) -> Vec<VendorId> {
        VendorId::all()
            .iter()
            .copied()
            .filter(|id| self.is_enabled(*id))
            .collect()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DetectState {
    #[serde(default)]
    known: HashSet<String>,
}

pub fn run_detect(cfg: &mut AppConfig) -> Vec<String> {
    let path = detect_path();
    let mut state: DetectState = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut newly = Vec::new();
    for id in VendorId::all() {
        let slug = id.slug().to_string();
        if state.known.contains(&slug) {
            continue;
        }
        state.known.insert(slug.clone());
        if crate::providers::has_local_credentials(*id, cfg) {
            cfg.set_enabled(*id, true);
            newly.push(slug);
        }
    }
    let _ = save_detect(&path, &state);
    if !cfg.load_recovered {
        let _ = cfg.save();
    }
    newly
}

/// Rename an unparseable `config.toml` to `config.toml.bak-<unix-ts>` so its
/// contents (API keys included) stay recoverable instead of being overwritten.
fn back_up_corrupt(path: &Path) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bak = path.with_extension(format!("toml.bak-{ts}"));
    let _ = fs::rename(path, bak);
}

fn save_detect(path: &Path, state: &DetectState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cfg-test-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // F-H2: an unparseable config.toml must not be silently replaced by
    // defaults — try_load reports the error and load_from backs the file up
    // (keys recoverable) and flags the recovery so nothing re-saves over it.
    #[test]
    fn corrupt_config_is_backed_up_not_destroyed() {
        let dir = scratch_dir("corrupt");
        let path = dir.join("config.toml");
        fs::write(&path, "not = valid = toml = {{{").unwrap();

        assert!(AppConfig::try_load(&path).is_err());

        let cfg = AppConfig::load_from(&path);
        assert!(cfg.load_recovered);
        assert!(!path.exists(), "bad file should have been renamed aside");

        let baks: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("toml.bak-"))
            .collect();
        assert_eq!(baks.len(), 1, "exactly one backup");
        let recovered = fs::read_to_string(baks[0].path()).unwrap();
        assert!(recovered.contains("not = valid = toml"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn absent_config_is_a_normal_default_load() {
        let dir = scratch_dir("absent");
        let path = dir.join("config.toml");
        assert!(matches!(AppConfig::try_load(&path), Ok(None)));
        let cfg = AppConfig::load_from(&path);
        assert!(!cfg.load_recovered);
        let _ = fs::remove_dir_all(&dir);
    }
}
