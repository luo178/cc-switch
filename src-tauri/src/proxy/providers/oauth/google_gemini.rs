//! Google Gemini OAuth 配置
//!
//! 定义 Google Gemini 的 OAuth 配置信息

/// Google OAuth 客户端 ID 占位符
/// 注意：需要从 Google Cloud Console 获取
pub const GOOGLE_CLIENT_ID: &str = "YOUR_GOOGLE_CLIENT_ID";

/// Google OAuth 设备码 URL
pub const GOOGLE_DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";

/// Google OAuth Token URL
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Google User Info URL
pub const GOOGLE_USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// Google Gemini API 基础 URL
#[allow(dead_code)]
pub const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com";
