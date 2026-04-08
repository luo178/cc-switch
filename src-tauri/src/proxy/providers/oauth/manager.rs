//! OAuth 管理器
//!
//! 统一的 OAuth 认证管理器，协调所有 OAuth 提供商的认证操作

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::proxy::providers::copilot_auth::CopilotAuthManager;
use crate::proxy::providers::oauth::provider::{DeviceCodeResponse, OAuthError, OAuthUserInfo, TokenResponse};
use crate::proxy::providers::oauth::provider_id::OAuthProviderId;
use crate::proxy::providers::oauth::storage::{OAuthAccount, OAuthStorage};
use crate::proxy::providers::oauth::alibaba_qwen::{
    ALIBABA_CLIENT_ID as ALIBABA_QWEN_CLIENT_ID,
    ALIBABA_DEVICE_CODE_URL as ALIBABA_QWEN_DEVICE_CODE_URL,
    ALIBABA_TOKEN_URL as ALIBABA_QWEN_TOKEN_URL,
    ALIBABA_USER_INFO_URL as ALIBABA_QWEN_USER_INFO_URL,
};
use crate::proxy::providers::oauth::github_copilot::{
    GITHUB_CLIENT_ID,
    GITHUB_DEVICE_CODE_URL,
    GITHUB_TOKEN_URL,
    GITHUB_USER_URL,
};
use crate::proxy::providers::oauth::google_gemini::{
    GOOGLE_CLIENT_ID,
    GOOGLE_DEVICE_CODE_URL,
    GOOGLE_TOKEN_URL,
    GOOGLE_USER_INFO_URL,
};
use crate::proxy::providers::oauth::minimax::{
    MINIMAX_CLIENT_ID,
    MINIMAX_DEVICE_CODE_URL,
    MINIMAX_TOKEN_URL,
    MINIMAX_USER_INFO_URL,
};
use crate::proxy::providers::oauth::moonshot::{
    MOONSHOT_CLIENT_ID,
    MOONSHOT_DEVICE_CODE_URL,
    MOONSHOT_TOKEN_URL,
    MOONSHOT_USER_INFO_URL,
};
use crate::proxy::providers::oauth::openai::{
    OPENAI_CLIENT_ID,
    OPENAI_DEVICE_CODE_URL,
    OPENAI_TOKEN_URL,
    OPENAI_USER_INFO_URL,
};
use crate::proxy::providers::oauth::volcengine::{
    VOLCENGINE_CLIENT_ID,
    VOLCENGINE_DEVICE_CODE_URL,
    VOLCENGINE_TOKEN_URL,
    VOLCENGINE_USER_INFO_URL,
};

/// OAuth 账号公开信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthAccountInfo {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
}

impl From<&OAuthAccount> for OAuthAccountInfo {
    fn from(account: &OAuthAccount) -> Self {
        Self {
            id: account.id.clone(),
            provider: "".to_string(),
            login: account.login.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            authenticated_at: account.authenticated_at,
            is_default: false,
        }
    }
}

impl OAuthAccountInfo {
    pub fn from_account(account: &OAuthAccount, provider_id: &OAuthProviderId) -> Self {
        Self {
            id: account.id.clone(),
            provider: provider_id.as_str().to_string(),
            login: account.login.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            authenticated_at: account.authenticated_at,
            is_default: false,
        }
    }
}

/// OAuth 设备码响应（带提供商信息）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

impl From<(OAuthProviderId, DeviceCodeResponse)> for OAuthDeviceCodeResponse {
    fn from((provider, response): (OAuthProviderId, DeviceCodeResponse)) -> Self {
        Self {
            provider: provider.as_str().to_string(),
            device_code: response.device_code,
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            expires_in: response.expires_in,
            interval: response.interval,
        }
    }
}

/// OAuth 认证状态
#[derive(Debug, Clone)]
pub struct OAuthAuthStatus {
    pub provider: OAuthProviderId,
    pub authenticated: bool,
    pub default_account_id: Option<String>,
    pub accounts: Vec<OAuthAccountInfo>,
}

impl OAuthAuthStatus {
    pub fn new(provider: OAuthProviderId) -> Self {
        Self {
            provider,
            authenticated: false,
            default_account_id: None,
            accounts: Vec::new(),
        }
    }
}

/// OAuth 管理器
///
/// 管理所有 OAuth 提供商的认证状态和操作
pub struct OAuthManager {
    /// 提供商存储（每个提供商一个存储实例）
    storages: Arc<RwLock<HashMap<OAuthProviderId, Arc<OAuthStorage>>>>,
    /// 数据目录
    data_dir: PathBuf,
    /// HTTP 客户端
    http_client: reqwest::Client,
    /// Copilot 专用认证管理器（复用已有实现）
    copilot_auth: Option<Arc<CopilotAuthManager>>,
}

impl OAuthManager {
    /// 创建新的 OAuth 管理器
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            storages: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
            http_client: reqwest::Client::new(),
            copilot_auth: None,
        }
    }

    /// 创建新的 OAuth 管理器（带 Copilot Auth）
    pub fn new_with_copilot(data_dir: PathBuf, copilot_auth: CopilotAuthManager) -> Self {
        Self {
            storages: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
            http_client: reqwest::Client::new(),
            copilot_auth: Some(Arc::new(copilot_auth)),
        }
    }

    /// 获取或创建存储
    pub async fn get_storage(&self, provider_id: OAuthProviderId) -> Arc<OAuthStorage> {
        // 快速路径：检查缓存
        {
            let storages = self.storages.read().await;
            if let Some(storage) = storages.get(&provider_id) {
                return Arc::clone(storage);
            }
        }

        // 创建新存储
        let storage = Arc::new(OAuthStorage::new(provider_id, self.data_dir.clone()));

        // 缓存
        {
            let mut storages = self.storages.write().await;
            storages.insert(provider_id, Arc::clone(&storage));
        }

        storage
    }

    /// 获取认证状态
    pub async fn get_auth_status(&self, provider_id: OAuthProviderId) -> OAuthAuthStatus {
        let storage = self.get_storage(provider_id).await;
        let accounts = storage.list_accounts().await;
        let default_account_id = storage.get_default_account_id().await;

        let provider = provider_id.as_str().to_string();
        let account_infos: Vec<OAuthAccountInfo> = accounts
            .iter()
            .map(|a| {
                let mut info = OAuthAccountInfo::from_account(a, &provider_id);
                info.provider = provider.clone();
                info.is_default = Some(&a.id) == default_account_id.as_ref();
                info
            })
            .collect();

        OAuthAuthStatus {
            provider: provider_id,
            authenticated: !accounts.is_empty(),
            default_account_id,
            accounts: account_infos,
        }
    }

    /// 移除账号
    pub async fn remove_account(&self, provider_id: OAuthProviderId, account_id: &str) -> Result<(), String> {
        let storage = self.get_storage(provider_id).await;
        storage.remove_account(account_id).await
    }

    /// 设置默认账号
    pub async fn set_default_account(&self, provider_id: OAuthProviderId, account_id: &str) -> Result<(), String> {
        let storage = self.get_storage(provider_id).await;
        storage.set_default_account(account_id).await
    }

    /// 清除所有认证
    pub async fn clear_auth(&self, provider_id: OAuthProviderId) -> Result<(), String> {
        let storage = self.get_storage(provider_id).await;
        storage.clear().await
    }

    /// 列出所有账号
    pub async fn list_accounts(&self, provider_id: OAuthProviderId) -> Vec<OAuthAccountInfo> {
        let storage = self.get_storage(provider_id).await;
        let accounts = storage.list_accounts().await;
        let default_account_id = storage.get_default_account_id().await;

        let provider = provider_id.as_str().to_string();
        accounts
            .into_iter()
            .map(|a| {
                let mut info = OAuthAccountInfo::from_account(&a, &provider_id);
                info.provider = provider.clone();
                info.is_default = Some(&a.id) == default_account_id.as_ref();
                info
            })
            .collect()
    }

    /// 检查是否已认证
    pub async fn is_authenticated(&self, provider_id: OAuthProviderId) -> bool {
        let storage = self.get_storage(provider_id).await;
        storage.is_authenticated().await
    }

    /// 获取提供商数据目录
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    // ==================== OAuth 设备码流程 ====================

    /// 获取提供商的 OAuth 配置
    fn get_provider_config(&self, provider_id: OAuthProviderId) -> ProviderOAuthConfig {
        match provider_id {
            OAuthProviderId::GitHubCopilot => ProviderOAuthConfig {
                client_id: GITHUB_CLIENT_ID.to_string(),
                device_code_url: GITHUB_DEVICE_CODE_URL.to_string(),
                token_url: GITHUB_TOKEN_URL.to_string(),
                user_info_url: Some(GITHUB_USER_URL.to_string()),
                scopes: vec!["read:user".to_string()],
            },
            OAuthProviderId::OpenAI => ProviderOAuthConfig {
                client_id: OPENAI_CLIENT_ID.to_string(),
                device_code_url: OPENAI_DEVICE_CODE_URL.to_string(),
                token_url: OPENAI_TOKEN_URL.to_string(),
                user_info_url: Some(OPENAI_USER_INFO_URL.to_string()),
                scopes: vec!["openid".to_string(), "model.read".to_string(), "offline".to_string()],
            },
            OAuthProviderId::GoogleGemini => ProviderOAuthConfig {
                client_id: GOOGLE_CLIENT_ID.to_string(),
                device_code_url: GOOGLE_DEVICE_CODE_URL.to_string(),
                token_url: GOOGLE_TOKEN_URL.to_string(),
                user_info_url: Some(GOOGLE_USER_INFO_URL.to_string()),
                scopes: vec![
                    "openid".to_string(),
                    "email".to_string(),
                    "profile".to_string(),
                    "https://www.googleapis.com/auth/generative-language.retriever".to_string(),
                ],
            },
            OAuthProviderId::AlibabaQwen => ProviderOAuthConfig {
                client_id: ALIBABA_QWEN_CLIENT_ID.to_string(),
                device_code_url: ALIBABA_QWEN_DEVICE_CODE_URL.to_string(),
                token_url: ALIBABA_QWEN_TOKEN_URL.to_string(),
                user_info_url: Some(ALIBABA_QWEN_USER_INFO_URL.to_string()),
                scopes: vec!["openapi".to_string()],
            },
            OAuthProviderId::MoonshotKimi => ProviderOAuthConfig {
                client_id: MOONSHOT_CLIENT_ID.to_string(),
                device_code_url: MOONSHOT_DEVICE_CODE_URL.to_string(),
                token_url: MOONSHOT_TOKEN_URL.to_string(),
                user_info_url: Some(MOONSHOT_USER_INFO_URL.to_string()),
                scopes: vec!["user.info".to_string(), "chatplt.compact".to_string()],
            },
            OAuthProviderId::MiniMax => ProviderOAuthConfig {
                client_id: MINIMAX_CLIENT_ID.to_string(),
                device_code_url: MINIMAX_DEVICE_CODE_URL.to_string(),
                token_url: MINIMAX_TOKEN_URL.to_string(),
                user_info_url: Some(MINIMAX_USER_INFO_URL.to_string()),
                scopes: vec!["user.base".to_string(), "chat.default".to_string()],
            },
            OAuthProviderId::VolcEngineArk => ProviderOAuthConfig {
                client_id: VOLCENGINE_CLIENT_ID.to_string(),
                device_code_url: VOLCENGINE_DEVICE_CODE_URL.to_string(),
                token_url: VOLCENGINE_TOKEN_URL.to_string(),
                user_info_url: Some(VOLCENGINE_USER_INFO_URL.to_string()),
                scopes: vec!["ark".to_string()],
            },
        }
    }

    /// 启动 OAuth 设备码流程
    pub async fn start_login(&self, provider_id: OAuthProviderId) -> Result<OAuthDeviceCodeResponse, OAuthError> {
        // GitHub Copilot 使用专用实现
        if provider_id == OAuthProviderId::GitHubCopilot {
            if let Some(ref copilot_auth) = self.copilot_auth {
                let _copilot_manager = copilot_auth.clone();
                let resp = copilot_auth.start_device_flow().await.map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;
                return Ok(OAuthDeviceCodeResponse {
                    provider: provider_id.as_str().to_string(),
                    device_code: resp.device_code,
                    user_code: resp.user_code,
                    verification_uri: resp.verification_uri,
                    expires_in: resp.expires_in,
                    interval: resp.interval,
                });
            }
            return Err(OAuthError::UnsupportedProvider("GitHub Copilot auth not initialized".to_string()));
        }

        let config = self.get_provider_config(provider_id);
        if config.client_id == "YOUR_CLIENT_ID" || config.client_id.is_empty() {
            return Err(OAuthError::UnsupportedProvider(format!(
                "OAuth not configured for {}: missing client ID", provider_id.display_name()
            )));
        }

        let scopes = config.scopes.join(" ");
        let response = self.http_client
            .post(&config.device_code_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("scope", scopes.as_str()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthError::NetworkError(format!(
                "Device code request failed: {status} - {text}"
            )));
        }

        let device_code: DeviceCodeResponse = response.json().await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        log::info!("[OAuthManager] Started device flow for {}: user_code={}",
            provider_id.display_name(), device_code.user_code);

        Ok(OAuthDeviceCodeResponse::from((provider_id, device_code)))
    }

    /// 轮询 OAuth Token（返回新添加的账号，如果成功）
    pub async fn poll_for_account(
        &self,
        provider_id: OAuthProviderId,
        device_code: &str,
    ) -> Result<Option<OAuthAccountInfo>, OAuthError> {
        // GitHub Copilot 使用专用实现
        if provider_id == OAuthProviderId::GitHubCopilot {
            if let Some(ref copilot_auth) = self.copilot_auth {
                let account = copilot_auth.poll_for_token(device_code).await
                    .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;
                if let Some(gh_account) = account {
                    let info = OAuthAccountInfo {
                        id: gh_account.id.clone(),
                        provider: provider_id.as_str().to_string(),
                        login: gh_account.login.clone(),
                        email: gh_account.avatar_url.clone(), // GitHub API doesn't return email in user endpoint
                        avatar_url: gh_account.avatar_url.clone(),
                        authenticated_at: gh_account.authenticated_at,
                        is_default: true,
                    };
                    return Ok(Some(info));
                }
                return Ok(None);
            }
            return Err(OAuthError::UnsupportedProvider("GitHub Copilot auth not initialized".to_string()));
        }

        let config = self.get_provider_config(provider_id);

        let response = self.http_client
            .post(&config.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let status = response.status();

        // 先检查状态码判断是否成功
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            // OAuth 错误可能在任何状态码中
            if text.contains("\"error\"") {
                #[derive(serde::Deserialize)]
                struct TokenErrorResponse {
                    error: String,
                    #[serde(default)]
                    error_description: Option<String>,
                }
                if let Ok(error_resp) = serde_json::from_str::<TokenErrorResponse>(&text) {
                    return match error_resp.error.as_str() {
                        "authorization_pending" => Err(OAuthError::AuthorizationPending),
                        "slow_down" => Err(OAuthError::AuthorizationPending),
                        "expired_token" => Err(OAuthError::ExpiredToken),
                        "access_denied" => Err(OAuthError::AccessDenied),
                        _ => Err(OAuthError::TokenExchangeFailed(format!(
                            "{}: {}", error_resp.error, error_resp.error_description.unwrap_or_default()
                        ))),
                    };
                }
            }
            return Err(OAuthError::NetworkError(format!(
                "Token request failed: {}", status
            )));
        }

        let token_response: TokenResponse = response.json().await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        // 获取用户信息
        let user_info = if let Some(ref user_info_url) = config.user_info_url {
            let user_resp = self.http_client
                .get(user_info_url)
                .header("Authorization", format!("Bearer {}", token_response.access_token))
                .send()
                .await?;

            if user_resp.status().is_success() {
                Some(user_resp.json::<OAuthUserInfo>().await
                    .map_err(|e| OAuthError::ParseError(e.to_string()))?)
            } else {
                None
            }
        } else {
            None
        };

        // 构建用户信息
        let user_info = user_info.unwrap_or_else(|| OAuthUserInfo {
            id: format!("{}_{}", provider_id.as_str(), &token_response.access_token[..8.min(token_response.access_token.len())]),
            login: format!("{} User", provider_id.display_name()),
            email: None,
            avatar_url: None,
            raw: serde_json::json!({}),
        });

        let expires_at = token_response.expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

        // 创建账号
        let account = OAuthAccount::from_user_info(&user_info, token_response.access_token.clone(), expires_at);
        let account_info = OAuthAccountInfo::from_account(&account, &provider_id);

        // 保存到存储
        let storage = self.get_storage(provider_id).await;
        storage.add_account(account).await;

        log::info!("[OAuthManager] Account added for {}: {}", provider_id.display_name(), user_info.login);

        Ok(Some(account_info))
    }
}

/// 提供商 OAuth 配置
struct ProviderOAuthConfig {
    client_id: String,
    device_code_url: String,
    token_url: String,
    user_info_url: Option<String>,
    scopes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_oauth_device_code_response_from() {
        let device_code = DeviceCodeResponse {
            device_code: "device_code_123".to_string(),
            user_code: "USER-CODE".to_string(),
            verification_uri: "https://example.com/verify".to_string(),
            expires_in: 300,
            interval: 5,
        };

        let response = OAuthDeviceCodeResponse::from((OAuthProviderId::OpenAI, device_code));

        assert_eq!(response.provider, "openai");
        assert_eq!(response.device_code, "device_code_123");
        assert_eq!(response.user_code, "USER-CODE");
        assert_eq!(response.verification_uri, "https://example.com/verify");
        assert_eq!(response.expires_in, 300);
        assert_eq!(response.interval, 5);
    }

    #[test]
    fn test_oauth_auth_status_new() {
        let status = OAuthAuthStatus::new(OAuthProviderId::GitHubCopilot);

        assert_eq!(status.provider, OAuthProviderId::GitHubCopilot);
        assert!(!status.authenticated);
        assert!(status.default_account_id.is_none());
        assert!(status.accounts.is_empty());
    }

    #[test]
    fn test_oauth_account_info_from_account() {
        let account = OAuthAccount {
            id: "12345".to_string(),
            login: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: None,
            authenticated_at: 1700000000,
        };

        let info = OAuthAccountInfo::from_account(&account, &OAuthProviderId::OpenAI);

        assert_eq!(info.id, "12345");
        assert_eq!(info.provider, "openai");
        assert_eq!(info.login, "testuser");
        assert_eq!(info.email, Some("test@example.com".to_string()));
        assert_eq!(info.avatar_url, Some("https://example.com/avatar.png".to_string()));
        assert_eq!(info.authenticated_at, 1700000000);
        assert!(!info.is_default);
    }

    #[test]
    fn test_oauth_manager_new() {
        let temp_dir = tempdir().unwrap();
        let manager = OAuthManager::new(temp_dir.path().to_path_buf());

        assert_eq!(manager.data_dir(), temp_dir.path());
    }
}
