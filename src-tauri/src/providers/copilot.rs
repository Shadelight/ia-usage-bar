//! GitHub Copilot — `gh auth token` o GITHUB_COPILOT_TOKEN.

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;

use serde_json::Value;

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    badge_line, json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok,
    ProviderSnapshot, VendorId,
};
use crate::paths::home_dir;

use super::Provider;

const USER_URL: &str = "https://api.github.com/copilot_internal/user";

pub struct Copilot;

impl Provider for Copilot {
    fn id(&self) -> VendorId {
        VendorId::Copilot
    }

    fn has_local_credentials(&self, cfg: &AppConfig) -> bool {
        cfg.api_key(VendorId::Copilot).is_some() || hosts_yml().exists()
    }

    fn refresh(&self, cfg: &AppConfig) -> ProviderSnapshot {
        let Some(token) = resolve_token(cfg) else {
            return snapshot_needs_auth(VendorId::Copilot, VendorId::Copilot.login_hint());
        };
        match fetch_user(&token) {
            Ok(body) => snapshot_from_json(&body),
            Err(e) => super::map_fetch_err(VendorId::Copilot, e),
        }
    }
}

fn hosts_yml() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("GH_CONFIG_DIR") {
        return std::path::PathBuf::from(dir).join("hosts.yml");
    }
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(dir).join("gh").join("hosts.yml");
    }
    if let Some(appdata) = dirs::config_dir() {
        let win = appdata.join("GitHub CLI").join("hosts.yml");
        if win.exists() {
            return win;
        }
    }
    home_dir().join(".config").join("gh").join("hosts.yml")
}

fn resolve_token(cfg: &AppConfig) -> Option<String> {
    if let Some(k) = cfg.api_key(VendorId::Copilot) {
        return Some(k);
    }
    let mut command = Command::new("gh");
    command.args(["auth", "token"]);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let out = command.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn fetch_user(token: &str) -> Result<Value, FetchError> {
    http::get_json(
        USER_URL,
        &[
            ("Authorization", &format!("token {token}")),
            ("Accept", "application/json"),
            ("Editor-Version", "vscode/1.96.2"),
            ("Editor-Plugin-Version", "copilot-chat/0.26.7"),
            ("User-Agent", "GitHubCopilotChat/0.26.7"),
            ("X-GitHub-Api-Version", "2025-04-01"),
        ],
    )
}

pub(crate) fn snapshot_from_json(body: &Value) -> ProviderSnapshot {
    let plan = json_str(body, &["copilot_plan"]).unwrap_or_else(|| "Copilot".into());
    let reset = json_str(body, &["quota_reset_date_utc", "quota_reset_date"]);
    let snaps = body.get("quota_snapshots").cloned().unwrap_or(Value::Null);
    let mut lines = Vec::new();
    push_quota(
        &mut lines,
        &snaps,
        "premium_interactions",
        "Premium",
        &reset,
        "always",
    );
    push_quota(&mut lines, &snaps, "chat", "Chat", &reset, "always");
    push_quota(
        &mut lines,
        &snaps,
        "completions",
        "Completions",
        &reset,
        "demand",
    );
    snapshot_ok(VendorId::Copilot, &plan, lines)
}

fn push_quota(
    lines: &mut Vec<crate::model::MetricLine>,
    snaps: &Value,
    key: &str,
    label: &str,
    reset: &Option<String>,
    visible: &str,
) {
    let Some(q) = snaps.get(key) else { return };
    if q.get("unlimited").and_then(|v| v.as_bool()) == Some(true) {
        lines.push(badge_line(key, label, "Ilimitado"));
        return;
    }
    let remaining = json_f64(q, &["percent_remaining"]).unwrap_or(100.0);
    let used = (100.0 - remaining).clamp(0.0, 100.0);
    lines.push(progress_pct(
        key,
        label,
        used,
        reset.clone(),
        2_592_000,
        visible,
    ));
}
