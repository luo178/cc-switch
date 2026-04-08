//! Moonshot AI (Kimi) OAuth 配置
//!
//! 定义 Moonshot AI (Kimi) 的 OAuth 配置信息

/// Moonshot OAuth 客户端 ID 占位符
/// 注意：需要从 Moonshot 开发者平台获取
pub const MOONSHOT_CLIENT_ID: &str = "YOUR_MOONSHOT_CLIENT_ID";

/// Moonshot OAuth 设备码 URL
pub const MOONSHOT_DEVICE_CODE_URL: &str = "https://platform.moonshot.cn/oauth/device/code";

/// Moonshot OAuth Token URL
pub const MOONSHOT_TOKEN_URL: &str = "https://platform.moonshot.cn/oauth/token";

/// Moonshot User Info URL
pub const MOONSHOT_USER_INFO_URL: &str = "https://platform.moonshot.cn/oauth/user/info";

/// Moonshot API 基础 URL
#[allow(dead_code)]
pub const MOONSHOT_API_BASE_URL: &str = "https://api.moonshot.cn";
