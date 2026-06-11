//! 阿里巴巴通义千问 OAuth 配置
//!
//! 定义阿里云 DashScope (通义千问) 的 OAuth 配置信息

/// 阿里云 OAuth 客户端 ID 占位符
/// 注意：需要从阿里云 RAM 控制台获取
pub const ALIBABA_CLIENT_ID: &str = "YOUR_ALIBABA_CLIENT_ID";

/// 阿里云 OAuth 设备码 URL
pub const ALIBABA_DEVICE_CODE_URL: &str = "https://oauth.aliyun.com/device/code";

/// 阿里云 OAuth Token URL
pub const ALIBABA_TOKEN_URL: &str = "https://oauth.aliyun.com/token";

/// 阿里云 User Info URL
pub const ALIBABA_USER_INFO_URL: &str = "https://api.aliyun.com/oauth/user_info";

/// 阿里云 DashScope API 基础 URL
#[allow(dead_code)]
pub const DASHSCOPE_API_BASE_URL: &str = "https://dashscope.aliyuncs.com";
