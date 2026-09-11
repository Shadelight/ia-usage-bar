//! Rutas de usuario (Windows-first).

use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| {
        // Never reinterpret a missing profile as the process working
        // directory: provider code may write refreshed credentials.
        eprintln!("Windows user profile directory is unavailable");
        std::env::temp_dir().join("iausagebar-missing-home")
    })
}

pub fn claude_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("CLAUDE_CONFIG_DIR") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    home_dir().join(".claude")
}

pub fn app_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(home_dir)
        .join("ia-usagebar")
}

pub fn config_path() -> PathBuf {
    app_config_dir().join("config.toml")
}

pub fn detect_path() -> PathBuf {
    app_config_dir().join("detect.json")
}
