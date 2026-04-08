//! OAuth Provider Types
//!
//! 定义 OAuth 相关的类型

use serde::{Deserialize, Serialize};

/// OAuth Token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// 访问令牌
    pub access_token: String,
    /// 令牌类型
    #[serde(default)]
    pub token_type: String,
    /// 过期时间（秒）
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// 刷新令牌
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// 权限范围
    #[serde(default)]
    pub scope: Option<String>,
    /// ID Token（部分 OAuth 提供商使用 JWT）
    #[serde(default)]
    pub id_token: Option<String>,
}

/// OAuth 设备码响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    /// 设备码（用于轮询）
    pub device_code: String,
    /// 用户码（显示给用户）
    pub user_code: String,
    /// 验证 URL
    pub verification_uri: String,
    /// 过期时间（秒）
    pub expires_in: u64,
    /// 轮询间隔（秒）
    pub interval: u64,
}

/// OAuth 用户信息
///
/// 通用的用户信息结构，不同提供商可能有额外字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    /// 用户唯一标识
    pub id: String,
    /// 用户名或昵称
    pub login: String,
    /// 邮箱（可选）
    #[serde(default)]
    pub email: Option<String>,
    /// 头像 URL（可选）
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// 提供商特定数据
    #[serde(default)]
    pub raw: serde_json::Value,
}

/// OAuth 错误类型
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("设备码流程未启动")]
    DeviceFlowNotStarted,

    #[error("等待用户授权中")]
    AuthorizationPending,

    #[error("用户拒绝授权")]
    AccessDenied,

    #[error("设备码已过期")]
    ExpiredToken,

    #[error("Token 无效或已过期")]
    TokenInvalid,

    #[error("Token 交换失败: {0}")]
    TokenExchangeFailed(String),

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("IO 错误: {0}")]
    IoError(String),

    #[error("账号不存在: {0}")]
    AccountNotFound(String),

    #[error("用户未订阅服务")]
    NoSubscription,

    #[error("不支持的提供商: {0}")]
    UnsupportedProvider(String),
}

impl From<reqwest::Error> for OAuthError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() || err.is_timeout() {
            OAuthError::NetworkError(err.to_string())
        } else {
            OAuthError::TokenExchangeFailed(err.to_string())
        }
    }
}

impl From<std::io::Error> for OAuthError {
    fn from(err: std::io::Error) -> Self {
        OAuthError::IoError(err.to_string())
    }
}
