//! OAuth 浏览器流程模块
//!
//! 实现基于浏览器的 Authorization Code + PKCE OAuth 流程
//! 不需要用户手动输入设备码，而是自动打开浏览器进行授权

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::proxy::providers::oauth::provider::{OAuthError, OAuthUserInfo, TokenResponse};
use crate::proxy::providers::oauth::provider_id::OAuthProviderId;

use super::storage::OAuthAccount;

/// OAuth 授权 URL 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFlowResponse {
    /// 提供商
    pub provider: String,
    /// 授权 URL（用于手动复制）
    pub authorization_url: String,
    /// 本地回调地址
    pub redirect_uri: String,
    /// 状态参数（用于防 CSRF）
    pub state: String,
    /// PKCE code_verifier
    pub code_verifier: String,
    /// 是否自动打开浏览器
    #[serde(default)]
    pub auto_open_browser: bool,
}

/// 用于存储的 PKCE 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkceInfo {
    /// code_verifier（用于后续交换 token）
    pub code_verifier: String,
    /// code_challenge（B64URL(SHA256(code_verifier))）
    pub code_challenge: String,
    /// state 参数
    pub state: String,
    /// redirect_uri
    pub redirect_uri: String,
}

/// OAuth 浏览器流程管理器
pub struct OAuthBrowserFlow {
    /// 数据目录
    data_dir: PathBuf,
    /// HTTP 客户端
    http_client: reqwest::Client,
    /// 活跃的 PKCE 流程（provider -> PKCE info）
    active_flows: Arc<RwLock<HashMap<OAuthProviderId, PkceInfo>>>,
}

/// OAuth 提供商配置
struct ProviderBrowserConfig {
    /// Client ID
    client_id: String,
    /// 授权 URL
    authorize_url: String,
    /// Token 交换 URL
    token_url: String,
    /// User Info URL
    user_info_url: Option<String>,
    /// scopes
    scopes: Vec<String>,
    /// 授权 URL 端点参数映射
    extra_params: HashMap<String, String>,
    /// 是否需要用户配置 Client ID
    needs_config: bool,
}

impl OAuthBrowserFlow {
    /// 创建新的浏览器流程管理器
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            active_flows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 生成 PKCE code_verifier 和 code_challenge
    fn generate_pkce() -> (String, String) {
        let mut rng = rand::thread_rng();
        // code_verifier: 43-128 个字符随机字符串（使用 URL 安全字符）
        let code_verifier: String = (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..64);
                let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
                chars[idx] as char
            })
            .collect();

        // code_challenge: BASE64URL(SHA256(code_verifier))
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(hash);

        (code_verifier, code_challenge)
    }

    /// 生成 state 参数
    fn generate_state() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        URL_SAFE_NO_PAD.encode(&bytes)
    }

    /// 获取提供商的浏览器流程配置
    fn get_provider_config(&self, provider_id: OAuthProviderId) -> ProviderBrowserConfig {
        // 优先级：环境变量 > settings > 默认 Client ID
        let get_client_id = |default: &str| -> String {
            // 先检查环境变量
            let env_key = format!(
                "CCSWITCH_{}_CLIENT_ID",
                provider_id.as_str().to_uppercase().replace("-", "_")
            );
            if let Ok(val) = std::env::var(&env_key) {
                if !val.is_empty() && !val.starts_with("YOUR_") {
                    return val;
                }
            }
            // 再检查 settings
            if let Some(val) = crate::settings::get_oauth_client_id(provider_id.as_str()) {
                if !val.is_empty() && !val.starts_with("YOUR_") {
                    return val;
                }
            }
            // 用默认/硬编码
            default.to_string()
        };

        match provider_id {
            OAuthProviderId::GitHubCopilot => {
                let client_id = get_client_id("Iv1.b507a08c87ecfe98");
                let needs = client_id.is_empty() || client_id.starts_with("YOUR_");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://github.com/login/oauth/authorize".to_string(),
                    token_url: "https://github.com/login/oauth/access_token".to_string(),
                    user_info_url: Some("https://api.github.com/user".to_string()),
                    scopes: vec!["read:user".to_string(), "read:org".to_string()],
                    extra_params: {
                        let mut m = HashMap::new();
                        m.insert("allow_signup".to_string(), "true".to_string());
                        m
                    },
                    needs_config: needs,
                }
            },
            OAuthProviderId::OpenAI => {
                let client_id = get_client_id("app_EMoamEEZ73f0CkXaXp7hrann");
                let needs = client_id.is_empty() || client_id.starts_with("YOUR_");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://auth.openai.com/oauth/authorize".to_string(),
                    token_url: "https://auth.openai.com/oauth/token".to_string(),
                    user_info_url: Some("https://api.openai.com/v1/user".to_string()),
                    scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string(), "offline_access".to_string()],
                    extra_params: HashMap::new(), // response_type 已在基础参数中添加
                    needs_config: needs,
                }
            },
            // Anthropic (Claude) - 需要配置 Client ID（API Key 认证更常用）
            // 注意：Anthropic 主要使用 API Key 认证，OAuth 支持有限
            OAuthProviderId::Anthropic => {
                let client_id = get_client_id("YOUR_ANTHROPIC_CLIENT_ID");
                let needs = client_id.is_empty() || client_id.starts_with("YOUR_");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://auth.anthropic.com/oauth/authorize".to_string(),
                    token_url: "https://auth.anthropic.com/oauth/token".to_string(),
                    user_info_url: Some("https://api.anthropic.com/v1/user".to_string()),
                    scopes: vec!["api:full".to_string()],
                    extra_params: HashMap::new(),
                    needs_config: needs,
                }
            },
            OAuthProviderId::GoogleGemini => {
                let client_id = get_client_id("1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com");
                let needs = client_id.is_empty() || client_id.starts_with("YOUR_");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                    token_url: "https://oauth2.googleapis.com/token".to_string(),
                    user_info_url: Some("https://www.googleapis.com/oauth2/v3/userinfo".to_string()),
                    scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
                    extra_params: {
                        let mut m = HashMap::new();
                        m.insert("access_type".to_string(), "offline".to_string());
                        m.insert("prompt".to_string(), "consent".to_string());
                        m
                    },
                    needs_config: needs,
                }
            },
            // 中国厂商 - 不提供公开 OAuth，需要用户配置自己的 Client ID
            OAuthProviderId::AlibabaQwen => {
                let client_id = get_client_id("");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://oauth.aliyun.com/authorize".to_string(),
                    token_url: "https://oauth.aliyun.com/oauth/token".to_string(),
                    user_info_url: Some("https://api.aliyun.com/oauth/user_info".to_string()),
                    scopes: vec!["openapi".to_string()],
                    extra_params: HashMap::new(),
                    needs_config: true,
                }
            },
            OAuthProviderId::MoonshotKimi => {
                let client_id = get_client_id("");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://platform.moonshot.cn/oauth/authorize".to_string(),
                    token_url: "https://platform.moonshot.cn/oauth/token".to_string(),
                    user_info_url: Some("https://platform.moonshot.cn/oauth/user/info".to_string()),
                    scopes: vec!["user.info".to_string(), "chatplt.compact".to_string()],
                    extra_params: HashMap::new(),
                    needs_config: true,
                }
            },
            OAuthProviderId::MiniMax => {
                let client_id = get_client_id("");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://api.minimax.chat/oauth/authorize".to_string(),
                    token_url: "https://api.minimax.chat/oauth/token".to_string(),
                    user_info_url: Some("https://api.minimax.chat/v1/user/info".to_string()),
                    scopes: vec!["user.base".to_string(), "chat.default".to_string()],
                    extra_params: HashMap::new(),
                    needs_config: true,
                }
            },
            OAuthProviderId::VolcEngineArk => {
                let client_id = get_client_id("");
                ProviderBrowserConfig {
                    client_id,
                    authorize_url: "https://ark.cn-beijing.volces.com/oauth/authorize".to_string(),
                    token_url: "https://ark.cn-beijing.volces.com/oauth/token".to_string(),
                    user_info_url: Some("https://ark.cn-beijing.volces.com/oauth/user_info".to_string()),
                    scopes: vec!["ark".to_string()],
                    extra_params: HashMap::new(),
                    needs_config: true,
                }
            },
        }
    }

    /// 构建授权 URL
    fn build_authorization_url(
        config: &ProviderBrowserConfig,
        redirect_uri: &str,
        code_challenge: &str,
        state: &str,
    ) -> String {
        let mut url = url::Url::parse(&config.authorize_url).expect("Invalid authorize URL");

        // 基础参数
        url.query_pairs_mut()
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("scope", &config.scopes.join(" "));

        // 添加额外参数
        for (key, value) in &config.extra_params {
            url.query_pairs_mut().append_pair(key, value);
        }

        url.to_string()
    }

    /// 启动 OAuth 浏览器流程
    pub async fn start_browser_flow(
        &self,
        provider_id: OAuthProviderId,
    ) -> Result<BrowserFlowResponse, String> {
        // 生成 PKCE
        let (code_verifier, code_challenge) = Self::generate_pkce();
        let state = Self::generate_state();

        // 获取配置
        let config = self.get_provider_config(provider_id);

        // 检查 Client ID 是否有效（只有明确标记需要配置的才报错）
        if config.needs_config {
            return Err(format!(
                "{} 需要配置 OAuth Client ID。请在设置页面配置或设置环境变量 CCSWITCH_{}_CLIENT_ID。\n\n提示：这个提供商不提供公开 OAuth，需要从 {} 开发者平台申请 OAuth 应用后获取 Client ID。",
                provider_id.display_name(),
                provider_id.as_str().to_uppercase().replace("-", "_"),
                provider_id.display_name()
            ));
        }

        // 启动本地 TCP 监听器（端口 1455）
        let listener = self.start_local_listener().await.map_err(|e: String| e)?;

        let local_addr = listener.local_addr().map_err(|e| format!("Failed to get local address: {}", e))?;

        // redirect_uri 使用 localhost
        let redirect_uri = format!("http://localhost:{}/auth/callback", local_addr.port());

        log::info!(
            "[OAuthBrowserFlow] start_browser_flow for {}: client_id={}, needs_config={}, redirect_uri={}",
            provider_id.display_name(),
            if config.client_id.is_empty() { "empty" } else { &config.client_id[..8.min(config.client_id.len())] },
            config.needs_config,
            redirect_uri
        );

        // 构建授权 URL
        let authorization_url =
            Self::build_authorization_url(&config, &redirect_uri, &code_challenge, &state);

        // 保存 PKCE 信息
        let pkce_info = PkceInfo {
            code_verifier: code_verifier.clone(),
            code_challenge,
            state: state.clone(),
            redirect_uri: redirect_uri.clone(),
        };

        {
            let mut flows = self.active_flows.write().await;
            flows.insert(provider_id, pkce_info);
        }

        log::info!(
            "[OAuthBrowserFlow] Started browser flow for {}: {}",
            provider_id.display_name(),
            authorization_url
        );

        Ok(BrowserFlowResponse {
            provider: provider_id.as_str().to_string(),
            authorization_url,
            redirect_uri,
            state,
            code_verifier,
            auto_open_browser: true,
        })
    }

    /// 启动本地 TCP 监听器
    async fn start_local_listener(&self) -> Result<TcpListener, String> {
        // 尝试多个端口
        let ports: Vec<u16> = vec![1455, 1456, 1457, 1458, 1459, 1460];

        for port in ports {
            // 尝试绑定到 127.0.0.1 (IPv4)
            let addr: SocketAddr = format!("127.0.0.1:{}", port)
                .parse()
                .map_err(|e| format!("Invalid address: {}", e))?;

            match TcpListener::bind(addr).await {
                Ok(listener) => return Ok(listener),
                Err(e) => {
                    log::debug!("[OAuthBrowserFlow] Port {} unavailable: {}", port, e);
                    continue;
                }
            }
        }

        // 尝试随机端口
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind to any port: {}", e))?;

        Ok(listener)
    }

    /// 等待 OAuth 回调
    async fn wait_for_callback(&self, listener: TcpListener) -> Result<CallbackData, String> {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        let (mut reader, mut writer) = socket.split();

        let mut buf = vec![0u8; 1024];
        let n = reader
            .read(&mut buf)
            .await
            .map_err(|e| format!("Failed to read request: {}", e))?;

        let request = String::from_utf8_lossy(&buf[..n]);

        // 解析 URL (GET /auth/callback?code=xxx&state=xxx HTTP/1.1)
        let parts: Vec<&str> = request.split_whitespace().collect();
        let path = parts.get(1).unwrap_or(&"/");

        let url = url::Url::parse(path)
            .map_err(|e| format!("Invalid URL: {}", e))?;

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.to_string());

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.to_string());

        // 发送响应
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authentication Successful!</h1><p>You can close this window and return to the application.</p></body></html>";
        writer
            .write_all(response.as_bytes())
            .await
            .map_err(|e| format!("Failed to send response: {}", e))?;

        Ok(CallbackData {
            code: code.unwrap_or_default(),
            state: state.unwrap_or_default(),
        })
    }

    /// 完成 OAuth 流程（等待回调）
    pub async fn complete_browser_flow(
        &self,
        provider_id: OAuthProviderId,
    ) -> Result<Option<BrowserFlowAccountInfo>, OAuthError> {
        // 获取 PKCE 信息
        let pkce_info = {
            let flows = self.active_flows.read().await;
            flows.get(&provider_id).cloned()
        };

        let pkce_info = pkce_info.ok_or(OAuthError::DeviceFlowNotStarted)?;

        // 需要重新绑定监听器来接收回调
        let listener = self
            .start_local_listener()
            .await
            .map_err(|e| OAuthError::IoError(e))?;

        // 等待回调（带超时，5 分钟）
        let callback_result = timeout(Duration::from_secs(300), async {
            self.wait_for_callback(listener).await
        })
        .await
        .map_err(|_| OAuthError::IoError("Timeout waiting for OAuth callback".to_string()))?;

        let callback_data = callback_result.map_err(|e| OAuthError::IoError(e))?;

        // 验证 state
        if callback_data.state != pkce_info.state {
            return Err(OAuthError::TokenExchangeFailed(
                "State mismatch - possible CSRF attack".to_string(),
            ));
        }

        // 交换 token
        let token_response = self
            .exchange_code(
                provider_id,
                &callback_data.code,
                &pkce_info.redirect_uri,
                &pkce_info.code_verifier,
            )
            .await?;

        // 获取用户信息
        let user_info = match self.get_user_info(provider_id, &token_response.access_token).await {
            Ok(Some(info)) => info,
            Ok(None) | Err(_) => OAuthUserInfo {
                id: format!(
                    "{}_{}",
                    provider_id.as_str(),
                    &token_response.access_token[..8.min(token_response.access_token.len())]
                ),
                login: format!("{} User", provider_id.display_name()),
                email: None,
                avatar_url: None,
                raw: serde_json::json!({}),
            },
        };

        // 清理 PKCE 流程
        {
            let mut flows = self.active_flows.write().await;
            flows.remove(&provider_id);
        }

        let expires_at = token_response
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

        // 创建账号
        let account = OAuthAccount::from_user_info(
            &user_info,
            token_response.access_token.clone(),
            expires_at,
        );

        // 保存账号
        let storage = super::storage::OAuthStorage::new(provider_id, self.data_dir.clone());
        storage.add_account(account.clone()).await;

        log::info!(
            "[OAuthBrowserFlow] Account created for {}: {}",
            provider_id.display_name(),
            user_info.login
        );

        Ok(Some(BrowserFlowAccountInfo {
            id: account.id.clone(),
            provider: provider_id.as_str().to_string(),
            login: user_info.login,
            email: user_info.email,
            avatar_url: user_info.avatar_url,
            authenticated_at: account.authenticated_at,
            is_default: true,
        }))
    }

    /// 交换授权码为 Token
    async fn exchange_code(
        &self,
        provider_id: OAuthProviderId,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, OAuthError> {
        let config = self.get_provider_config(provider_id);

        let response = self
            .http_client
            .post(&config.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(text));
        }

        let token: TokenResponse = response.json().await.map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(token)
    }

    /// 获取用户信息
    async fn get_user_info(
        &self,
        provider_id: OAuthProviderId,
        access_token: &str,
    ) -> Result<Option<OAuthUserInfo>, OAuthError> {
        let config = self.get_provider_config(provider_id);

        let user_info_url = match &config.user_info_url {
            Some(url) => url,
            None => return Ok(None),
        };

        let response = self
            .http_client
            .get(user_info_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let user_info: OAuthUserInfo = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(Some(user_info))
    }

    /// 取消活跃的 OAuth 流程
    pub async fn cancel_flow(&self, provider_id: OAuthProviderId) {
        let mut flows = self.active_flows.write().await;
        flows.remove(&provider_id);
        log::info!("[OAuthBrowserFlow] Cancelled flow for {}", provider_id.display_name());
    }

    /// Headless 模式：从回调 URL 完成认证
    pub async fn complete_browser_flow_with_url(
        &self,
        provider_id: OAuthProviderId,
        callback_url: &str,
    ) -> Result<Option<BrowserFlowAccountInfo>, OAuthError> {
        // 获取 PKCE 信息
        let pkce_info = {
            let flows = self.active_flows.read().await;
            flows.get(&provider_id).cloned()
        };

        let pkce_info = pkce_info.ok_or(OAuthError::DeviceFlowNotStarted)?;

        // 解析回调 URL 获取 code 和 state
        let url = url::Url::parse(callback_url)
            .map_err(|e| OAuthError::ParseError(format!("Invalid callback URL: {}", e)))?;

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.to_string());

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.to_string());

        let code = code.ok_or(OAuthError::TokenExchangeFailed("No authorization code in callback URL".to_string()))?;
        let state = state.unwrap_or_default();

        // 验证 state
        if !state.is_empty() && state != pkce_info.state {
            return Err(OAuthError::TokenExchangeFailed("State mismatch - possible CSRF attack".to_string()));
        }

        // 交换 token
        let token_response = self
            .exchange_code(provider_id, &code, &pkce_info.redirect_uri, &pkce_info.code_verifier)
            .await?;

        // 获取用户信息
        let user_info = match self.get_user_info(provider_id, &token_response.access_token).await {
            Ok(Some(info)) => info,
            Ok(None) | Err(_) => OAuthUserInfo {
                id: format!("{}_{}", provider_id.as_str(), &token_response.access_token[..8.min(token_response.access_token.len())]),
                login: format!("{} User", provider_id.display_name()),
                email: None,
                avatar_url: None,
                raw: serde_json::json!({}),
            },
        };

        // 清理 PKCE 流程
        {
            let mut flows = self.active_flows.write().await;
            flows.remove(&provider_id);
        }

        let expires_at = token_response.expires_in.map(|secs| chrono::Utc::now().timestamp() + secs as i64);

        // 创建账号
        let account = OAuthAccount::from_user_info(&user_info, token_response.access_token.clone(), expires_at);

        // 保存账号
        let storage = super::storage::OAuthStorage::new(provider_id, self.data_dir.clone());
        storage.add_account(account.clone()).await;

        log::info!("[OAuthBrowserFlow] Account created via callback URL for {}", provider_id.display_name());

        Ok(Some(BrowserFlowAccountInfo {
            id: account.id.clone(),
            provider: provider_id.as_str().to_string(),
            login: user_info.login,
            email: user_info.email,
            avatar_url: user_info.avatar_url,
            authenticated_at: account.authenticated_at,
            is_default: true,
        }))
    }
}

/// OAuth 账号信息（浏览器流程返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFlowAccountInfo {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
}

/// 回调数据
struct CallbackData {
    code: String,
    state: String,
}