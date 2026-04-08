//! OAuth 模块
//!
//! 提供通用的 OAuth 认证管理框架，支持多种 OAuth 提供商。
//!
//! ## 支持的提供商
//! - GitHub Copilot
//! - OpenAI
//! - Google Gemini
//! - 阿里巴巴通义千问
//! - Moonshot AI (Kimi)
//! - MiniMax
//! - VolcEngine (字节火山引擎)
//!
//! ## 架构
//! - `provider_id`: 提供商唯一标识枚举
//! - `provider`: OAuth 类型定义
//! - `storage`: 通用账号存储管理
//! - `manager`: OAuth 管理器

mod manager;
pub mod provider;
mod provider_id;
mod storage;

// 各配置文件（供 manager 内部使用）
pub(crate) mod alibaba_qwen;
pub(crate) mod github_copilot;
pub(crate) mod google_gemini;
pub(crate) mod minimax;
pub(crate) mod moonshot;
pub(crate) mod openai;
pub(crate) mod volcengine;

// 核心组件
pub use manager::{OAuthAccountInfo, OAuthAuthStatus, OAuthDeviceCodeResponse, OAuthManager};
pub use provider::OAuthError;
pub use provider_id::OAuthProviderId;
