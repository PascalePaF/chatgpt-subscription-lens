use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionRequest {
    pub credential: String,
    #[serde(default)]
    pub timezone_offset_min: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub local_only: bool,
    pub read_only: bool,
    pub allowed_hosts: [&'static str; 1],
}

#[derive(Debug)]
pub struct ResolvedCredential {
    pub access_token: Zeroizing<String>,
    pub account_id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub claimed_plan: Option<String>,
    pub token_expires_at: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InspectionResult {
    pub queried_at: String,
    pub identity: AccountIdentity,
    pub subscription: SubscriptionSummary,
    pub usage: UsageSummary,
    pub invoices: Vec<BillingRecord>,
    pub mobile_records: Vec<BillingRecord>,
    pub payment_methods: Vec<PaymentMethodSummary>,
    pub sources: Vec<SourceStatus>,
    pub warnings: Vec<String>,
    pub coverage: CoverageInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialValidation {
    pub kind: String,
    pub valid: bool,
    pub complete: bool,
    pub can_query: bool,
    pub summary: String,
    pub checks: Vec<CredentialCheck>,
    pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialCheck {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountIdentity {
    pub name: Option<String>,
    pub email: Option<String>,
    pub user_id: Option<String>,
    pub account_id: Option<String>,
    pub account_created_at: Option<String>,
    pub token_expires_at: Option<String>,
    pub claimed_plan: Option<String>,
    pub has_previously_paid: Option<bool>,
    pub deactivated: Option<bool>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionSummary {
    pub state: String,
    pub plan: String,
    pub plan_code: Option<String>,
    pub active: bool,
    pub active_start: Option<String>,
    pub active_until: Option<String>,
    pub remaining_seconds: Option<i64>,
    pub will_renew: Option<bool>,
    pub purchase_origin: Option<String>,
    pub billing_period: Option<String>,
    pub currency: Option<String>,
    pub current_amount: Option<f64>,
    pub current_amount_source: Option<String>,
    pub delinquent: bool,
    pub grace_period_end: Option<String>,
    pub free_reason: Option<String>,
    pub seats_in_use: Option<i64>,
    pub seats_entitled: Option<i64>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub status: String,
    pub plan_type: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub credits: Option<Value>,
    pub limit_reached: Option<bool>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub reset_at: Option<String>,
    pub reset_after_seconds: Option<i64>,
    pub window_seconds: Option<i64>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct BillingRecord {
    pub id: String,
    pub record_type: String,
    pub product: String,
    pub status: String,
    pub created_at: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub store: Option<String>,
    pub will_renew: Option<bool>,
    pub refunded_at: Option<String>,
    pub source_note: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodSummary {
    pub kind: String,
    pub brand: String,
    pub first6: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<u32>,
    pub exp_year: Option<u32>,
    pub is_default: bool,
    pub source_note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub official_public_api: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageInfo {
    pub current_subscription: String,
    pub web_invoice_history: String,
    pub mobile_history: String,
    pub quota: String,
}

impl Default for CoverageInfo {
    fn default() -> Self {
        Self {
            current_subscription: "通过 ChatGPT 当前账户端点核对".into(),
            web_invoice_history: "仅显示端点实际返回的网页账单".into(),
            mobile_history:
                "OpenAI 未提供公开的 Apple/Google 完整账单 API；仅显示账户返回的最近移动订阅记录"
                    .into(),
            quota: "通过 Codex 客户端使用的配额端点核对".into(),
        }
    }
}
