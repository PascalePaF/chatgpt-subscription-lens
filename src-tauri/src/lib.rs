mod credential;
mod error;
mod models;
mod normalize;
mod upstream;

use models::{AppInfo, CredentialValidation, InspectionRequest, InspectionResult};
use zeroize::Zeroizing;

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "订阅镜",
        version: env!("CARGO_PKG_VERSION"),
        local_only: true,
        read_only: true,
        allowed_hosts: ["chatgpt.com"],
    }
}

#[tauri::command]
fn validate_credential(credential: String) -> CredentialValidation {
    let protected_input = Zeroizing::new(credential);
    credential::validate_credential_input(protected_input.as_str())
}

#[tauri::command]
async fn inspect_subscription(
    request: InspectionRequest,
) -> Result<InspectionResult, error::AppError> {
    let protected_input = Zeroizing::new(request.credential);
    let validation = credential::validate_credential_input(protected_input.as_str());
    if !validation.can_query {
        return Err(error::AppError::IncompleteCredential(
            validation.missing.join("、"),
        ));
    }
    let client = upstream::build_client()?;
    let credential = credential::resolve_credential(&client, protected_input.as_str()).await?;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            validate_credential,
            inspect_subscription
        ])
        .setup(|app| {
            let executable = std::env::current_exe()?;
            let install_directory = executable
                .parent()
                .ok_or_else(|| std::io::Error::other("无法确定程序所在目录"))?;
            let webview_data = install_directory
                .join("subscription-lens-data")
                .join("WebView2");
            std::fs::create_dir_all(&webview_data)?;

            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("订阅镜 · ChatGPT 订阅查询")
            .inner_size(1240.0, 820.0)
            .min_inner_size(960.0, 680.0)
            .resizable(true)
            .center()
            .data_directory(webview_data)
            .incognito(true)
            .enable_clipboard_access()
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run 订阅镜");
}
