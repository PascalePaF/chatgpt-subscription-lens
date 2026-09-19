use std::cmp::Ordering;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

use crate::error::AppError;
use crate::models::{
    AccountIdentity, BillingRecord, CoverageInfo, InspectionResult, PaymentMethodSummary,
    ResolvedCredential, SourceStatus, SubscriptionSummary, UsageSummary, UsageWindow,
};
use crate::upstream::{FetchOutcome, RawSources};

pub fn resolve_account_id_from_check(payload: &Value, preferred: Option<&str>) -> Option<String> {
    select_account_entry(payload, preferred).and_then(|(key, entry)| {
        string_at(entry, &["account", "account_id"])
            .or_else(|| string_at(entry, &["account", "id"]))
            .map(ToOwned::to_owned)
            .or_else(|| (key != "default").then(|| key.to_string()))
    })
}

pub fn build_result(
    credential: ResolvedCredential,
    raw: RawSources,
) -> Result<InspectionResult, AppError> {
    if !raw.account_check.success()
        && !raw.portal.success()
        && !raw.invoices.success()
        && !raw.payment_methods.success()
        && !raw.usage.success()
    {
        let reasons = [
            raw.account_check.error.as_deref(),
            raw.portal.error.as_deref(),
            raw.invoices.error.as_deref(),
            raw.payment_methods.error.as_deref(),
            raw.usage.error.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("；");
        return Err(AppError::NoSources(reasons));
    }

    let account_entry = raw.account_check.value.as_ref().and_then(|payload| {
        select_account_entry(payload, raw.resolved_account_id.as_deref()).map(|(_, entry)| entry)
    });
    let account = account_entry.and_then(|entry| entry.get("account"));
    let entitlement = account_entry.and_then(|entry| entry.get("entitlement"));
    let last_active = account_entry.and_then(|entry| entry.get("last_active_subscription"));
    let portal = raw.portal.value.as_ref();

    let mut invoices = raw
        .invoices
        .value
        .as_ref()
        .map(normalize_invoices)
        .unwrap_or_default();
    invoices.sort_by(compare_records_desc);
    let payment_methods = raw
        .payment_methods
        .value
        .as_ref()
        .map(normalize_payment_methods)
        .unwrap_or_default();

    let identity = AccountIdentity {
        name: credential.name.clone(),
        email: credential.email.clone(),
        user_id: credential.user_id.clone(),
        account_id: raw
            .resolved_account_id
            .clone()
            .or_else(|| credential.account_id.clone()),
        account_created_at: account
            .and_then(|value| time_at(value, &["created_time"]))
            .or_else(|| account.and_then(|value| time_at(value, &["created_at"]))),
        token_expires_at: credential.token_expires_at.clone(),
        claimed_plan: credential.claimed_plan.clone(),
        has_previously_paid: account
            .and_then(|value| bool_at(value, &["has_previously_paid_subscription"])),
        deactivated: account.and_then(|value| bool_at(value, &["is_deactivated"])),
    };

    let mut subscription = normalize_subscription(
        portal,
        account,
        entitlement,
        last_active,
        credential.claimed_plan.as_deref(),
    );
    if let Some(invoice) = invoices
        .iter()
        .find(|record| record.amount.is_some() && !is_failed_status(&record.status))
    {
        subscription.current_amount = invoice.amount;
        subscription.current_amount_source = Some("最近一张网页账单".into());
        if subscription.currency.is_none() {
            subscription.currency = invoice.currency.clone();
        }
    }

    let usage = raw
        .usage
        .value
        .as_ref()
        .map(normalize_usage)
        .unwrap_or_else(|| UsageSummary {
            status: "unavailable".into(),
            message: raw.usage.error.clone(),
            ..Default::default()
        });
    let mobile_records = normalize_mobile_records(&subscription, last_active);

    let mut warnings = Vec::new();
    append_source_warning(&mut warnings, "账户订阅", &raw.account_check);
    append_source_warning(&mut warnings, "订阅周期", &raw.portal);
    append_source_warning(&mut warnings, "网页账单", &raw.invoices);
    append_source_warning(&mut warnings, "支付方式", &raw.payment_methods);
    append_source_warning(&mut warnings, "Codex 额度", &raw.usage);
    if subscription
        .purchase_origin
        .as_deref()
        .is_some_and(is_mobile_origin)
    {
        warnings.push(
            "Apple/Google 的完整逐笔收据由对应商店管理；这里的移动端记录仅代表 ChatGPT 账户端点最后一次可确认的订阅。"
                .into(),
        );
    }
    warnings.push(
        "订阅和账单端点属于 ChatGPT 产品内部接口，不是 OpenAI 承诺稳定的公开开发者 API，未来可能变化。"
            .into(),
    );

    let sources = vec![
        source_status(
            "account-check",
            "账户与权益",
            &raw.account_check,
            "ChatGPT 当前账户状态",
        ),
        source_status(
            "subscriptions",
            "订阅周期",
            &raw.portal,
            "当前周期、续费与币种",
        ),
        source_status(
            "invoices",
            "网页账单",
            &raw.invoices,
            "chatgpt.com 直购账单历史",
        ),
        source_status(
            "payment-methods",
            "支付方式",
            &raw.payment_methods,
            "银行卡品牌与脱敏卡号",
        ),
        source_status("usage", "Codex 额度", &raw.usage, "滚动额度窗口与恢复时间"),
    ];

    Ok(InspectionResult {
        queried_at: Utc::now().to_rfc3339(),
        identity,
        subscription,
        usage,
        invoices,
        mobile_records,
        payment_methods,
        sources,
        warnings,
        coverage: CoverageInfo::default(),
    })
}

fn normalize_payment_methods(payload: &Value) -> Vec<PaymentMethodSummary> {
    let default_id = first_owned([
        string_at(payload, &["default_payment_method_id"]),
        string_at(payload, &["default_payment_method"]),
        string_at(payload, &["defaultPaymentMethodId"]),
    ]);
    let items: Vec<&Value> = if let Some(array) = payload.as_array() {
        array.iter().collect()
    } else if let Some(array) = ["payment_methods", "data", "items", "cards"]
        .into_iter()
        .find_map(|key| payload.get(key).and_then(Value::as_array))
    {
        array.iter().collect()
    } else if let Some(array) = payload
        .get("payment_methods")
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
    {
        array.iter().collect()
    } else {
        payload
            .get("payment_method")
            .filter(|value| value.is_object())
            .into_iter()
            .collect()
    };

    let mut methods = items
        .into_iter()
        .filter_map(|item| {
            let card = item.get("card").unwrap_or(item);
            let kind = first_owned([string_at(item, &["type"]), string_at(card, &["type"])])
                .unwrap_or_else(|| "card".into());
            if !kind.to_ascii_lowercase().contains("card") && item.get("card").is_none() {
                return None;
            }
            let brand = first_owned([
                string_at(card, &["brand"]),
                string_at(card, &["network"]),
                string_at(card, &["display_brand"]),
                string_at(item, &["brand"]),
            ])
            .map(|value| normalize_card_brand(&value))
            .unwrap_or_else(|| "card".into());
            let first6 = ["iin", "bin", "first6", "first_6", "card_bin"]
                .into_iter()
                .find_map(|key| digit_prefix_at(card, &[key], 6))
                .or_else(|| {
                    ["iin", "bin", "first6", "first_6", "card_bin"]
                        .into_iter()
                        .find_map(|key| digit_prefix_at(item, &[key], 6))
                });
            let last4 = ["last4", "last_4", "display_last4", "card_last4"]
                .into_iter()
                .find_map(|key| digit_suffix_at(card, &[key], 4))
                .or_else(|| {
                    ["last4", "last_4", "display_last4", "card_last4"]
                        .into_iter()
                        .find_map(|key| digit_suffix_at(item, &[key], 4))
                });
            if brand == "card" && first6.is_none() && last4.is_none() {
                return None;
            }
            let item_id = string_at(item, &["id"]);
            let is_default = bool_at(item, &["is_default"])
                .or_else(|| bool_at(item, &["default"]))
                .unwrap_or_else(|| {
                    item_id
                        .zip(default_id.as_deref())
                        .is_some_and(|(left, right)| left == right)
                });
            Some(PaymentMethodSummary {
                kind: "card".into(),
                brand,
                first6,
                last4,
                exp_month: i64_at(card, &["exp_month"])
                    .filter(|value| (1..=12).contains(value))
                    .map(|value| value as u32),
                exp_year: i64_at(card, &["exp_year"])
                    .filter(|value| (2000..=9999).contains(value))
                    .map(|value| value as u32),
                is_default,
                source_note: "ChatGPT 支付方式端点；仅保留品牌、前 6 位和尾号 4 位".into(),
            })
        })
        .collect::<Vec<_>>();
    methods.sort_by_key(|method| !method.is_default);
    methods
}

fn normalize_card_brand(value: &str) -> String {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_', '-'], "");
    match normalized.as_str() {
        "visa" => "visa".into(),
        "mastercard" | "master" | "mc" => "mastercard".into(),
        "amex" | "americanexpress" => "amex".into(),
        "unionpay" => "unionpay".into(),
        "jcb" => "jcb".into(),
        "discover" => "discover".into(),
        _ => "card".into(),
    }
}

fn digit_prefix_at(value: &Value, path: &[&str], length: usize) -> Option<String> {
    let text = scalar_text_at(value, path)?;
    let digits = text
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (digits.len() >= length).then(|| digits.chars().take(length).collect())
}

fn digit_suffix_at(value: &Value, path: &[&str], length: usize) -> Option<String> {
    let text = scalar_text_at(value, path)?;
    let digits = text
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (digits.len() >= length).then(|| {
        digits
            .chars()
            .rev()
            .take(length)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    })
}

fn scalar_text_at(value: &Value, path: &[&str]) -> Option<String> {
    let value = value_at(value, path)?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_u64().map(|number| number.to_string()))
}

fn select_account_entry<'a>(
    payload: &'a Value,
    preferred: Option<&str>,
) -> Option<(&'a str, &'a Value)> {
    let accounts = payload.get("accounts")?.as_object()?;
    if let Some(preferred) = preferred {
        if let Some(entry) = accounts.get(preferred) {
            if !entry_is_deactivated(entry) {
                return Some((accounts.get_key_value(preferred)?.0.as_str(), entry));
            }
        }
    }
    if let Some((key, entry)) = accounts.get_key_value("default") {
        if !entry_is_deactivated(entry) {
            return Some((key.as_str(), entry));
        }
    }
    accounts
        .iter()
        .find(|(_, entry)| {
            !entry_is_deactivated(entry)
                && bool_at(entry, &["entitlement", "has_active_subscription"]) == Some(true)
        })
        .or_else(|| {
            accounts
                .iter()
                .find(|(_, entry)| !entry_is_deactivated(entry))
        })
        .map(|(key, value)| (key.as_str(), value))
}

fn entry_is_deactivated(entry: &Value) -> bool {
    bool_at(entry, &["account", "is_deactivated"]) == Some(true)
}

fn normalize_subscription(
    portal: Option<&Value>,
    account: Option<&Value>,
    entitlement: Option<&Value>,
    last_active: Option<&Value>,
    claimed_plan: Option<&str>,
) -> SubscriptionSummary {
    let plan_code = first_owned([
        portal.and_then(|value| string_at(value, &["plan_type"])),
        account.and_then(|value| string_at(value, &["plan_type"])),
        entitlement.and_then(|value| string_at(value, &["subscription_plan"])),
        claimed_plan,
    ]);
    let active = entitlement
        .and_then(|value| bool_at(value, &["has_active_subscription"]))
        .unwrap_or_else(|| {
            portal
                .and_then(|value| time_at(value, &["active_until"]))
                .and_then(|value| parse_time(&value))
                .is_some_and(|value| value > Utc::now())
        });
    let active_start = portal
        .and_then(|value| time_at(value, &["active_start"]))
        .or_else(|| entitlement.and_then(|value| time_at(value, &["purchase_date"])));
    let active_until = portal
        .and_then(|value| time_at(value, &["active_until"]))
        .or_else(|| entitlement.and_then(|value| time_at(value, &["expires_at"])))
        .or_else(|| entitlement.and_then(|value| time_at(value, &["renews_at"])));
    let remaining_seconds = active_until
        .as_deref()
        .and_then(parse_time)
        .map(|value| (value - Utc::now()).num_seconds().max(0));
    let delinquent = portal
        .and_then(|value| bool_at(value, &["is_delinquent"]))
        .or_else(|| entitlement.and_then(|value| bool_at(value, &["is_delinquent"])))
        .unwrap_or(false);
    let has_previously_paid = account
        .and_then(|value| bool_at(value, &["has_previously_paid_subscription"]))
        .unwrap_or(false);
    let plan = display_plan(plan_code.as_deref(), active);
    let state = if delinquent {
        "delinquent"
    } else if active {
        "active"
    } else if has_previously_paid || !is_free_plan(plan_code.as_deref()) {
        "expired"
    } else {
        "free"
    };
    let free_reason = entitlement.and_then(free_reason);

    SubscriptionSummary {
        state: state.into(),
        plan,
        plan_code,
        active,
        active_start,
        active_until,
        remaining_seconds,
        will_renew: portal
            .and_then(|value| bool_at(value, &["will_renew"]))
            .or_else(|| last_active.and_then(|value| bool_at(value, &["will_renew"]))),
        purchase_origin: last_active
            .and_then(|value| string_at(value, &["purchase_origin_platform"]))
            .map(ToOwned::to_owned),
        billing_period: portal
            .and_then(|value| string_at(value, &["billing_period"]))
            .or_else(|| entitlement.and_then(|value| string_at(value, &["billing_period"])))
            .map(ToOwned::to_owned),
        currency: portal
            .and_then(|value| string_at(value, &["billing_currency"]))
            .or_else(|| entitlement.and_then(|value| string_at(value, &["billing_currency"])))
            .map(|value| value.to_uppercase()),
        current_amount: None,
        current_amount_source: None,
        delinquent,
        grace_period_end: portal
            .and_then(|value| time_at(value, &["grace_period_end_timestamp"]))
            .or_else(|| {
                entitlement.and_then(|value| time_at(value, &["grace_period_end_timestamp"]))
            }),
        free_reason,
        seats_in_use: portal.and_then(|value| i64_at(value, &["seats_in_use"])),
        seats_entitled: portal.and_then(|value| i64_at(value, &["seats_entitled"])),
    }
}

fn normalize_invoices(payload: &Value) -> Vec<BillingRecord> {
    let items: &[Value] = if let Some(array) = payload.as_array() {
        array.as_slice()
    } else {
        ["invoices", "data", "items", "transactions"]
            .into_iter()
            .find_map(|key| payload.get(key).and_then(Value::as_array))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    };
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| normalize_invoice(item, index))
        .collect()
}

fn normalize_invoice(item: &Value, index: usize) -> Option<BillingRecord> {
    if !item.is_object() {
        return None;
    }
    let id = string_at(item, &["id"])
        .or_else(|| string_at(item, &["invoice_id"]))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("invoice-{index}"));
    let amount = ["amount_paid", "total", "amount_due"]
        .into_iter()
        .find_map(|key| number_at(item, &[key]).map(|value| value / 100.0))
        .or_else(|| number_at(item, &["amount"]));
    let product = first_owned([
        string_at(item, &["description"]),
        string_at(item, &["product", "plan"]),
        string_at(item, &["product", "name"]),
        string_at(item, &["lines", "data", "0", "description"]),
        string_at(item, &["metadata", "plan_type"]),
    ])
    .unwrap_or_else(|| "ChatGPT 订阅".into());
    let status = first_owned([
        string_at(item, &["status"]),
        string_at(item, &["payment_status"]),
    ])
    .unwrap_or_else(|| "unknown".into());
    Some(BillingRecord {
        id,
        record_type: "web_invoice".into(),
        product,
        status,
        created_at: time_at(item, &["created_at"])
            .or_else(|| time_at(item, &["created"]))
            .or_else(|| time_at(item, &["paid_at"])),
        period_start: time_at(item, &["period_start"])
            .or_else(|| time_at(item, &["lines", "data", "0", "period", "start"])),
        period_end: time_at(item, &["period_end"])
            .or_else(|| time_at(item, &["lines", "data", "0", "period", "end"])),
        amount,
        currency: string_at(item, &["currency"]).map(|value| value.to_uppercase()),
        store: Some("chatgpt_web".into()),
        will_renew: None,
        refunded_at: time_at(item, &["refunded_at"]),
        source_note: Some("ChatGPT 网页账单端点".into()),
    })
}

fn normalize_mobile_records(
    subscription: &SubscriptionSummary,
    last_active: Option<&Value>,
) -> Vec<BillingRecord> {
    let Some(origin) = subscription.purchase_origin.as_deref() else {
        return Vec::new();
    };
    if !is_mobile_origin(origin) {
        return Vec::new();
    }
    let id = last_active
        .and_then(|value| string_at(value, &["subscription_id"]))
        .unwrap_or("last-mobile-subscription")
        .to_string();
    vec![BillingRecord {
        id,
        record_type: "mobile_last_known".into(),
        product: subscription.plan.clone(),
        status: subscription.state.clone(),
        created_at: subscription.active_start.clone(),
        period_start: subscription.active_start.clone(),
        period_end: subscription.active_until.clone(),
        amount: None,
        currency: subscription.currency.clone(),
        store: Some(origin.to_string()),
        will_renew: subscription.will_renew,
        refunded_at: None,
        source_note: Some("ChatGPT 账户端点可确认的最近移动订阅；不是商店完整购买历史".into()),
    }]
}

fn normalize_usage(payload: &Value) -> UsageSummary {
    let rate_limit = payload.get("rate_limit").unwrap_or(payload);
    let mut windows = Vec::new();
    for (key, fallback_label) in [
        ("primary_window", "主要额度窗口"),
        ("secondary_window", "次要额度窗口"),
    ] {
        if let Some(window) = rate_limit.get(key) {
            if !window.is_null() {
                if let Some(parsed) = normalize_window(window, fallback_label, None) {
                    windows.push(parsed);
                }
            }
        }
    }
    if let Some(additional) = payload
        .get("additional_rate_limits")
        .and_then(Value::as_array)
    {
        for (index, item) in additional.iter().enumerate() {
            let model = first_owned([
                string_at(item, &["model"]),
                string_at(item, &["limit_name"]),
                string_at(item, &["name"]),
            ]);
            let window = item
                .get("rate_limit")
                .and_then(|value| value.get("primary_window").or_else(|| value.get("window")))
                .or_else(|| item.get("primary_window"))
                .or_else(|| item.get("window"));
            if let Some(window) = window {
                let label = model
                    .as_deref()
                    .map(|value| format!("{value} 专属额度"))
                    .unwrap_or_else(|| format!("附加额度 {}", index + 1));
                if let Some(parsed) = normalize_window(window, &label, model) {
                    windows.push(parsed);
                }
            }
        }
    }
    let limit_reached = bool_at(rate_limit, &["limit_reached"]);
    UsageSummary {
        status: "available".into(),
        plan_type: string_at(payload, &["plan_type"]).map(ToOwned::to_owned),
        windows,
        credits: payload
            .get("credits")
            .filter(|value| !value.is_null())
            .cloned(),
        limit_reached,
        message: if limit_reached == Some(true) {
            Some("当前额度窗口已用尽".into())
        } else {
            None
        },
    }
}

fn normalize_window(
    window: &Value,
    fallback_label: &str,
    model: Option<String>,
) -> Option<UsageWindow> {
    let used =
        number_at(window, &["used_percent"]).or_else(|| number_at(window, &["used_percentage"]))?;
    let seconds =
        i64_at(window, &["limit_window_seconds"]).or_else(|| i64_at(window, &["window_seconds"]));
    let label = seconds
        .map(window_label)
        .unwrap_or_else(|| fallback_label.to_string());
    Some(UsageWindow {
        label,
        used_percent: used.clamp(0.0, 100.0),
        remaining_percent: (100.0 - used).clamp(0.0, 100.0),
        reset_at: time_at(window, &["reset_at"]),
        reset_after_seconds: i64_at(window, &["reset_after_seconds"]),
        window_seconds: seconds,
        model,
    })
}

fn source_status(id: &str, label: &str, outcome: &FetchOutcome, detail: &str) -> SourceStatus {
    SourceStatus {
        id: id.into(),
        label: label.into(),
        status: if outcome.success() {
            "ok"
        } else {
            "unavailable"
        }
        .into(),
        detail: outcome.error.clone().unwrap_or_else(|| {
            outcome
                .http_status
                .map(|status| format!("HTTP {status} · {detail}"))
                .unwrap_or_else(|| detail.to_string())
        }),
        official_public_api: false,
    }
}

fn append_source_warning(warnings: &mut Vec<String>, label: &str, source: &FetchOutcome) {
    if let Some(error) = source.error.as_deref() {
        warnings.push(format!("{label}未返回：{error}"));
    }
}

fn free_reason(entitlement: &Value) -> Option<String> {
    if bool_at(entitlement, &["is_active_subscription_gratis"]) == Some(true) {
        return Some("官方赠送期".into());
    }
    let discount = entitlement.get("discount")?;
    let kind = string_at(discount, &["discount_type"]);
    let amount = number_at(discount, &["amount"]);
    if kind == Some("percentage") && amount.is_some_and(|value| value >= 100.0) {
        return string_at(discount, &["promo_campaign_id"])
            .map(|value| format!("100% 优惠 · {value}"))
            .or_else(|| Some("100% 优惠".into()));
    }
    None
}

fn display_plan(code: Option<&str>, active: bool) -> String {
    let normalized = code.unwrap_or("").to_ascii_lowercase();
    if normalized.contains("prolite") || normalized.contains("pro5x") {
        "Pro 5X".into()
    } else if normalized == "pro" || normalized.contains("pro20x") || normalized.contains("proplan")
    {
        "Pro 20X".into()
    } else if normalized.contains("plus") {
        "Plus".into()
    } else if normalized.contains("business") {
        "Business".into()
    } else if normalized.contains("team") {
        "Team".into()
    } else if normalized.contains("enterprise") {
        "Enterprise".into()
    } else if normalized.contains("edu") {
        "Edu".into()
    } else if normalized.contains("go") {
        "Go".into()
    } else if normalized.is_empty() || normalized.contains("free") || !active {
        "Free".into()
    } else {
        code.unwrap_or("Free").to_string()
    }
}

fn is_free_plan(code: Option<&str>) -> bool {
    match code {
        None => true,
        Some(value) => value.is_empty() || value.to_ascii_lowercase().contains("free"),
    }
}

fn is_mobile_origin(origin: &str) -> bool {
    let origin = origin.to_ascii_lowercase();
    origin.contains("ios")
        || origin.contains("apple")
        || origin.contains("android")
        || origin.contains("google")
        || origin.contains("play_store")
        || origin.contains("app_store")
}

fn is_failed_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "failed" | "void" | "uncollectible" | "refunded"
    )
}

fn compare_records_desc(left: &BillingRecord, right: &BillingRecord) -> Ordering {
    let left_time = left.created_at.as_deref().and_then(parse_time);
    let right_time = right.created_at.as_deref().and_then(parse_time);
    right_time.cmp(&left_time)
}

fn window_label(seconds: i64) -> String {
    match seconds {
        1..=21_600 => format!("{} 小时滚动窗口", (seconds as f64 / 3600.0).round() as i64),
        21_601..=172_800 => format!("{} 天窗口", (seconds as f64 / 86_400.0).round() as i64),
        172_801..=691_200 => "每周窗口".into(),
        _ => format!("{} 天窗口", (seconds as f64 / 86_400.0).round() as i64),
    }
}

fn first_owned<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for part in path {
        current = if let Ok(index) = part.parse::<usize>() {
            current.as_array()?.get(index)?
        } else {
            current.get(*part)?
        };
    }
    Some(current)
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    value_at(value, path)?.as_str()
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    let value = value_at(value, path)?;
    value.as_bool().or_else(|| match value.as_str()? {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    })
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    let value = value_at(value, path)?;
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn i64_at(value: &Value, path: &[&str]) -> Option<i64> {
    let value = value_at(value, path)?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        .or_else(|| value.as_f64().map(|number| number as i64))
        .or_else(|| {
            value
                .as_str()
                .and_then(|text| text.parse::<f64>().ok())
                .map(|number| number as i64)
        })
}

fn time_at(value: &Value, path: &[&str]) -> Option<String> {
    let value = value_at(value, path)?;
    if let Some(text) = value.as_str() {
        return normalize_time_text(text);
    }
    if let Some(number) = value.as_i64().or_else(|| value.as_f64().map(|v| v as i64)) {
        return timestamp_to_iso(number);
    }
    None
}

fn normalize_time_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(number) = text.parse::<f64>() {
        return timestamp_to_iso(number as i64);
    }
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .or_else(|| Some(text.to_string()))
}

fn timestamp_to_iso(mut timestamp: i64) -> Option<String> {
    if timestamp.abs() > 10_000_000_000 {
        timestamp /= 1000;
    }
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|value| value.to_rfc3339())
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn selects_preferred_account_and_normalizes_plan() {
        let payload = json!({
            "accounts": {
                "default": {
                    "account": {"account_id": "personal", "plan_type": "free"},
                    "entitlement": {"has_active_subscription": false}
                },
                "team-id": {
                    "account": {"account_id": "team-id", "plan_type": "business"},
                    "entitlement": {"has_active_subscription": true}
                }
            }
        });
        assert_eq!(
            resolve_account_id_from_check(&payload, Some("team-id")),
            Some("team-id".into())
        );
    }

    #[test]
    fn parses_invoice_minor_units() {
        let payload = json!({
            "data": [{
                "id": "in_1",
                "status": "paid",
                "amount_paid": 2000,
                "currency": "usd",
                "created": 1760000000,
                "description": "ChatGPT Plus"
            }]
        });
        let records = normalize_invoices(&payload);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].amount, Some(20.0));
        assert_eq!(records[0].currency.as_deref(), Some("USD"));
    }

    #[test]
    fn labels_weekly_window() {
        let window = json!({
            "used_percent": 13.5,
            "limit_window_seconds": 604800,
            "reset_at": 1760000000
        });
        let normalized = normalize_window(&window, "fallback", None).expect("window");
        assert_eq!(normalized.label, "每周窗口");
        assert_eq!(normalized.remaining_percent, 86.5);
    }

    #[test]
    fn normalizes_and_masks_payment_methods() {
        let payload = json!({
            "default_payment_method_id": "pm_master",
            "payment_methods": [
                {
                    "id": "pm_visa",
                    "type": "card",
                    "card": {"brand": "Visa", "last4": "4242", "exp_month": 12, "exp_year": 2030}
                },
                {
                    "id": "pm_master",
                    "type": "card",
                    "card": {"brand": "MasterCard", "iin": "55555599", "last4": "4444"}
                }
            ]
        });
        let methods = normalize_payment_methods(&payload);
        assert_eq!(methods.len(), 2);
        assert!(methods[0].is_default);
        assert_eq!(methods[0].brand, "mastercard");
        assert_eq!(methods[0].first6.as_deref(), Some("555555"));
        assert_eq!(methods[0].last4.as_deref(), Some("4444"));
        assert_eq!(methods[1].brand, "visa");
        assert_eq!(methods[1].first6, None);
    }
}
