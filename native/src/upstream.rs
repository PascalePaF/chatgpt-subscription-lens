use std::time::Duration;

use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, AUTHORIZATION, REFERER,
    USER_AGENT,
};
use serde_json::Value;
use url::Url;

use crate::error::AppError;
use crate::normalize::resolve_account_id_from_check;

const ACCOUNT_CHECK_URL: &str = "https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27";
const SUBSCRIPTIONS_URL: &str = "https://chatgpt.com/backend-api/subscriptions";
const INVOICES_URL: &str = "https://chatgpt.com/backend-api/invoices";
const PAYMENT_METHODS_URL: &str = "https://chatgpt.com/backend-api/payments/payment_methods";
const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
const CODEX_UA: &str = "codex_cli_rs/1.0.0 (Windows 10.0.0; x86_64) subscription-lens/1.1.2";
const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct RawSources {
    pub account_check: FetchOutcome,
    pub portal: FetchOutcome,
    pub invoices: FetchOutcome,
    pub payment_methods: FetchOutcome,
    pub usage: FetchOutcome,
    pub resolved_account_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct FetchOutcome {
    pub value: Option<Value>,
    pub error: Option<String>,
    pub http_status: Option<u16>,
}

impl FetchOutcome {
    pub fn success(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Clone, Copy)]
enum RequestProfile {
    Browser,
    Codex,
}

pub fn build_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| AppError::Client(error.to_string()))
}

pub async fn fetch_all(
    client: &reqwest::Client,
    access_token: &str,
    preferred_account_id: Option<&str>,
    timezone_offset_min: i32,
) -> RawSources {
    let mut check_url = Url::parse(ACCOUNT_CHECK_URL).expect("fixed account URL");
    check_url
        .query_pairs_mut()
        .append_pair("timezone_offset_min", &timezone_offset_min.to_string());
    let account_check = fetch_json(
        client,
        check_url,
        access_token,
        preferred_account_id,
        RequestProfile::Browser,
    )
    .await;

    let resolved_account_id = account_check
        .value
        .as_ref()
        .and_then(|value| resolve_account_id_from_check(value, preferred_account_id))
        .or_else(|| preferred_account_id.map(ToOwned::to_owned));

    let usage_future = fetch_json(
        client,
        Url::parse(USAGE_URL).expect("fixed usage URL"),
        access_token,
        resolved_account_id.as_deref(),
        RequestProfile::Codex,
    );

    let (portal, invoices, payment_methods, usage) =
        if let Some(account_id) = resolved_account_id.as_deref() {
            let portal_url = scoped_url(SUBSCRIPTIONS_URL, account_id);
            let invoices_url = scoped_url(INVOICES_URL, account_id);
            let payment_methods_url = scoped_url(PAYMENT_METHODS_URL, account_id);
            tokio::join!(
                fetch_json(
                    client,
                    portal_url,
                    access_token,
                    Some(account_id),
                    RequestProfile::Browser,
                ),
                fetch_json(
                    client,
                    invoices_url,
                    access_token,
                    Some(account_id),
                    RequestProfile::Browser,
                ),
                fetch_json(
                    client,
                    payment_methods_url,
                    access_token,
                    Some(account_id),
                    RequestProfile::Browser,
                ),
                usage_future,
            )
        } else {
            let usage = usage_future.await;
            (
                FetchOutcome {
                    error: Some("Access Token 中没有 account_id，账户检查也未返回可用账号".into()),
                    ..Default::default()
                },
                FetchOutcome {
                    error: Some("缺少 account_id，无法查询网页账单".into()),
                    ..Default::default()
                },
                FetchOutcome {
                    error: Some("缺少 account_id，无法查询支付方式".into()),
                    ..Default::default()
                },
                usage,
            )
        };

    RawSources {
        account_check,
        portal,
        invoices,
        payment_methods,
        usage,
        resolved_account_id,
    }
}

fn scoped_url(base: &str, account_id: &str) -> Url {
    let mut url = Url::parse(base).expect("fixed scoped URL");
    url.query_pairs_mut().append_pair("account_id", account_id);
    url
}

async fn fetch_json(
    client: &reqwest::Client,
    url: Url,
    access_token: &str,
    account_id: Option<&str>,
    profile: RequestProfile,
) -> FetchOutcome {
    debug_assert_eq!(url.scheme(), "https");
    debug_assert_eq!(url.host_str(), Some("chatgpt.com"));

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    let authorization = match HeaderValue::from_str(&format!("Bearer {access_token}")) {
        Ok(value) => value,
        Err(_) => {
            return FetchOutcome {
                error: Some("Access Token 含有无效字符".into()),
                ..Default::default()
            }
        }
    };
    headers.insert(AUTHORIZATION, authorization);
    if let Some(account_id) = account_id {
        if let Ok(value) = HeaderValue::from_str(account_id) {
            headers.insert("chatgpt-account-id", value);
        }
    }

    match profile {
        RequestProfile::Browser => {
            headers.insert(USER_AGENT, HeaderValue::from_static(BROWSER_UA));
            headers.insert(
                ACCEPT_LANGUAGE,
                HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
            );
            headers.insert(REFERER, HeaderValue::from_static("https://chatgpt.com/"));
            headers.insert(
                "sec-ch-ua",
                HeaderValue::from_static(
                    "\"Chromium\";v=\"140\", \"Not=A?Brand\";v=\"24\", \"Google Chrome\";v=\"140\"",
                ),
            );
            headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
            headers.insert(
                "sec-ch-ua-platform",
                HeaderValue::from_static("\"Windows\""),
            );
            headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
            headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
            headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        }
        RequestProfile::Codex => {
            headers.insert(USER_AGENT, HeaderValue::from_static(CODEX_UA));
            headers.insert("originator", HeaderValue::from_static("codex_cli_rs"));
        }
    }

    let response = match client.get(url.clone()).headers(headers).send().await {
        Ok(response) => response,
        Err(error) => {
            return FetchOutcome {
                error: Some(network_error(&error)),
                ..Default::default()
            }
        }
    };
    let status = response.status();
    let status_code = status.as_u16();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return FetchOutcome {
            error: Some("响应体超过 5 MiB 安全上限".into()),
            http_status: Some(status_code),
            ..Default::default()
        };
    }
    let bytes = match response.bytes().await {
        Ok(bytes) if bytes.len() <= MAX_RESPONSE_BYTES => bytes,
        Ok(_) => {
            return FetchOutcome {
                error: Some("响应体超过 5 MiB 安全上限".into()),
                http_status: Some(status_code),
                ..Default::default()
            }
        }
        Err(error) => {
            return FetchOutcome {
                error: Some(network_error(&error)),
                http_status: Some(status_code),
                ..Default::default()
            }
        }
    };
    if !status.is_success() {
        return FetchOutcome {
            error: Some(status_error(status_code)),
            http_status: Some(status_code),
            ..Default::default()
        };
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => FetchOutcome {
            value: Some(value),
            http_status: Some(status_code),
            ..Default::default()
        },
        Err(_) => FetchOutcome {
            error: Some("响应不是有效 JSON".into()),
            http_status: Some(status_code),
            ..Default::default()
        },
    }
}

fn status_error(status: u16) -> String {
    match status {
        401 => "HTTP 401：凭证无效或已过期".into(),
        403 => "HTTP 403：ChatGPT 拒绝了该查询；可能是网络风控或接口权限变化".into(),
        404 => "HTTP 404：该内部端点当前不可用".into(),
        429 => "HTTP 429：请求过于频繁，请稍后再试".into(),
        500..=599 => format!("HTTP {status}：ChatGPT 服务暂时异常"),
        _ => format!("HTTP {status}"),
    }
}

fn network_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "连接 ChatGPT 超时".into()
    } else if error.is_connect() {
        "无法连接 ChatGPT，请检查网络或代理设置".into()
    } else {
        "网络传输失败".into()
    }
}
