//! Google Antigravity — RPC local o Cloud Code API con sesión Google.

use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::http::{self, FetchError};
use crate::model::{
    json_f64, json_str, progress_pct, snapshot_needs_auth, snapshot_ok, snapshot_with_status,
    values_line, ProviderSnapshot, ProviderStatus, ProviderStatusReason, UsageSource, VendorId,
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
            if let Ok(mut snap) = fetch_local(base) {
                snap.set_active_source(UsageSource::LocalSession);
                return snap;
            }
        }
        // A running but unhealthy local server must not mask a valid Google
        // session. Local is preferred, Cloud is the fallback.
        let Some(token) = read_keyring_token() else {
            if !local_bases.is_empty() {
                return local_service_unavailable();
            }
            return snapshot_needs_auth(
                VendorId::Antigravity,
                "No hay una sesión Google guardada para Antigravity",
            );
        };
        match fetch_cloud(&token) {
            Ok(mut snap) => {
                snap.set_active_source(UsageSource::Oauth);
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
    let (plan, project) = fetch_cloud_context(token);
    let body = project
        .as_ref()
        .map(|project| json!({"cloudaicompanionProject": project}))
        .unwrap_or_else(|| json!({}));
    let mut last = FetchError::Network("Antigravity: Cloud Code no respondió".into());
    for url in QUOTA_URLS {
        match http::post_json(
            url,
            &[
                ("Authorization", &format!("Bearer {token}")),
                ("User-Agent", "antigravity"),
            ],
            &body,
        ) {
            Ok(body) => {
                return Ok(snapshot_from_quota(&body, &plan));
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn fetch_cloud_context(token: &str) -> (String, Option<String>) {
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
                return (s, project_from(&body));
            }
            if let Some(s) = body
                .pointer("/cloudaicompanionTier")
                .and_then(|v| v.as_str())
            {
                return (s.to_string(), project_from(&body));
            }
            return ("Antigravity".into(), project_from(&body));
        }
    }
    ("Antigravity".into(), None)
}

fn project_from(body: &Value) -> Option<String> {
    json_str(
        body,
        &["cloudaicompanionProject", "cloudAiCompanionProject"],
    )
    .or_else(|| {
        body.pointer("/cloudaicompanionProject")
            .and_then(|value| value.as_str())
            .map(str::to_string)
    })
}

const LOCAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);
const LS_SERVICE: &str = "exa.language_server_pb.LanguageServerService";

fn fetch_local(addr: &str) -> Result<ProviderSnapshot, FetchError> {
    let base = addr.trim_end_matches('/');
    let csrf = std::env::var("ANTIGRAVITY_CSRF_TOKEN")
        .ok()
        .or_else(process_csrf_token);
    let mut headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("Connect-Protocol-Version", "1".to_string()),
    ];
    if let Some(token) = csrf.filter(|token| !token.trim().is_empty()) {
        headers.push(("X-Codeium-Csrf-Token", token));
    }
    let refs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    // A listening port is not automatically the language server. Connect's
    // lightweight readiness endpoint prevents quota calls to unrelated ports.
    local_post(base, "GetUnleashData", &refs)?;
    let summary = local_post(base, "RetrieveUserQuotaSummary", &refs)
        .or_else(|_| local_post(base, "GetUserStatus", &refs))
        .or_else(|_| local_post(base, "GetCommandModelConfigs", &refs))?;
    let plan = fetch_local_plan(base, &refs).unwrap_or_else(|| "Antigravity".into());
    Ok(snapshot_from_quota(&summary, &plan))
}

fn local_post(base: &str, method: &str, headers: &[(&str, &str)]) -> Result<Value, FetchError> {
    http::post_json_local(
        &format!("{base}/{LS_SERVICE}/{method}"),
        headers,
        &json!({}),
        LOCAL_TIMEOUT,
    )
}

fn process_csrf_token() -> Option<String> {
    #[cfg(windows)]
    {
        let script = "Get-CimInstance Win32_Process | Select-Object Name,CommandLine | ConvertTo-Json -Compress";
        let raw = run_hidden("powershell", &["-NoProfile", "-Command", script])?;
        let processes: Value = serde_json::from_str(&raw).ok()?;
        let candidates = processes
            .as_array()
            .cloned()
            .unwrap_or_else(|| vec![processes]);
        candidates.iter().find_map(|process| {
            let name = process
                .get("Name")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let command = process
                .get("CommandLine")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let lower = command.to_ascii_lowercase();
            (is_antigravity_process(name) && lower.contains("antigravity")
                || lower.contains("--app_data_dir antigravity"))
            .then(|| command_flag(command, "--csrf_token"))
            .flatten()
        })
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn command_flag(command: &str, flag: &str) -> Option<String> {
    let mut words = command.split_whitespace();
    while let Some(word) = words.next() {
        if word.trim_matches('"') == flag {
            return words
                .next()
                .map(|value| value.trim_matches('"').to_string())
                .filter(|value| !value.is_empty());
        }
        if let Some(value) = word.strip_prefix(&format!("{flag}=")) {
            return (!value.is_empty()).then(|| value.trim_matches('"').to_string());
        }
    }
    None
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
                bases.push(format!("https://{trimmed}"));
                bases.push(format!("http://{trimmed}"));
            }
        }
    }
    for port in discover_windows_ports() {
        for scheme in ["https", "http"] {
            let url = format!("{scheme}://127.0.0.1:{port}");
            if !bases.contains(&url) {
                bases.push(url);
            }
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
        // Nunca heredar stdin: un hijo que espere entrada bloquearía
        // el hilo de refresh/detect para siempre.
        .stdin(std::process::Stdio::null())
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
                    let Some(pct) = bucket_pct(b) else {
                        continue;
                    };
                    lines.push(progress_pct(
                        &format!(
                            "{}_{}",
                            name.to_ascii_lowercase().replace(' ', "_"),
                            label.to_ascii_lowercase().replace(' ', "_")
                        ),
                        &format!("{name} {label}"),
                        pct,
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

fn bucket_pct(b: &Value) -> Option<f64> {
    if let Some(rem) = json_f64(b, &["remainingFraction", "remaining_fraction"]) {
        let used = if rem <= 1.0 {
            (1.0 - rem) * 100.0
        } else {
            100.0 - rem
        };
        return Some(used.clamp(0.0, 100.0));
    }
    let used = json_f64(b, &["usedFraction", "utilization", "usedPercent"])?;
    if used <= 1.0 {
        Some(used * 100.0)
    } else {
        Some(used)
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
    let Some(pct) = bucket_pct(b) else { return };
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

    #[test]
    fn missing_bucket_measurement_is_omitted() {
        let snapshot = snapshot_from_quota(
            &serde_json::json!({
                "plan": "Pro",
                "groups": [{"displayName": "Gemini", "fiveHour": {}}]
            }),
            "Pro",
        );
        assert!(snapshot.quotas.is_empty());
    }

    #[test]
    fn extracts_csrf_token_from_language_server_arguments() {
        assert_eq!(
            command_flag(
                "language_server --app_data_dir antigravity --csrf_token csrf-123",
                "--csrf_token"
            ),
            Some("csrf-123".into())
        );
        assert_eq!(
            command_flag("language_server --csrf_token=csrf-456", "--csrf_token"),
            Some("csrf-456".into())
        );
    }
}
