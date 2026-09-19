mod credential;
mod error;
pub mod models;
mod normalize;
mod ui;
mod upstream;

use models::{CredentialValidation, InspectionRequest, InspectionResult};
use zeroize::Zeroizing;

pub use error::AppError;

pub fn validate_credential(credential: &str) -> CredentialValidation {
    credential::validate_credential_input(credential)
}

pub async fn inspect_subscription(
    request: InspectionRequest,
) -> Result<InspectionResult, AppError> {
    let protected_input = Zeroizing::new(request.credential);
    let validation = credential::validate_credential_input(protected_input.as_str());
    if !validation.can_query {
        let missing = if validation.missing.is_empty() {
            validation.summary
        } else {
            validation.missing.join("、")
        };
        return Err(AppError::IncompleteCredential(missing));
    }

    let client = upstream::build_client()?;
    let credential = credential::resolve_credential(&client, protected_input.as_str()).await?;
    credential::ensure_resolved_complete(&credential)?;
    let timezone_offset = request.timezone_offset_min.unwrap_or(0).clamp(-840, 840);
    let raw = upstream::fetch_all(
        &client,
        credential.access_token.as_str(),
        credential.account_id.as_deref(),
        timezone_offset,
    )
    .await;
    normalize::build_result(credential, raw)
}

pub fn run() -> eframe::Result {
    ui::run_native_app()
}
