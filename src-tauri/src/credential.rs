use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::{ACCEPT, COOKIE, USER_AGENT};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::error::AppError;
use crate::models::ResolvedCredential;

const SESSION_ENDPOINT: &str = "https://chatgpt.com/api/auth/session";
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
const MAX_CREDENTIAL_BYTES: usize = 1024 * 1024;

pub async fn resolve_credential(
    client: &reqwest::Client,
    input: &str,
) -> Result<ResolvedCredential, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::MissingCredential);
    }
    if trimmed.len() > MAX_CREDENTIAL_BYTES {
        return Err(AppError::CredentialTooLarge);
    }

    let mut document = None;
    let mut access_token = None;
    let mut session_token: Option<Zeroizing<String>> = None;

    if trimmed.starts_with('{') {
        let parsed: Value =
            serde_json::from_str(trimmed).map_err(|_| AppError::UnsupportedCredential)?;
        access_token = first_string(
            &parsed,
            &[
                &["accessToken"],
                &["access_token"],
                &["tokens", "access_token"],
                &["session", "accessToken"],
                &["session", "access_token"],
            ],
        );
        session_token = first_string(
            &parsed,
            &[
                &["sessionToken"],
                &["session_token"],
                &["tokens", "session_token"],
            ],
        )
        .map(Zeroizing::new);
        document = Some(parsed);
    } else {
        let bearer = trimmed
            .strip_prefix("Bearer ")
            .or_else(|| trimmed.strip_prefix("bearer "))
            .unwrap_or(trimmed)
            .trim();
        if looks_like_jwt(bearer) {
            access_token = Some(bearer.to_string());
        } else {
            session_token = Some(Zeroizing::new(bearer.to_string()));
        }
    }

    if access_token.is_none() {
        let opaque = session_token
            .as_deref()
            .ok_or(AppError::MissingCredential)?;
        let session = exchange_session_cookie(client, opaque).await?;
        access_token = first_string(
            &session,
            &[
                &["accessToken"],
                &["access_token"],
                &["tokens", "access_token"],
            ],
        );
        if access_token.is_none() {
            return Err(AppError::SessionExchange(
                "会话响应中没有 accessToken".into(),
            ));
        }
        document = Some(session);
    }

    let token = access_token.ok_or(AppError::MissingCredential)?;
    if !looks_like_jwt(&token) {
        return Err(AppError::UnsupportedCredential);
    }
    let claims = decode_jwt_claims(&token)?;
    let now = Utc::now().timestamp();
    if claims
        .get("exp")
        .and_then(Value::as_i64)
        .is_some_and(|exp| exp <= now)
    {
        return Err(AppError::ExpiredToken);
    }

    let auth = claims
        .get("https://api.openai.com/auth")
        .and_then(Value::as_object);
    let profile = claims
        .get("https://api.openai.com/profile")
        .and_then(Value::as_object);
    let session = document.as_ref();

    let account_id = first_non_empty([
        session.and_then(|v| value_str(v, &["account", "id"])),
        session.and_then(|v| value_str(v, &["account", "account_id"])),
        auth.and_then(|v| v.get("chatgpt_account_id"))
            .and_then(Value::as_str),
        claims.get("chatgpt_account_id").and_then(Value::as_str),
    ]);
    let email = first_non_empty([
        session.and_then(|v| value_str(v, &["user", "email"])),
        profile.and_then(|v| v.get("email")).and_then(Value::as_str),
        claims.get("email").and_then(Value::as_str),
    ]);
    let name = first_non_empty([
        session.and_then(|v| value_str(v, &["user", "name"])),
        profile.and_then(|v| v.get("name")).and_then(Value::as_str),
        claims.get("name").and_then(Value::as_str),
    ]);
    let user_id = first_non_empty([
        session.and_then(|v| value_str(v, &["user", "id"])),
        auth.and_then(|v| v.get("user_id")).and_then(Value::as_str),
        claims.get("sub").and_then(Value::as_str),
    ]);
    let claimed_plan = first_non_empty([
        session.and_then(|v| value_str(v, &["account", "planType"])),
        session.and_then(|v| value_str(v, &["account", "plan_type"])),
        auth.and_then(|v| v.get("chatgpt_plan_type"))
            .and_then(Value::as_str),
    ]);
    let token_expires_at = claims
        .get("exp")
        .and_then(Value::as_i64)
        .and_then(timestamp_to_rfc3339);

    Ok(ResolvedCredential {
        access_token: Zeroizing::new(token),
        account_id,
        email,
        name,
        user_id,
        claimed_plan,
        token_expires_at,
    })
}

async fn exchange_session_cookie(
    client: &reqwest::Client,
    session_token: &str,
) -> Result<Value, AppError> {
    if session_token.is_empty()
        || session_token.len() > 16_384
        || session_token.contains(['\r', '\n', ';'])
    {
        return Err(AppError::UnsafeSessionToken);
    }
    let cookie = format!(
        "__Secure-next-auth.session-token={0}; next-auth.session-token={0}; __Secure-authjs.session-token={0}; authjs.session-token={0}",
        session_token
    );
    let response = client
        .get(SESSION_ENDPOINT)
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, BROWSER_UA)
        .header(COOKIE, cookie)
        .send()
        .await
        .map_err(|e| AppError::SessionExchange(network_message(&e)))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::SessionExchange(format!(
            "HTTP {}",
            status.as_u16()
        )));
    }
    response
        .json::<Value>()
        .await
        .map_err(|_| AppError::SessionExchange("响应不是有效 JSON".into()))
}

fn first_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        value_str(value, path)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn value_str<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn first_non_empty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn looks_like_jwt(value: &str) -> bool {
    value.len() >= 40 && value.split('.').count() == 3
}

fn decode_jwt_claims(token: &str) -> Result<Value, AppError> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or(AppError::UnsupportedCredential)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AppError::UnsupportedCredential)?;
    serde_json::from_slice(&decoded).map_err(|_| AppError::UnsupportedCredential)
}

fn timestamp_to_rfc3339(timestamp: i64) -> Option<String> {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|value: DateTime<Utc>| value.to_rfc3339())
}

fn network_message(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "连接超时".into()
    } else if error.is_connect() {
        "无法连接 chatgpt.com".into()
    } else {
        "网络请求失败".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_nested_auth_json() {
        let value = json!({
            "tokens": {"access_token": "header.payload.signature"},
            "session_token": "opaque"
        });
        assert_eq!(
            first_string(&value, &[&["tokens", "access_token"]]),
            Some("header.payload.signature".into())
        );
        assert_eq!(
            first_string(&value, &[&["session_token"]]),
            Some("opaque".into())
        );
    }

    #[test]
    fn rejects_cookie_header_injection() {
        let value = "abc\r\nInjected: true";
        assert!(value.contains(['\r', '\n', ';']));
    }

    #[test]
    fn credential_size_limit_is_one_mibibyte() {
        assert_eq!(MAX_CREDENTIAL_BYTES, 1_048_576);
    }
}
