//! Google Antigravity — RPC local o Cloud Code API con sesión Google.

use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok, snapshot_with_status,
    values_line, ProviderSnapshot, ProviderStatus, ProviderStatusReason, VendorId,
};

use super::Provider;

const QUOTA_URLS: &[&str] = &[
    "https://daily-cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
    "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
];
const PLAN_URLS: &[&str] = &[
    "https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist",
    "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist",
];

pub struct Antigravity;

impl Provider for Antigravity {
    fn id(&self) -> VendorId {
        VendorId::Antigravity
    }

    fn has_local_credentials(&self, _cfg: &AppConfig) -> bool {
        read_keyring_token().is_some() || std::env::var("ANTIGRAVITY_LS_ADDRESS").is_ok()
    }

    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let local_bases = discover_local_bases();
        for base in &local_bases {
            if let Ok(snap) = fetch_local(base) {
                return snap;
            }
        }
        if !local_bases.is_empty() {
            return local_service_unavailable();
        }
        let Some(token) = read_keyring_token() else {
            return snapshot_needs_auth(
                VendorId::Antigravity,
                "No hay una sesión Google guardada para Antigravity",
            );
        };
        match fetch_cloud(&token) {
            Ok(mut snap) => {
                snap.lines.insert(
                    0,
                    values_line("source", "Fuente", "Google API (app cerrada)", "always"),
                );
                snap
            }
            Err(e) => super::map_fetch_err(VendorId::Antigravity, e),
        }
    }
}

fn local_service_unavailable() -> ProviderSnapshot {
    snapshot_with_status(
        VendorId::Antigravity,
        ProviderStatus::Unavailable,
        ProviderStatusReason::LocalServiceUnavailable,
        "El servicio local de Antigravity no está disponible",
    )
}

fn read_keyring_token() -> Option<String> {
    let entry = keyring::Entry::new("gemini", "antigravity").ok()?;
    let raw = entry.get_password().ok()?;
    parse_blob(&raw)
}

fn parse_blob(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let json = if let Some(enc) = raw.strip_prefix("go-keyring-base64:") {
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, enc.trim()).ok()?;
        String::from_utf8(bytes).ok()?
    } else {
        raw.to_string()
    };
    let root: Value = serde_json::from_str(&json).ok()?;
    let token = root.get("token").filter(|v| v.is_object()).unwrap_or(&root);
    for k in [
        "access_token",
        "accessToken",
        "token",
        "id_token",
        "idToken",
        "bearerToken",
        "auth_token",
        "authToken",
    ] {
        if let Some(s) = token.get(k).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn fetch_cloud(token: &str) -> Result<ProviderSnapshot, FetchError> {
    let mut last = FetchError::Network("Antigravity: Cloud Code no respondió".into());
    for url in QUOTA_URLS {
        match http::post_json(
            url,
            &[
                ("Authorization", &format!("Bearer {token}")),
                ("User-Agent", "antigravity"),
            ],
            &json!({}),
        ) {
            Ok(body) => {
                let plan = fetch_plan(token).unwrap_or_else(|| "Antigravity".into());
                return Ok(snapshot_from_quota(&body, &plan));
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn fetch_plan(token: &str) -> Option<String> {
    for url in PLAN_URLS {
        if let Ok(body) = http::post_json(
            url,
            &[
                ("Authorization", &format!("Bearer {token}")),
                ("User-Agent", "agy"),
            ],
            &json!({}),
        ) {
            if let Some(s) = json_str(&body, &["currentTier", "tierId", "plan", "planName"]) {
                return Some(s);
            }
            if let Some(s) = body
                .pointer("/cloudaicompanionTier")
                .and_then(|v| v.as_str())
            {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn fetch_local(addr: &str) -> Result<ProviderSnapshot, FetchError> {
    let base = if addr.starts_with("http") {
        addr.trim_end_matches('/').to_string()
    } else {
        format!("http://{addr}")
    };
    let csrf = fetch_csrf(&base);
    let mut headers = vec![("Content-Type", "application/json".to_string())];
    if let Some(token) = csrf {
        headers.push(("x-codeium-csrf-token", token));
    }
    let refs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let url =
        format!("{base}/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary");
    let body = http::post_json(&url, &refs, &json!({}))?;
    let plan = fetch_local_plan(&base, &refs).unwrap_or_else(|| "Antigravity".into());
    Ok(snapshot_from_quota(&body, &plan))
}

fn fetch_csrf(base: &str) -> Option<String> {
    let html = http::get_text(base).ok()?;
    html.split("csrfToken\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
}

fn fetch_local_plan(base: &str, headers: &[(&str, &str)]) -> Option<String> {
    let url = format!("{base}/exa.language_server_pb.LanguageServerService/GetUserStatus");
    let body = http::post_json(&url, headers, &json!({})).ok()?;
    json_str(&body, &["currentTier", "tierId", "plan", "planName"]).or_else(|| {
        body.pointer("/userStatus/plan")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
}

fn discover_local_bases() -> Vec<String> {
    let mut bases = Vec::new();
    if let Ok(addr) = std::env::var("ANTIGRAVITY_LS_ADDRESS") {
        let trimmed = addr.trim();
        if !trimmed.is_empty() {
            if trimmed.starts_with("http") {
                bases.push(trimmed.trim_end_matches('/').to_string());
            } else {
                bases.push(format!("http://{trimmed}"));
            }
        }
    }
    for port in discover_windows_ports() {
        let url = format!("http://127.0.0.1:{port}");
        if !bases.contains(&url) {
            bases.push(url);
        }
    }
    bases
}

fn is_antigravity_process(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    let n = n.strip_suffix(".exe").unwrap_or(&n);
    n.contains("language_server") || n == "agy" || n.contains("antigravity")
}

fn discover_windows_ports() -> Vec<u16> {
    #[cfg(windows)]
    {
        let Some(tasklist) = run_hidden("tasklist", &["/FO", "CSV", "/NH"]) else {
            return Vec::new();
        };
        let mut pids = std::collections::HashSet::new();
        for line in tasklist.lines() {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 2 {
                continue;
            }
            let name = cols[0].trim_matches('"');
            if !is_antigravity_process(name) {
                continue;
            }
            if let Ok(pid) = cols[1].trim_matches('"').parse::<u32>() {
                pids.insert(pid);
            }
        }
        if pids.is_empty() {
            return Vec::new();
        }
        let Some(netstat) = run_hidden("netstat", &["-ano", "-p", "tcp"]) else {
            return Vec::new();
        };
        let mut per_pid: std::collections::BTreeMap<u32, Vec<u16>> =
            std::collections::BTreeMap::new();
        for line in netstat.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 5 || !cols[0].eq_ignore_ascii_case("TCP") {
                continue;
            }
            if !cols.iter().any(|c| c.eq_ignore_ascii_case("LISTENING")) {
                continue;
            }
            let local = cols[1];
            if !local.starts_with("127.0.0.1:") && !local.starts_with("[::1]:") {
                continue;
            }
            let Some((_, port_s)) = local.rsplit_once(':') else {
                continue;
            };
            let Ok(port) = port_s.parse::<u16>() else {
                continue;
            };
            let Ok(pid) = cols[cols.len() - 1].parse::<u32>() else {
                continue;
            };
            if pids.contains(&pid) && port != 0 {
                per_pid.entry(pid).or_default().push(port);
            }
        }
        probe_order(per_pid)
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn run_hidden(cmd: &str, args: &[&str]) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() && out.stdout.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn probe_order(per_pid: std::collections::BTreeMap<u32, Vec<u16>>) -> Vec<u16> {
    let groups: Vec<Vec<u16>> = per_pid
        .into_values()
        .map(|mut group| {
            group.sort_unstable_by(|a, b| b.cmp(a));
            group.dedup();
            group
        })
        .collect();
    let mut ports: Vec<u16> = Vec::new();
    for rank in 0..groups.iter().map(Vec::len).max().unwrap_or(0) {
        for port in groups.iter().filter_map(|group| group.get(rank)) {
            if !ports.contains(port) {
                ports.push(*port);
            }
        }
    }
    ports
}

pub(crate) fn snapshot_from_quota(body: &Value, plan: &str) -> ProviderSnapshot {
    let inner = body.get("response").unwrap_or(body);
    let mut lines = Vec::new();
    let groups = inner
        .get("quotaSummaries")
        .or_else(|| inner.get("groups"))
        .or_else(|| inner.get("modelGroups"))
        .cloned()
        .unwrap_or(Value::Array(vec![inner.clone()]));
    if let Some(arr) = groups.as_array() {
        for group in arr {
            let name = json_str(group, &["displayName", "name", "modelFamily"])
                .unwrap_or_else(|| "Pool".into());
            push_bucket(&mut lines, group, &name, "fiveHour", "5h", 18_000, "always");
            push_bucket(&mut lines, group, &name, "weekly", "7d", 604_800, "always");
            if let Some(buckets) = group.get("buckets").and_then(|v| v.as_array()) {
                for b in buckets {
                    let label =
                        json_str(b, &["displayName", "name", "bucketId"]).unwrap_or(name.clone());
                    let window = json_str(b, &["window"]).unwrap_or_default();
                    let secs = if window.contains("week") || window.contains("7") {
                        604_800
                    } else {
                        18_000
                    };
                    lines.push(progress_pct(
                        &format!(
                            "{}_{}",
                            name.to_ascii_lowercase().replace(' ', "_"),
                            label.to_ascii_lowercase().replace(' ', "_")
                        ),
                        &format!("{name} {label}"),
                        bucket_pct(b),
                        json_str(b, &["resetTime", "resetsAt", "reset_at"]),
                        secs,
                        "always",
                    ));
                }
            }
        }
    }
    if lines.is_empty() {
        if let Some(pct) = json_f64(inner, &["utilization", "usedPercent"]) {
            lines.push(progress_pct("quota", "Cuota", pct, None, 18_000, "always"));
        }
    }
    snapshot_ok(VendorId::Antigravity, plan, lines)
}

fn bucket_pct(b: &Value) -> f64 {
    if let Some(rem) = json_f64(b, &["remainingFraction", "remaining_fraction"]) {
        let used = if rem <= 1.0 {
            (1.0 - rem) * 100.0
        } else {
            100.0 - rem
        };
        return used.clamp(0.0, 100.0);
    }
    let used = json_f64(b, &["usedFraction", "utilization", "usedPercent"]).unwrap_or(0.0);
    if used <= 1.0 {
        used * 100.0
    } else {
        used
    }
}

fn push_bucket(
    lines: &mut Vec<crate::model::MetricLine>,
    group: &Value,
    name: &str,
    key: &str,
    suffix: &str,
    window: i64,
    visible: &str,
) {
    let Some(b) = group.get(key) else { return };
    let pct = bucket_pct(b);
    let reset = json_str(b, &["resetTime", "resetsAt", "reset_at"]);
    lines.push(progress_pct(
        &format!("{}_{suffix}", name.to_ascii_lowercase().replace(' ', "_")),
        &format!("{name} {suffix}"),
        pct,
        reset,
        window,
        visible,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_rpc_failure_is_not_reported_as_missing_google_auth() {
        let snapshot = local_service_unavailable();
        assert_eq!(snapshot.status, ProviderStatus::Unavailable);
        assert_eq!(
            snapshot.status_reason,
            Some(ProviderStatusReason::LocalServiceUnavailable)
        );
    }

    #[test]
    fn absent_google_session_is_needs_auth() {
        let snapshot = snapshot_needs_auth(VendorId::Antigravity, "missing session");
        assert_eq!(snapshot.status, ProviderStatus::NeedsAuth);
        assert_eq!(
            snapshot.status_reason,
            Some(ProviderStatusReason::MissingCredential)
        );
    }
}
