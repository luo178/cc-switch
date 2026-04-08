//! OAuth 提供商 ID
//!
//! 定义所有支持的 OAuth 提供商唯一标识

use serde::{Deserialize, Serialize};

/// OAuth 提供商 ID
///
/// 每个 OAuth 提供商对应一个唯一的标识符，用于：
/// - 区分不同的 OAuth 服务
/// - 存储文件名命名
/// - 前端 UI 显示
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProviderId {
    /// GitHub Copilot
    GitHubCopilot,
    /// OpenAI
    OpenAI,
    /// Google Gemini
    GoogleGemini,
    /// 阿里巴巴通义千问
    AlibabaQwen,
    /// Moonshot AI (Kimi)
    MoonshotKimi,
    /// MiniMax
    MiniMax,
    /// VolcEngine (字节火山引擎 Ark)
    VolcEngineArk,
}

impl OAuthProviderId {
    /// 获取提供商显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            OAuthProviderId::GitHubCopilot => "GitHub Copilot",
            OAuthProviderId::OpenAI => "OpenAI",
            OAuthProviderId::GoogleGemini => "Google Gemini",
            OAuthProviderId::AlibabaQwen => "通义千问",
            OAuthProviderId::MoonshotKimi => "Moonshot Kimi",
            OAuthProviderId::MiniMax => "MiniMax",
            OAuthProviderId::VolcEngineArk => "火山引擎 Ark",
        }
    }

    /// 获取存储文件名
    pub fn storage_filename(&self) -> String {
        match self {
            OAuthProviderId::GitHubCopilot => "oauth_github_copilot.json".to_string(),
            OAuthProviderId::OpenAI => "oauth_openai.json".to_string(),
            OAuthProviderId::GoogleGemini => "oauth_google_gemini.json".to_string(),
            OAuthProviderId::AlibabaQwen => "oauth_alibaba_qwen.json".to_string(),
            OAuthProviderId::MoonshotKimi => "oauth_moonshot.json".to_string(),
            OAuthProviderId::MiniMax => "oauth_minimax.json".to_string(),
            OAuthProviderId::VolcEngineArk => "oauth_volcengine.json".to_string(),
        }
    }

    /// 获取 API 端点（部分提供商需要）
    pub fn api_endpoint(&self) -> Option<&'static str> {
        match self {
            OAuthProviderId::GitHubCopilot => Some("https://api.githubcopilot.com"),
            OAuthProviderId::OpenAI => Some("https://api.openai.com"),
            OAuthProviderId::GoogleGemini => Some("https://generativelanguage.googleapis.com"),
            OAuthProviderId::AlibabaQwen => Some("https://dashscope.aliyuncs.com"),
            OAuthProviderId::MoonshotKimi => Some("https://api.moonshot.cn"),
            OAuthProviderId::MiniMax => Some("https://api.minimax.chat"),
            OAuthProviderId::VolcEngineArk => Some("https://ark.cn-beijing.volces.com"),
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthProviderId::GitHubCopilot => "github_copilot",
            OAuthProviderId::OpenAI => "openai",
            OAuthProviderId::GoogleGemini => "google_gemini",
            OAuthProviderId::AlibabaQwen => "alibaba_qwen",
            OAuthProviderId::MoonshotKimi => "moonshot_kimi",
            OAuthProviderId::MiniMax => "minimax",
            OAuthProviderId::VolcEngineArk => "volcengine_ark",
        }
    }

    /// 获取所有支持的提供商
    pub fn all() -> &'static [OAuthProviderId] {
        &[
            OAuthProviderId::GitHubCopilot,
            OAuthProviderId::OpenAI,
            OAuthProviderId::GoogleGemini,
            OAuthProviderId::AlibabaQwen,
            OAuthProviderId::MoonshotKimi,
            OAuthProviderId::MiniMax,
            OAuthProviderId::VolcEngineArk,
        ]
    }

    /// 检查是否支持设备码流程
    pub fn supports_device_code(&self) -> bool {
        match self {
            // GitHub Copilot 使用设备码
            OAuthProviderId::GitHubCopilot => true,
            // 其他厂商可能需要根据实际情况调整
            _ => true,
        }
    }

    /// 检查是否需要额外的 token 交换
    ///
    /// 部分 OAuth 提供商在获取 access_token 后还需要额外步骤获取 API token
    pub fn requires_token_exchange(&self) -> bool {
        match self {
            // GitHub Copilot 需要额外交换 Copilot token
            OAuthProviderId::GitHubCopilot => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for OAuthProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for OAuthProviderId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github_copilot" => Ok(OAuthProviderId::GitHubCopilot),
            "openai" => Ok(OAuthProviderId::OpenAI),
            "google_gemini" => Ok(OAuthProviderId::GoogleGemini),
            "alibaba_qwen" | "qwen" => Ok(OAuthProviderId::AlibabaQwen),
            "moonshot_kimi" | "moonshot" | "kimi" => Ok(OAuthProviderId::MoonshotKimi),
            "minimax" => Ok(OAuthProviderId::MiniMax),
            "volcengine_ark" | "volcengine" | "ark" => Ok(OAuthProviderId::VolcEngineArk),
            _ => Err(format!("Unknown OAuth provider: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_display_name() {
        assert_eq!(OAuthProviderId::GitHubCopilot.display_name(), "GitHub Copilot");
        assert_eq!(OAuthProviderId::OpenAI.display_name(), "OpenAI");
        assert_eq!(OAuthProviderId::GoogleGemini.display_name(), "Google Gemini");
    }

    #[test]
    fn test_provider_id_storage_filename() {
        assert_eq!(
            OAuthProviderId::GitHubCopilot.storage_filename(),
            "oauth_github_copilot.json"
        );
        assert_eq!(
            OAuthProviderId::OpenAI.storage_filename(),
            "oauth_openai.json"
        );
    }

    #[test]
    fn test_provider_id_api_endpoint() {
        assert_eq!(
            OAuthProviderId::GitHubCopilot.api_endpoint(),
            Some("https://api.githubcopilot.com")
        );
        assert_eq!(
            OAuthProviderId::OpenAI.api_endpoint(),
            Some("https://api.openai.com")
        );
        assert_eq!(OAuthProviderId::GoogleGemini.api_endpoint(), Some("https://generativelanguage.googleapis.com"));
    }

    #[test]
    fn test_provider_id_from_str() {
        assert_eq!(
            "github_copilot".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::GitHubCopilot
        );
        assert_eq!(
            "openai".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::OpenAI
        );
        assert_eq!(
            "google_gemini".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::GoogleGemini
        );
        assert_eq!(
            "alibaba_qwen".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::AlibabaQwen
        );
        assert_eq!(
            "moonshot_kimi".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::MoonshotKimi
        );
        assert_eq!(
            "minimax".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::MiniMax
        );
        assert_eq!(
            "volcengine_ark".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::VolcEngineArk
        );
    }

    #[test]
    fn test_provider_id_all() {
        let all = OAuthProviderId::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_requires_token_exchange() {
        assert!(OAuthProviderId::GitHubCopilot.requires_token_exchange());
        assert!(!OAuthProviderId::OpenAI.requires_token_exchange());
        assert!(!OAuthProviderId::GoogleGemini.requires_token_exchange());
    }
}
