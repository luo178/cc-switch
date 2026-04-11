use tauri::State;
use std::sync::Arc;
use tauri_plugin_opener::OpenerExt;

use crate::proxy::providers::oauth::{
    BrowserFlowResponse, OAuthAccountInfo, OAuthAuthStatus, OAuthDeviceCodeResponse, OAuthError, OAuthManager, OAuthProviderId,
};

/// OAuth 认证状态
pub struct OAuthAuthState(pub Arc<OAuthManager>);

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthAccount {
    pub id: String,
    pub provider: String,
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
}

impl From<&OAuthAccountInfo> for ManagedAuthAccount {
    fn from(info: &OAuthAccountInfo) -> Self {
        Self {
            id: info.id.clone(),
            provider: info.provider.clone(),
            login: info.login.clone(),
            email: info.email.clone(),
            avatar_url: info.avatar_url.clone(),
            authenticated_at: info.authenticated_at,
            is_default: info.is_default,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_error: Option<String>,
    pub accounts: Vec<ManagedAuthAccount>,
}

impl From<&OAuthAuthStatus> for ManagedAuthStatus {
    fn from(status: &OAuthAuthStatus) -> Self {
        Self {
            provider: status.provider.as_str().to_string(),
            authenticated: status.authenticated,
            default_account_id: status.default_account_id.clone(),
            migration_error: None,
            accounts: status.accounts.iter().map(ManagedAuthAccount::from).collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

impl From<&OAuthDeviceCodeResponse> for ManagedAuthDeviceCodeResponse {
    fn from(resp: &OAuthDeviceCodeResponse) -> Self {
        Self {
            provider: resp.provider.clone(),
            device_code: resp.device_code.clone(),
            user_code: resp.user_code.clone(),
            verification_uri: resp.verification_uri.clone(),
            expires_in: resp.expires_in,
            interval: resp.interval,
        }
    }
}

/// 将提供商字符串转换为 OAuthProviderId
fn parse_provider_id(provider: &str) -> Result<OAuthProviderId, String> {
    provider.parse::<OAuthProviderId>().map_err(|e| e.to_string())
}

fn map_oauth_error(e: OAuthError) -> String {
    match e {
        OAuthError::AuthorizationPending => "authorization_pending".to_string(),
        OAuthError::AccessDenied => "access_denied".to_string(),
        OAuthError::ExpiredToken => "expired_token".to_string(),
        OAuthError::NoSubscription => "no_subscription".to_string(),
        _ => e.to_string(),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_start_login(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<ManagedAuthDeviceCodeResponse, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let resp = state
        .0
        .start_login(provider_id)
        .await
        .map_err(map_oauth_error)?;
    Ok(ManagedAuthDeviceCodeResponse::from(&resp))
}

/// OAuth 浏览器流程响应（自动打开浏览器）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedBrowserFlowResponse {
    pub provider: String,
    pub authorization_url: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_verifier: String,
    pub auto_open_browser: bool,
}

impl From<&BrowserFlowResponse> for ManagedBrowserFlowResponse {
    fn from(resp: &BrowserFlowResponse) -> Self {
        Self {
            provider: resp.provider.clone(),
            authorization_url: resp.authorization_url.clone(),
            redirect_uri: resp.redirect_uri.clone(),
            state: resp.state.clone(),
            code_verifier: resp.code_verifier.clone(),
            auto_open_browser: resp.auto_open_browser,
        }
    }
}

/// 启动 OAuth 浏览器流程（自动打开浏览器）
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_start_browser_flow(
    auth_provider: String,
    auto_open_browser: Option<bool>,
    state: State<'_, OAuthAuthState>,
    app: tauri::AppHandle,
) -> Result<ManagedBrowserFlowResponse, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let resp = state
        .0
        .start_browser_flow(provider_id)
        .await?;

    // 根据参数决定是否自动打开浏览器
    let should_open = auto_open_browser.unwrap_or(true);
    if should_open {
        if let Err(e) = app.opener().open_url(&resp.authorization_url, None::<String>) {
            log::warn!("[auth_start_browser_flow] Failed to auto-open browser: {}", e);
        }
    }

    Ok(ManagedBrowserFlowResponse::from(&resp))
}

/// 完成 OAuth 浏览器流程（等待回调）
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_complete_browser_flow(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<Option<ManagedAuthAccount>, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let account = state
        .0
        .complete_browser_flow(provider_id)
        .await
        .map_err(map_oauth_error)?;
    Ok(account.map(|a| ManagedAuthAccount::from(&a)))
}

/// Headless 模式：使用回调 URL 完成认证
/// 用户手动在浏览器中完成 OAuth 授权后，将回调 URL 粘贴到这里
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_complete_with_callback_url(
    auth_provider: String,
    callback_url: String,
    state: State<'_, OAuthAuthState>,
) -> Result<Option<ManagedAuthAccount>, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let account = state
        .0
        .complete_browser_flow_with_url(provider_id, &callback_url)
        .await
        .map_err(map_oauth_error)?;
    Ok(account.map(|a| ManagedAuthAccount::from(&a)))
}

/// 取消 OAuth 浏览器流程
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_cancel_browser_flow(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state.0.cancel_browser_flow(provider_id).await;
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_poll_for_account(
    auth_provider: String,
    device_code: String,
    state: State<'_, OAuthAuthState>,
) -> Result<Option<ManagedAuthAccount>, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let account = state
        .0
        .poll_for_account(provider_id, &device_code)
        .await
        .map_err(map_oauth_error)?;
    Ok(account.map(|a| ManagedAuthAccount::from(&a)))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_list_accounts(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<Vec<ManagedAuthAccount>, String> {
    let provider_id = parse_provider_id(&auth_provider)?;

    let accounts = state.0.list_accounts(provider_id).await;
    Ok(accounts.iter().map(ManagedAuthAccount::from).collect())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_get_status(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<ManagedAuthStatus, String> {
    let provider_id = parse_provider_id(&auth_provider)?;

    let status = state.0.get_auth_status(provider_id).await;
    Ok(ManagedAuthStatus::from(&status))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_remove_account(
    auth_provider: String,
    account_id: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state
        .0
        .remove_account(provider_id, &account_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_set_default_account(
    auth_provider: String,
    account_id: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state
        .0
        .set_default_account(provider_id, &account_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_logout(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state
        .0
        .clear_auth(provider_id)
        .await
        .map_err(|e| e.to_string())
}

/// 列出所有支持的 OAuth 提供商
#[tauri::command(rename_all = "camelCase")]
pub fn auth_list_providers() -> Vec<OAuthProviderInfo> {
    OAuthProviderId::all()
        .iter()
        .map(|id| OAuthProviderInfo {
            id: id.as_str().to_string(),
            name: id.display_name().to_string(),
            supports_device_code: id.supports_device_code(),
            requires_token_exchange: id.requires_token_exchange(),
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthProviderInfo {
    pub id: String,
    pub name: String,
    pub supports_device_code: bool,
    pub requires_token_exchange: bool,
}

/// 保存 OAuth Client ID
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_save_client_id(
    provider_id: String,
    client_id: String,
) -> Result<(), String> {
    crate::settings::set_oauth_client_id(&provider_id, client_id)
        .map_err(|e| e.to_string())
}

/// 移除 OAuth Client ID
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_remove_client_id(provider_id: String) -> Result<(), String> {
    crate::settings::remove_oauth_client_id(&provider_id)
        .map_err(|e| e.to_string())
}

/// 获取所有配置的 Client ID（仅返回已配置的，不返回占位符）
#[tauri::command(rename_all = "camelCase")]
pub fn auth_list_client_ids() -> std::collections::HashMap<String, String> {
    let mut result = std::collections::HashMap::new();
    for id in OAuthProviderId::all() {
        if let Some(client_id) = crate::settings::get_oauth_client_id(id.as_str()) {
            result.insert(id.as_str().to_string(), client_id);
        }
    }
    result
}
