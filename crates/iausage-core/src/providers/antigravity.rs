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
        read_keyring_session().is_some() || std::env::var("ANTIGRAVITY_LS_ADDRESS").is_ok()
    }

    fn refresh(&self, _cfg: &AppConfig) -> ProviderSnapshot {
        let local_bases = discover_local_bases();
        // One CSRF lookup per refresh: it spawns PowerShell (~2 s) and used to
        // run once per probed port, which could push the refresh past the
        // dashboard's 12 s deadline and keep a stale error on screen.
        let csrf = if local_bases.is_empty() {
            None
        } else {
            std::env::var("ANTIGRAVITY_CSRF_TOKEN")
                .ok()
                .or_else(process_csrf_token)
        };
        for base in &local_bases {
            if let Ok(mut snap) = fetch_local(base, csrf.as_deref()) {
                snap.set_active_source(UsageSource::LocalSession);
                return snap;
            }
        }
        // A running but unhealthy local server must not mask a valid Google
        // session. Local is preferred, Cloud is the fallback.
        let Some(session) = read_keyring_session() else {
            if !local_bases.is_empty() {
                return local_service_unavailable();
            }
            return snapshot_needs_auth(
                VendorId::Antigravity,
                "No hay una sesión Google guardada para Antigravity",
            );
        };
        // Antigravity renueva el access token solo mientras está abierta; con
        // la app cerrada uno vencido solo consigue un 401 de Google. Renovarlo
        // aquí exigiría el cliente OAuth de Antigravity, que no es nuestro.
        if session.expired {
            return session_expired();
        }
        match fetch_cloud(&session.token) {
            Ok(mut snap) => {
                snap.set_active_source(UsageSource::Oauth);
                snap.lines.insert(
                    0,
                    values_line("source", "Fuente", "Google API", "details"),
                );
                snap.lines.insert(
                    1,
                    values_line("antigravity_app", "Antigravity", "App cerrada", "details"),
                );
                if let Some(email) = session.email.as_deref() {
                    snap.lines.insert(
                        2,
                        values_line("account", "Cuenta", &mask_email(email), "details"),
                    );
                }
                snap
            }
            Err(FetchError::Http(401, _)) => session_expired(),
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

fn session_expired() -> ProviderSnapshot {
    snapshot_with_status(
        VendorId::Antigravity,
        ProviderStatus::NeedsAuth,
        ProviderStatusReason::OAuthExpired,
        "La sesión Google de Antigravity caducó: abre Antigravity para renovarla.",
    )
}

struct GoogleSession {
    token: String,
    expired: bool,
    email: Option<String>,
}

fn read_keyring_session() -> Option<GoogleSession> {
    // Antigravity's Go client writes this credential directly as
    // "<service>:<user>" (visible via `cmdkey /list` as
    // `gemini:antigravity`), not the "<user>.<service>" target that
    // keyring-rs's own `Entry::new(service, user)` derives on Windows —
    // that mismatch alone makes the plain constructor miss the entry
    // entirely. And unlike our own credentials (always written through
    // keyring-rs's `set_password`, which stores UTF-16), this blob is raw
    // UTF-8 JSON: `get_password()` still decodes Windows credentials as
    // UTF-16 and returns `Ok` with garbage instead of an error, so it can't
    // be used as a probe here. Go straight to the raw bytes.
    let entry =
        keyring::Entry::new_with_target("gemini:antigravity", "gemini", "antigravity").ok()?;
    let raw = entry.get_secret().ok()?;
    let text = String::from_utf8_lossy(&raw)
        .trim_end_matches('\0')
        .to_string();
    parse_blob(&text, chrono::Utc::now().timestamp())
}

fn parse_blob(raw: &str, now_unix: i64) -> Option<GoogleSession> {
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
    let access = [
        "access_token",
        "accessToken",
        "token",
        "id_token",
        "idToken",
        "bearerToken",
        "auth_token",
        "authToken",
    ]
    .iter()
    .find_map(|k| {
        token
            .get(*k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    })?;
    // `expiry` es el `oauth2.Token.Expiry` de Go (RFC 3339). Su valor cero
    // (año 1) significa "sin vencimiento", no "vencido".
    let expired = token
        .get("expiry")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .is_some_and(|at| at.timestamp() > 0 && at.timestamp() <= now_unix);
    Some(GoogleSession {
        token: access.to_string(),
        expired,
        email: json_str(&root, &["email", "user_email", "account"]).or_else(|| {
            token
                .get("email")
                .and_then(|v| v.as_str())
                .filter(|s| s.contains('@'))
                .map(str::to_string)
        }),
    })
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
            if let Some(s) = plan_from_assist(&body) {
                return (s, project_from(&body));
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

fn fetch_local(addr: &str, csrf: Option<&str>) -> Result<ProviderSnapshot, FetchError> {
    let base = addr.trim_end_matches('/');
    let mut headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("Connect-Protocol-Version", "1".to_string()),
    ];
    if let Some(token) = csrf.filter(|token| !token.trim().is_empty()) {
        headers.push(("X-Codeium-Csrf-Token", token.to_string()));
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
    plan_from_assist(&body).or_else(|| {
        body.pointer("/userStatus/plan")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    })
}

pub(crate) fn plan_from_assist(body: &Value) -> Option<String> {
    for pointer in [
        "/currentTier/name",
        "/paidTier/name",
        "/response/currentTier/name",
        "/response/paidTier/name",
        "/userStatus/currentTier/name",
        "/userStatus/plan",
    ] {
        if let Some(name) = body
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "Antigravity")
        {
            return Some(name.to_string());
        }
    }
    json_str(body, &["plan", "planName"]).filter(|s| s != "Antigravity")
}

fn mask_email(email: &str) -> String {
    let Some((user, domain)) = email.split_once('@') else {
        return email.to_string();
    };
    let prefix: String = user.chars().take(2).collect();
    format!("{prefix}***@{domain}")
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
    let mut pending: Vec<(crate::model::MetricLine, String, String, Vec<String>)> = Vec::new();
    let groups = inner
        .get("quotaSummaries")
        .or_else(|| inner.get("groups"))
        .or_else(|| inner.get("modelGroups"))
        .cloned()
        .unwrap_or(Value::Array(vec![inner.clone()]));
    if let Some(arr) = groups.as_array() {
        for group in arr {
            let display = json_str(group, &["displayName", "name", "modelFamily"])
                .unwrap_or_else(|| "Pool".into());
            let group_id = canonical_group_id(group, &display);
            let group_label = pretty_group_label(&group_id, &display);
            let models = group_models(group);
            let mut buckets: Vec<(&Value, Option<(&'static str, i64, &'static str)>)> = Vec::new();
            if let Some(list) = group.get("buckets").and_then(|v| v.as_array()) {
                buckets.extend(list.iter().map(|bucket| (bucket, None)));
            } else {
                if let Some(b) = group.get("fiveHour") {
                    buckets.push((b, Some(("5h", 18_000, "5h"))));
                }
                if let Some(b) = group.get("weekly") {
                    buckets.push((b, Some(("weekly", 604_800, "weekly"))));
                }
            }
            for (bucket, forced_window) in buckets {
                let Some(pct) = bucket_pct(bucket) else {
                    continue;
                };
                let (suffix, window_secs, window_label) =
                    forced_window.unwrap_or_else(|| window_from_bucket(bucket, group));
                let id = format!("{group_id}_{suffix}");
                if pending.iter().any(|(line, ..)| line.id() == id) {
                    continue;
                }
                pending.push((
                    progress_pct(
                        &id,
                        window_label,
                        pct,
                        json_str(bucket, &["resetTime", "resetsAt", "reset_at"]),
                        window_secs,
                        "always",
                    ),
                    group_id.clone(),
                    group_label.clone(),
                    models.clone(),
                ));
            }
        }
    }
    let mut lines: Vec<_> = pending.iter().map(|(line, ..)| line.clone()).collect();
    if lines.is_empty() {
        if let Some(pct) = json_f64(inner, &["utilization", "usedPercent"]) {
            lines.push(progress_pct("quota", "Cuota", pct, None, 18_000, "always"));
        }
    }
    if let Some(overages) = overage_line(inner) {
        lines.push(overages);
    }
    let mut snapshot = snapshot_ok(VendorId::Antigravity, plan, lines);
    for quota in &mut snapshot.quotas {
        if let Some((_, group_id, group_label, models)) =
            pending.iter().find(|(line, ..)| line.id() == quota.id)
        {
            quota.group_id = Some(group_id.clone());
            quota.group_label = Some(group_label.clone());
            quota.models = models.clone();
            quota.visible = "always".into();
        }
    }
    snapshot
}

fn overage_line(inner: &Value) -> Option<crate::model::MetricLine> {
    let enabled = inner
        .get("creditOveragesEnabled")
        .or_else(|| inner.get("overageEnabled"))
        .or_else(|| inner.get("enableCreditOverages"))
        .and_then(|v| v.as_bool())?;
    Some(values_line(
        "overages",
        "Credit Overages",
        if enabled { "Activado" } else { "Desactivado" },
        "details",
    ))
}

fn canonical_group_id(group: &Value, display_name: &str) -> String {
    let buckets = group
        .get("buckets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for bucket in &buckets {
        if let Some(id) = bucket.get("bucketId").and_then(|v| v.as_str()) {
            let prefix = id
                .rsplit_once('-')
                .map(|(head, _)| head)
                .unwrap_or(id)
                .to_ascii_lowercase();
            if prefix.contains("gemini") {
                return "gemini_models".into();
            }
            if prefix.contains("3p") || prefix.contains("claude") || prefix.contains("gpt") {
                return "claude_gpt_models".into();
            }
        }
    }
    let lower = display_name.to_ascii_lowercase();
    if lower.contains("gemini") {
        return "gemini_models".into();
    }
    if lower.contains("claude") || lower.contains("gpt") {
        return "claude_gpt_models".into();
    }
    display_name
        .to_ascii_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        .trim_matches('_')
        .to_string()
}

fn pretty_group_label(group_id: &str, display_name: &str) -> String {
    match group_id {
        "gemini_models" => "Gemini Models".into(),
        "claude_gpt_models" => "Claude + GPT".into(),
        _ => display_name.to_string(),
    }
}

fn group_models(group: &Value) -> Vec<String> {
    for key in ["models", "includedModels", "modelIds"] {
        if let Some(arr) = group.get(key).and_then(|v| v.as_array()) {
            let names: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .or_else(|| json_str(item, &["displayName", "name", "id"]))
                })
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !names.is_empty() {
                return names;
            }
        }
    }
    let desc = json_str(group, &["description"]).unwrap_or_default();
    parse_models_from_description(&desc)
}

pub(crate) fn parse_models_from_description(desc: &str) -> Vec<String> {
    let trimmed = desc.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("model") || !(trimmed.contains(':') || trimmed.contains(',')) {
        return Vec::new();
    }
    let rest = trimmed.split_once(':').map(|(_, rest)| rest).unwrap_or(trimmed);
    rest.split([',', ';', '&'])
        .map(|part| {
            part.trim()
                .trim_start_matches("and ")
                .trim_start_matches("y ")
                .trim()
                .to_string()
        })
        .filter(|part| !part.is_empty() && part.len() < 80)
        .collect()
}

fn window_from_bucket(bucket: &Value, _group: &Value) -> (&'static str, i64, &'static str) {
    let window = json_str(bucket, &["window"]).unwrap_or_default();
    let bucket_id = json_str(bucket, &["bucketId"]).unwrap_or_default();
    let display = json_str(bucket, &["displayName", "name"]).unwrap_or_default();
    let hay = format!("{window} {bucket_id} {display}").to_ascii_lowercase();
    if hay.contains("week") || hay.contains("7d") || hay.contains("7-d") || hay.contains("weekly")
    {
        ("weekly", 604_800, "weekly")
    } else {
        ("5h", 18_000, "5h")
    }
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

    #[test]
    fn expired_google_session_is_detected_from_the_go_token_expiry() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-14T12:00:00-04:00")
            .unwrap()
            .timestamp();
        let blob = |expiry: &str| {
            format!(
                r#"{{"token":{{"access_token":"ya29.x","refresh_token":"1//r","expiry":"{expiry}"}},"auth_method":"consumer"}}"#
            )
        };
        let past = parse_blob(&blob("2026-09-13T15:57:58.5818433-04:00"), now).unwrap();
        assert_eq!(past.token, "ya29.x");
        assert!(past.expired);
        assert!(
            !parse_blob(&blob("2026-09-14T13:00:00-04:00"), now)
                .unwrap()
                .expired
        );
        // Go zero time: sin vencimiento conocido.
        assert!(
            !parse_blob(&blob("0001-01-01T00:00:00Z"), now)
                .unwrap()
                .expired
        );
    }

    #[test]
    fn maps_two_groups_of_two_windows_and_plan_object() {
        let body = serde_json::json!({
            "groups": [
                {
                    "displayName": "GEMINI MODELS",
                    "description": "Models within this group: Gemini Flash, Gemini Pro",
                    "buckets": [
                        {"bucketId": "gemini-weekly", "displayName": "Weekly Limit Remaining",
                         "window": "weekly", "remainingFraction": 0.9452, "resetTime": "2099-01-01T00:00:00Z"},
                        {"bucketId": "gemini-5h", "displayName": "Five Hour Limit Remaining",
                         "window": "5h", "remainingFraction": 0.8512, "resetTime": "2099-01-01T04:28:00Z"}
                    ]
                },
                {
                    "displayName": "CLAUDE AND GPT MODELS",
                    "description": "Models within this group: Claude Opus, Claude Sonnet, GPT-OSS",
                    "buckets": [
                        {"bucketId": "3p-weekly", "window": "weekly", "remainingFraction": 1.0},
                        {"bucketId": "3p-5h", "window": "5h", "remainingFraction": 1.0}
                    ]
                }
            ]
        });
        let snapshot = snapshot_from_quota(&body, "Google AI Pro");
        assert_eq!(snapshot.plan, "Google AI Pro");
        assert_eq!(snapshot.quotas.len(), 4);
        let groups = crate::model::groups_from_quotas(&snapshot.quotas);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, "gemini_models");
        assert_eq!(groups[0].models, vec!["Gemini Flash", "Gemini Pro"]);
        assert_eq!(groups[1].id, "claude_gpt_models");
        assert_eq!(groups[1].models.len(), 3);
        let weekly = snapshot.quotas.iter().find(|q| q.id == "gemini_models_weekly").unwrap();
        assert_eq!(weekly.label, "weekly");
        assert!((weekly.used_percent.unwrap() - 5.48).abs() < 0.001);
        assert_eq!(weekly.visible, "always");
        assert_eq!(
            plan_from_assist(&serde_json::json!({"currentTier": {"id": "standard-tier", "name": "Google AI Pro"}})),
            Some("Google AI Pro".into())
        );
    }

    #[test]
    fn description_without_model_list_does_not_become_models() {
        assert!(parse_models_from_description(
            "Quota is consumed proportionally to the cost of the tokens."
        )
        .is_empty());
        assert_eq!(
            parse_models_from_description("Models within this group: Gemini Flash, Gemini Pro"),
            vec!["Gemini Flash", "Gemini Pro"]
        );
    }

    #[test]
    fn expired_google_session_asks_to_open_antigravity() {
        let snapshot = session_expired();
        assert_eq!(snapshot.status, ProviderStatus::NeedsAuth);
        assert_eq!(
            snapshot.status_reason,
            Some(ProviderStatusReason::OAuthExpired)
        );
    }
}
