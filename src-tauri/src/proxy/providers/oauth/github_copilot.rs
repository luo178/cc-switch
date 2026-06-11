//! GitHub Copilot OAuth 配置
//!
//! 定义 GitHub Copilot 的 OAuth 配置信息

/// GitHub OAuth 客户端 ID（VS Code 使用的 ID）
pub const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

/// GitHub 设备码 URL
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";

/// GitHub OAuth Token URL
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Copilot Token URL
#[allow(dead_code)]
pub const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";

/// GitHub User API URL
pub const GITHUB_USER_URL: &str = "https://api.github.com/user";

/// Copilot 使用量 API URL
#[allow(dead_code)]
pub const COPILOT_USAGE_URL: &str = "https://api.github.com/copilot_internal/user";

/// 默认 Copilot API 端点
#[allow(dead_code)]
pub const DEFAULT_COPILOT_API_ENDPOINT: &str = "https://api.githubcopilot.com";

/// Copilot API Header 常量
#[allow(dead_code)]
pub const COPILOT_EDITOR_VERSION: &str = "vscode/1.110.1";
#[allow(dead_code)]
pub const COPILOT_PLUGIN_VERSION: &str = "copilot-chat/0.38.2";
#[allow(dead_code)]
pub const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.38.2";
#[allow(dead_code)]
pub const COPILOT_API_VERSION: &str = "2025-10-01";
