//! Tauri 命令包装：把 OAuthManager 暴露给前端
//!
//! 这些命令由 OAuthProviderSection、useOAuthAuth 等前端组件调用。
//! 注意：generic 的 auth_start_login / auth_poll_for_account 等命令保留在
//! `commands::auth` 中以兼容旧的 managed-auth 调用路径。

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::proxy::providers::oauth::{
    browser_flow::BrowserFlowResponse, OAuthAccountInfo, OAuthAuthStatus, OAuthDeviceCodeResponse,
    OAuthManager, OAuthProviderId,
};

/// OAuthManager 全局状态
pub struct OAuthManagerState(pub Arc<RwLock<OAuthManager>>);

/// 启动 OAuth 浏览器流程（自动打开浏览器 / Headless 模式）
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_start_browser_flow(
    auth_provider: String,
    auto_open_browser: Option<bool>,
    state: State<'_, OAuthManagerState>,
) -> Result<BrowserFlowResponse, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    let mut response = manager
        .start_browser_flow(provider_id)
        .await
        .map_err(|e| e.to_string())?;
    response.auto_open_browser = auto_open_browser.unwrap_or(true);
    Ok(response)
}

/// 完成 OAuth 浏览器流程（等待浏览器回调）
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_complete_browser_flow(
    auth_provider: String,
    state: State<'_, OAuthManagerState>,
) -> Result<Option<OAuthAccountInfo>, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager
        .complete_browser_flow(provider_id)
        .await
        .map_err(|e| e.to_string())
}

/// 完成 OAuth 浏览器流程（Headless 模式 - 用户粘贴回调 URL）
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_complete_with_callback_url(
    auth_provider: String,
    callback_url: String,
    state: State<'_, OAuthManagerState>,
) -> Result<Option<OAuthAccountInfo>, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager
        .complete_browser_flow_with_url(provider_id, &callback_url)
        .await
        .map_err(|e| e.to_string())
}

/// 取消 OAuth 浏览器流程
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_cancel_browser_flow(
    auth_provider: String,
    state: State<'_, OAuthManagerState>,
) -> Result<(), String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager.cancel_browser_flow(provider_id).await;
    Ok(())
}

/// 列出所有支持的 OAuth 提供商
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_list_providers() -> Result<Vec<OAuthProviderInfo>, String> {
    Ok(vec![
        OAuthProviderInfo {
            id: "github_copilot".to_string(),
            name: "GitHub Copilot".to_string(),
            supports_device_code: true,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            supports_device_code: false,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic (Claude)".to_string(),
            supports_device_code: false,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "google_gemini".to_string(),
            name: "Google Gemini".to_string(),
            supports_device_code: true,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "alibaba_qwen".to_string(),
            name: "阿里巴巴通义千问".to_string(),
            supports_device_code: true,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "moonshot_kimi".to_string(),
            name: "Moonshot AI (Kimi)".to_string(),
            supports_device_code: true,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "minimax".to_string(),
            name: "MiniMax".to_string(),
            supports_device_code: true,
            requires_token_exchange: true,
        },
        OAuthProviderInfo {
            id: "volcengine_ark".to_string(),
            name: "火山引擎 Ark".to_string(),
            supports_device_code: true,
            requires_token_exchange: true,
        },
    ])
}

/// 保存 OAuth Client ID
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_save_client_id(
    provider_id: String,
    client_id: String,
    _state: State<'_, OAuthManagerState>,
) -> Result<(), String> {
    crate::settings::set_oauth_client_id(&provider_id, client_id).map_err(|e| e.to_string())
}

/// 移除 OAuth Client ID
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_remove_client_id(
    provider_id: String,
    _state: State<'_, OAuthManagerState>,
) -> Result<(), String> {
    crate::settings::remove_oauth_client_id(&provider_id).map_err(|e| e.to_string())
}

/// 列出所有已配置的 Client ID
#[tauri::command(rename_all = "camelCase")]
pub async fn auth_list_client_ids(
    _state: State<'_, OAuthManagerState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    Ok(crate::settings::list_oauth_client_ids())
}

/// OAuth 提供商信息（前端用）
#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthProviderInfo {
    pub id: String,
    pub name: String,
    pub supports_device_code: bool,
    pub requires_token_exchange: bool,
}

/// 获取 OAuth 认证状态（OAuthManager 实现，覆盖 main 的 ManagedAuthStatus）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_get_status(
    auth_provider: String,
    state: State<'_, OAuthManagerState>,
) -> Result<OAuthAuthStatus, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    Ok(manager.get_auth_status(provider_id).await)
}

/// 轮询检查账号是否授权完成（OAuthManager 实现）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_poll_for_account(
    auth_provider: String,
    device_code: String,
    state: State<'_, OAuthManagerState>,
) -> Result<Option<OAuthAccountInfo>, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager
        .poll_for_account(provider_id, &device_code)
        .await
        .map_err(|e| e.to_string())
}

/// 启动 OAuth 登录（OAuthManager 实现）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_start_login(
    auth_provider: String,
    state: State<'_, OAuthManagerState>,
) -> Result<OAuthDeviceCodeResponse, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager
        .start_login(provider_id)
        .await
        .map_err(|e| e.to_string())
}

/// 列出已认证账号（OAuthManager 实现）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_list_accounts(
    auth_provider: String,
    state: State<'_, OAuthManagerState>,
) -> Result<Vec<OAuthAccountInfo>, String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    Ok(manager.list_accounts(provider_id).await)
}

/// 移除账号（OAuthManager 实现）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_remove_account(
    auth_provider: String,
    account_id: String,
    state: State<'_, OAuthManagerState>,
) -> Result<(), String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager
        .remove_account(provider_id, &account_id)
        .await
}

/// 设置默认账号（OAuthManager 实现）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_set_default_account(
    auth_provider: String,
    account_id: String,
    state: State<'_, OAuthManagerState>,
) -> Result<(), String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager
        .set_default_account(provider_id, &account_id)
        .await
}

/// 登出（OAuthManager 实现）
#[tauri::command(rename_all = "camelCase")]
pub async fn oauth_auth_logout(
    auth_provider: String,
    state: State<'_, OAuthManagerState>,
) -> Result<(), String> {
    let provider_id: OAuthProviderId = auth_provider
        .parse()
        .map_err(|e: String| e)?;
    let manager = state.0.read().await;
    manager.clear_auth(provider_id).await
}
