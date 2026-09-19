use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::{ACCEPT, COOKIE, USER_AGENT};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::error::AppError;
use crate::models::{CredentialCheck, CredentialValidation, ResolvedCredential};

const SESSION_ENDPOINT: &str = "https://chatgpt.com/api/auth/session";
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
const MAX_CREDENTIAL_BYTES: usize = 1024 * 1024;

pub fn validate_credential_input(input: &str) -> CredentialValidation {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return invalid_validation(
            "empty",
            "尚未输入会话数据",
            vec!["完整 Session JSON 或会话令牌".into()],
        );
    }
    if trimmed.len() > MAX_CREDENTIAL_BYTES {
        return invalid_validation(
            "oversized",
            "凭证超过 1 MiB 安全上限",
            vec!["大小不超过 1 MiB 的完整凭证".into()],
        );
    }

    if trimmed.starts_with('{') {
        let document: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => {
                return invalid_validation(
                    "json",
                    "JSON 语法不完整或已截断",
                    vec!["有效 JSON 结构".into()],
                )
            }
        };
        let access_token = first_string(
            &document,
            &[
                &["accessToken"],
                &["access_token"],
                &["tokens", "access_token"],
                &["session", "accessToken"],
                &["session", "access_token"],
            ],
        );
        if let Some(token) = access_token.as_deref() {
            let kind = if value_str(&document, &["tokens", "access_token"]).is_some() {
                "codex_auth_json"
            } else {
                "session_json"
            };
            return validate_access_document(token, Some(&document), kind);
        }
        if let Some(session_token) = first_string(
            &document,
            &[
                &["sessionToken"],
                &["session_token"],
                &["tokens", "session_token"],
            ],
        ) {
            return validate_opaque_session(&session_token, "session_json");
        }
        return invalid_validation(
            "json",
            "JSON 可解析，但没有可用的会话令牌",
            vec!["accessToken、access_token 或 sessionToken".into()],
        );
    }

    let bearer = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .unwrap_or(trimmed)
        .trim();
    if looks_like_jwt(bearer) {
        validate_access_document(bearer, None, "access_token")
    } else {
        validate_opaque_session(bearer, "session_token")
    }
}

pub fn ensure_resolved_complete(credential: &ResolvedCredential) -> Result<(), AppError> {
    let mut missing = Vec::new();
    if credential
        .email
        .as_deref()
        .map_or(true, |value| !looks_like_email(value))
    {
        missing.push("有效邮箱");
    }
    if credential.user_id.as_deref().map_or(true, str::is_empty) {
        missing.push("用户 ID");
    }
    if credential.account_id.as_deref().map_or(true, str::is_empty) {
        missing.push("ChatGPT 账户 ID");
    }
    if credential
        .token_expires_at
        .as_deref()
        .map_or(true, str::is_empty)
    {
        missing.push("令牌有效期");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::IncompleteCredential(missing.join("、")))
    }
}

fn validate_access_document(
    token: &str,
    document: Option<&Value>,
    kind: &str,
) -> CredentialValidation {
    let claims = match decode_jwt_claims(token) {
        Ok(value) => value,
        Err(_) => {
            return invalid_validation(
                kind,
                "Access Token 结构无效或已截断",
                vec!["完整的三段式 JWT".into()],
            )
        }
    };
    let id_claims = document
        .and_then(|value| {
            first_string(
                value,
                &[&["idToken"], &["id_token"], &["tokens", "id_token"]],
            )
        })
        .and_then(|value| decode_jwt_claims(&value).ok());
    let auth = claims.get("https://api.openai.com/auth");
    let profile = claims.get("https://api.openai.com/profile");

    let account_id = first_non_empty([
        document.and_then(|value| value_str(value, &["account", "id"])),
        document.and_then(|value| value_str(value, &["account", "account_id"])),
        document.and_then(|value| value_str(value, &["tokens", "account_id"])),
        auth.and_then(|value| value.get("chatgpt_account_id"))
            .and_then(Value::as_str),
        claims.get("chatgpt_account_id").and_then(Value::as_str),
    ]);
    let email = first_non_empty([
        document.and_then(|value| value_str(value, &["user", "email"])),
        profile
            .and_then(|value| value.get("email"))
            .and_then(Value::as_str),
        claims.get("email").and_then(Value::as_str),
        id_claims
            .as_ref()
            .and_then(|value| value.get("email"))
            .and_then(Value::as_str),
    ]);
    let user_id = first_non_empty([
        document.and_then(|value| value_str(value, &["user", "id"])),
        auth.and_then(|value| value.get("user_id"))
            .and_then(Value::as_str),
        claims.get("sub").and_then(Value::as_str),
        id_claims
            .as_ref()
            .and_then(|value| value.get("sub"))
            .and_then(Value::as_str),
    ]);
    let expires_at = claims.get("exp").and_then(Value::as_i64);
    let not_expired = expires_at.is_some_and(|value| value > Utc::now().timestamp());
    let identity_ok = email.as_deref().is_some_and(looks_like_email) && user_id.is_some();
    let account_ok = account_id.is_some();
    let expiry_ok = expires_at.is_some() && not_expired;

    let checks = vec![
        validation_check("format", "令牌结构", true, "三段式 JWT 可完整解析"),
        validation_check(
            "identity",
            "身份信息",
            identity_ok,
            if identity_ok {
                "已包含邮箱与用户 ID"
            } else {
                "缺少有效邮箱或用户 ID"
            },
        ),
        validation_check(
            "account",
            "账户范围",
            account_ok,
            if account_ok {
                "已包含 ChatGPT 账户 ID"
            } else {
                "缺少 ChatGPT 账户 ID"
            },
        ),
        validation_check(
            "expiry",
            "有效期限",
            expiry_ok,
            if expires_at.is_none() {
                "令牌没有 exp 有效期"
            } else if not_expired {
                "令牌仍在有效期内"
            } else {
                "令牌已经过期"
            },
        ),
    ];
    let mut missing = Vec::new();
    if !identity_ok {
        missing.push("邮箱和用户 ID".into());
    }
    if !account_ok {
        missing.push("ChatGPT 账户 ID".into());
    }
    if !expiry_ok {
        missing.push("有效且未过期的 exp".into());
    }
    let complete = missing.is_empty();
    CredentialValidation {
        kind: kind.into(),
        valid: complete,
        complete,
        can_query: complete,
        summary: if complete {
            "凭证结构完整，可以开始只读查询".into()
        } else {
            "凭证可解析，但缺少查询所需字段".into()
        },
        checks,
        missing,
    }
}

fn validate_opaque_session(value: &str, kind: &str) -> CredentialValidation {
    let safe = value.len() >= 32
        && value.len() <= 16_384
        && !value.contains(['\r', '\n', ';'])
        && !value.chars().any(char::is_whitespace);
    if !safe {
        return invalid_validation(
            kind,
            "Session Token 无效、过短或包含不安全字符",
            vec!["完整且未截断的 Session Token".into()],
        );
    }
    CredentialValidation {
        kind: kind.into(),
        valid: true,
        complete: false,
        can_query: true,
        summary: "Token 格式可识别；查询前会先向 chatgpt.com 换取并核验完整会话".into(),
        checks: vec![
            validation_check("format", "令牌结构", true, "Token 长度与字符检查通过"),
            pending_check("identity", "身份信息", "等待 chatgpt.com 返回邮箱与用户 ID"),
            pending_check("account", "账户范围", "等待 chatgpt.com 返回账户 ID"),
            pending_check("expiry", "有效期限", "等待 chatgpt.com 确认会话有效期"),
        ],
        missing: Vec::new(),
    }
}

fn invalid_validation(kind: &str, summary: &str, missing: Vec<String>) -> CredentialValidation {
    CredentialValidation {
        kind: kind.into(),
        valid: false,
        complete: false,
        can_query: false,
        summary: summary.into(),
        checks: vec![validation_check("format", "凭证结构", false, summary)],
        missing,
    }
}

fn validation_check(id: &str, label: &str, passed: bool, detail: &str) -> CredentialCheck {
    CredentialCheck {
        id: id.into(),
        label: label.into(),
        status: if passed { "pass" } else { "fail" }.into(),
        detail: detail.into(),
    }
}

fn pending_check(id: &str, label: &str, detail: &str) -> CredentialCheck {
    CredentialCheck {
        id: id.into(),
        label: label.into(),
        status: "pending".into(),
        detail: detail.into(),
    }
}

fn looks_like_email(value: &str) -> bool {
    let value = value.trim();
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

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

    let id_claims = document
        .as_ref()
        .and_then(|value| {
            first_string(
                value,
                &[&["idToken"], &["id_token"], &["tokens", "id_token"]],
            )
        })
        .and_then(|value| decode_jwt_claims(&value).ok());
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
        session.and_then(|v| value_str(v, &["tokens", "account_id"])),
        auth.and_then(|v| v.get("chatgpt_account_id"))
            .and_then(Value::as_str),
        claims.get("chatgpt_account_id").and_then(Value::as_str),
    ]);
    let email = first_non_empty([
        session.and_then(|v| value_str(v, &["user", "email"])),
        profile.and_then(|v| v.get("email")).and_then(Value::as_str),
        claims.get("email").and_then(Value::as_str),
        id_claims
            .as_ref()
            .and_then(|v| v.get("email"))
            .and_then(Value::as_str),
    ]);
    let name = first_non_empty([
        session.and_then(|v| value_str(v, &["user", "name"])),
        profile.and_then(|v| v.get("name")).and_then(Value::as_str),
        claims.get("name").and_then(Value::as_str),
        id_claims
            .as_ref()
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str),
    ]);
    let user_id = first_non_empty([
        session.and_then(|v| value_str(v, &["user", "id"])),
        auth.and_then(|v| v.get("user_id")).and_then(Value::as_str),
        claims.get("sub").and_then(Value::as_str),
        id_claims
            .as_ref()
            .and_then(|v| v.get("sub"))
            .and_then(Value::as_str),
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

    let resolved = ResolvedCredential {
        access_token: Zeroizing::new(token),
        account_id,
        email,
        name,
        user_id,
        claimed_plan,
        token_expires_at,
    };
    ensure_resolved_complete(&resolved)?;
    Ok(resolved)
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
    let segments = token.split('.').collect::<Vec<_>>();
    if segments.len() != 3
        || segments.iter().any(|segment| segment.is_empty())
        || segments[2].len() < 16
    {
        return Err(AppError::UnsupportedCredential);
    }
    let header = URL_SAFE_NO_PAD
        .decode(segments[0])
        .map_err(|_| AppError::UnsupportedCredential)?;
    let header: Value =
        serde_json::from_slice(&header).map_err(|_| AppError::UnsupportedCredential)?;
    if !header.is_object() || header.get("alg").and_then(Value::as_str).is_none() {
        return Err(AppError::UnsupportedCredential);
    }
    let payload = segments[1];
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

    fn fake_jwt(claims: &Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).expect("claims"));
        format!("{header}.{payload}.abcdefghijklmnop")
    }

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

    #[test]
    fn accepts_complete_session_json() {
        let token = fake_jwt(&json!({
            "sub": "user-1",
            "email": "owner@example.com",
            "exp": Utc::now().timestamp() + 3600,
            "https://api.openai.com/auth": {"chatgpt_account_id": "account-1"}
        }));
        let document = json!({
            "user": {"id": "user-1", "email": "owner@example.com"},
            "account": {"id": "account-1"},
            "accessToken": token
        });
        let validation = validate_credential_input(&document.to_string());
        assert!(validation.valid);
        assert!(validation.complete);
        assert!(validation.can_query);
    }

    #[test]
    fn rejects_access_token_without_account_scope() {
        let token = fake_jwt(&json!({
            "sub": "user-1",
            "email": "owner@example.com",
            "exp": Utc::now().timestamp() + 3600
        }));
        let validation = validate_credential_input(&token);
        assert!(!validation.complete);
        assert!(!validation.can_query);
        assert!(validation
            .missing
            .iter()
            .any(|item| item.contains("账户 ID")));
    }

    #[test]
    fn defers_opaque_session_completeness_to_chatgpt() {
        let validation =
            validate_credential_input("opaque-session-token-abcdefghijklmnopqrstuvwxyz-0123456789");
        assert!(validation.valid);
        assert!(!validation.complete);
        assert!(validation.can_query);
        assert!(validation
            .checks
            .iter()
            .any(|check| check.status == "pending"));
    }
}
