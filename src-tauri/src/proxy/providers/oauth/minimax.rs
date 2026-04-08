//! MiniMax OAuth 配置
//!
//! 定义 MiniMax 的 OAuth 配置信息

/// MiniMax OAuth 客户端 ID 占位符
/// 注意：需要从 MiniMax 开放平台获取
pub const MINIMAX_CLIENT_ID: &str = "YOUR_MINIMAX_CLIENT_ID";

/// MiniMax OAuth 设备码 URL
pub const MINIMAX_DEVICE_CODE_URL: &str = "https://api.minimax.chat/oauth/device/code";

/// MiniMax OAuth Token URL
pub const MINIMAX_TOKEN_URL: &str = "https://api.minimax.chat/oauth/token";

/// MiniMax User Info URL
pub const MINIMAX_USER_INFO_URL: &str = "https://api.minimax.chat/v1/user/info";

/// MiniMax API 基础 URL
#[allow(dead_code)]
pub const MINIMAX_API_BASE_URL: &str = "https://api.minimax.chat";
