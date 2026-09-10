//! Decodifica el payload de un JWT sin verificar firma (solo lectura local).

use base64::Engine;

pub fn claims(token: &str) -> Option<serde_json::Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn exp_secs(token: &str) -> Option<i64> {
    let c = claims(token)?;
    c.get("exp")
        .and_then(|v| v.as_i64())
        .or_else(|| c.get("exp").and_then(|v| v.as_f64()).map(|f| f as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_jwt(claims: serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn reads_exp_and_sub() {
        let jwt = fake_jwt(serde_json::json!({"exp": 123, "sub": "auth0|abc"}));
        assert_eq!(exp_secs(&jwt), Some(123));
        assert_eq!(
            claims(&jwt).unwrap().get("sub").and_then(|v| v.as_str()),
            Some("auth0|abc")
        );
    }
}
