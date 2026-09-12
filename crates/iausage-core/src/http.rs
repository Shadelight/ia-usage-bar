//! Cliente HTTP compartido (reqwest blocking).

use std::time::Duration;

use reqwest::blocking::{Client, RequestBuilder};
use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug)]
pub enum FetchError {
    RateLimited,
    Http(u16, String),
    Network(String),
    Parse(String),
}

impl FetchError {
    pub fn message(&self) -> String {
        match self {
            FetchError::RateLimited => "Límite de peticiones (reintentando)".to_string(),
            FetchError::Http(code, body) => {
                if body.is_empty() {
                    format!("HTTP {code}")
                } else {
                    format!("HTTP {code}: {body}")
                }
            }
            FetchError::Network(e) | FetchError::Parse(e) => e.clone(),
        }
    }
}

pub fn client() -> Result<Client, FetchError> {
    Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(8))
        .build()
        .map_err(|e| FetchError::Network(format!("cliente HTTP: {e}")))
}

pub(crate) fn redact_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                let key = key.to_ascii_lowercase();
                if [
                    "token",
                    "secret",
                    "password",
                    "cookie",
                    "authorization",
                    "api_key",
                    "email",
                ]
                .iter()
                .any(|needle| key.contains(needle))
                {
                    *item = Value::String("[redacted]".into());
                } else {
                    redact_json(item);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_json),
        _ => {}
    }
}

pub fn send_json(builder: RequestBuilder) -> Result<Value, FetchError> {
    let resp = builder
        .send()
        .map_err(|e| FetchError::Network(format!("red: {e}")))?;
    #[cfg(debug_assertions)]
    let endpoint = format!(
        "{}://{}{}",
        resp.url().scheme(),
        resp.url().host_str().unwrap_or("unknown"),
        resp.url().path()
    );
    let status = resp.status();
    let code = status.as_u16();
    if code == 429 {
        return Err(FetchError::RateLimited);
    }
    let text = resp
        .text()
        .map_err(|e| FetchError::Network(format!("cuerpo: {e}")))?;
    if !status.is_success() {
        let safe_text = serde_json::from_str::<Value>(&text)
            .map(|mut value| {
                redact_json(&mut value);
                value.to_string()
            })
            .unwrap_or(text);
        let clip: String = safe_text.chars().take(180).collect();
        return Err(FetchError::Http(code, clip));
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    let value: Value =
        serde_json::from_str(&text).map_err(|e| FetchError::Parse(format!("json: {e}")))?;
    #[cfg(debug_assertions)]
    {
        let mut safe = value.clone();
        redact_json(&mut safe);
        eprintln!("[usage raw {endpoint}] {safe}");
    }
    Ok(value)
}

pub fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<Value, FetchError> {
    let c = client()?;
    let mut req = c.get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    send_json(req)
}

pub fn get_text(url: &str) -> Result<String, FetchError> {
    let c = client()?;
    let resp = c
        .get(url)
        .send()
        .map_err(|e| FetchError::Network(format!("red: {e}")))?;
    resp.text()
        .map_err(|e| FetchError::Network(format!("cuerpo: {e}")))
}

pub fn post_form(url: &str, form: &[(&str, &str)]) -> Result<Value, FetchError> {
    let c = client()?;
    send_json(c.post(url).form(form))
}

pub fn post_json(url: &str, headers: &[(&str, &str)], body: &Value) -> Result<Value, FetchError> {
    let c = client()?;
    let mut req = c.post(url).json(body);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    send_json(req)
}

/// Local language servers frequently use a self-signed loopback certificate.
/// Keep this opt-in so remote provider requests always retain TLS validation.
pub fn post_json_local(
    url: &str,
    headers: &[(&str, &str)],
    body: &Value,
    timeout: Duration,
) -> Result<Value, FetchError> {
    let c = Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| FetchError::Network(format!("cliente HTTP local: {e}")))?;
    let mut req = c.post(url).json(body);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    send_json(req)
}
