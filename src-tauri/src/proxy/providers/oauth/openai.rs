//! OpenAI OAuth 配置
//!
//! 定义 OpenAI 的 OAuth 配置信息

/// OpenAI OAuth 客户端 ID 占位符
/// 注意：实际使用需要从 OpenAI 开发者平台获取
pub const OPENAI_CLIENT_ID: &str = "YOUR_OPENAI_CLIENT_ID";

/// OpenAI 设备码 URL
pub const OPENAI_DEVICE_CODE_URL: &str = "https://openai.com/oauth/device/code";

/// OpenAI OAuth Token URL
pub const OPENAI_TOKEN_URL: &str = "https://openai.com/oauth/token";

/// OpenAI User Info URL
pub const OPENAI_USER_INFO_URL: &str = "https://api.openai.com/v1/user";

/// OpenAI API 基础 URL
#[allow(dead_code)]
pub const OPENAI_API_BASE_URL: &str = "https://api.openai.com";
