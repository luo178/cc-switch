//! VolcEngine Ark OAuth 配置
//!
//! 定义字节火山引擎 Ark 的 OAuth 配置信息

/// VolcEngine OAuth 客户端 ID 占位符
/// 注意：需要从火山引擎控制台获取
pub const VOLCENGINE_CLIENT_ID: &str = "YOUR_VOLCENGINE_CLIENT_ID";

/// VolcEngine OAuth 设备码 URL
pub const VOLCENGINE_DEVICE_CODE_URL: &str = "https://ark.cn-beijing.volces.com/oauth/device/code";

/// VolcEngine OAuth Token URL
pub const VOLCENGINE_TOKEN_URL: &str = "https://ark.cn-beijing.volces.com/oauth/token";

/// VolcEngine User Info URL
pub const VOLCENGINE_USER_INFO_URL: &str = "https://ark.cn-beijing.volces.com/oauth/user/info";

/// VolcEngine Ark API 基础 URL
#[allow(dead_code)]
pub const VOLCENGINE_API_BASE_URL: &str = "https://ark.cn-beijing.volces.com";
