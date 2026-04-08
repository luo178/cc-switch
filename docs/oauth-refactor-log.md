# OAuth 多厂商认证重构文档

> 重构时间：2026-04-08
> 完成时间：2026-04-09
> 项目：cc-switch
> 目标：将 OAuth 认证从仅支持 GitHub Copilot 扩展到支持所有主要 AI 厂商

---

## 1. 需求背景

### 1.1 用户原始需求

用户提出：OAuth 认证应该支持所有厂商（类似 opencode auth login），而不只是 Anthropic、OpenAI、Google 这几家。

### 1.2 需要支持的 OAuth 厂商列表

| 厂商 | 支持OAuth的模型/服务 | 备注 |
|------|---------------------|------|
| OpenAI | GPT-4、GPT-5等系列 | 通过OpenAI OAuth社区库，或将OAuth作为Codex订阅的一部分提供。 |
| Google | Gemini系列 | 通过Gemini CLI、Vertex AI等服务支持OAuth。 |
| Anthropic | Claude 系列 | 可通过OAuth流程授权，或使用"setup-token"认证。 |
| 阿里巴巴 | Qwen (通义千问) | 原生支持OAuth免费层访问。 |
| Moonshot AI | Kimi | 通过CLI Proxy API等工具支持OAuth授权。 |
| MiniMax | MiniMax M2.5 系列 | 支持通过OAuth进行授权。 |
| VolcEngine | Ark Chat Model | 支持OAuth等双重认证方式 |

### 1.3 当前实现情况分析

**后端 (`src-tauri/src/`)**
- [auth.rs](src-tauri/src/proxy/providers/auth.rs) 定义了 `AuthStrategy` 枚举，其中 `GoogleOAuth` 和 `GitHubCopilot` 是 OAuth 方式
- [copilot_auth.rs](src-tauri/src/proxy/providers/copilot_auth.rs) 实现了 GitHub Copilot 的完整 OAuth 设备码流程（多账号支持、token 刷新、持久化等）
- [oauth/manager.rs](src-tauri/src/proxy/providers/oauth/manager.rs) 实现了通用的 `OAuthManager`，支持 7 个厂商的统一设备码流程
- [commands/auth.rs](src-tauri/src/commands/auth.rs) 实现了全部 OAuth 命令（`auth_start_login`、`auth_poll_for_account` 等）

**前端 (`src/`)**
- [useOAuthAuth.ts](src/components/providers/forms/hooks/useOAuthAuth.ts) 封装了完整的 OAuth 登录轮询逻辑
- [OAuthProviderSection.tsx](src/components/settings/OAuthProviderSection.tsx) 通用 OAuth 认证 UI 组件
- [AuthCenterPanel.tsx](src/components/settings/AuthCenterPanel.tsx) 展示全部 7 个厂商的认证区块
- [auth.ts](src/lib/api/auth.ts) 完整的 TypeScript API 类型定义

---

## 2. 架构设计

### 2.1 重构方案

```
src-tauri/src/proxy/providers/
├── auth.rs                      # AuthStrategy 枚举
├── oauth/                       # 通用 OAuth 模块
│   ├── mod.rs                   # 模块入口
│   ├── provider_id.rs           # OAuthProviderId 枚举（7个厂商）
│   ├── provider.rs              # OAuth 类型定义
│   ├── storage.rs               # 通用账号存储
│   ├── manager.rs               # OAuth 管理器（含 start_login/poll_for_account）
│   ├── github_copilot.rs        # GitHub Copilot 配置常量
│   ├── openai.rs                # OpenAI 配置常量
│   ├── google_gemini.rs         # Google Gemini 配置常量
│   ├── alibaba_qwen.rs          # 阿里巴巴通义千问配置常量
│   ├── moonshot.rs              # Moonshot AI (Kimi) 配置常量
│   ├── minimax.rs               # MiniMax 配置常量
│   └── volcengine.rs            # VolcEngine Ark 配置常量
└── commands/auth.rs             # OAuth 命令层
```

### 2.2 关键设计点

#### OAuthProviderConfig - 每个厂商的配置

```rust
struct OAuthProviderConfig {
    client_id: String,
    device_code_url: String,
    token_url: String,
    scopes: Vec<String>,
    user_info_url: Option<String>,
}
```

#### OAuthProviderId - 提供商标识枚举

```rust
pub enum OAuthProviderId {
    GitHubCopilot,    // GitHub Copilot
    OpenAI,           // OpenAI
    GoogleGemini,     // Google Gemini
    AlibabaQwen,      // 阿里巴巴通义千问
    MoonshotKimi,     // Moonshot AI (Kimi)
    MiniMax,          // MiniMax
    VolcEngineArk,   // 字节火山引擎 Ark
}
```

#### 统一存储格式 - 每个厂商独立存储文件

```
~/.config/cc-switch/
├── oauth_github_copilot.json
├── oauth_openai.json
├── oauth_google_gemini.json
├── oauth_alibaba_qwen.json
├── oauth_moonshot.json
├── oauth_minimax.json
└── oauth_volcengine.json
```

---

## 3. 实现过程

### 3.1 第一阶段：创建基础结构

#### 创建 oauth/mod.rs - 模块入口

```rust
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
//! - `manager`: OAuth 管理器（含 start_login/poll_for_account）

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

#### 创建 oauth/provider_id.rs - 提供商 ID 枚举

```rust
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
            OAuthProviderId::GitHubCopilot => true,
            _ => true,
        }
    }

    /// 检查是否需要额外的 token 交换
    pub fn requires_token_exchange(&self) -> bool {
        match self {
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
```

#### 创建 oauth/provider.rs - OAuth 类型定义

```rust
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
```

#### 创建 oauth/storage.rs - 通用账号存储

```rust
//! OAuth 存储管理
//!
//! 提供通用的 OAuth 账号存储和管理功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::provider_id::OAuthProviderId;
use super::provider::OAuthUserInfo;

/// OAuth 账号数据
///
/// 存储单个 OAuth 账号的完整信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccount {
    /// 账号唯一标识（由提供商返回的用户 ID）
    pub id: String,
    /// 用户名
    pub login: String,
    /// 邮箱
    #[serde(default)]
    pub email: Option<String>,
    /// 头像 URL
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// OAuth access_token
    pub access_token: String,
    /// 刷新令牌（如果有）
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Token 过期时间戳（Unix 秒）
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// 认证时间戳
    pub authenticated_at: i64,
}

impl OAuthAccount {
    /// 检查 token 是否即将过期（提前 60 秒）
    pub fn is_token_expiring_soon(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now().timestamp();
            expires_at - now < 60
        } else {
            false
        }
    }

    /// 从用户信息和 token 创建账号
    pub fn from_user_info(user_info: &OAuthUserInfo, access_token: String, expires_at: Option<i64>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: user_info.id.clone(),
            login: user_info.login.clone(),
            email: user_info.email.clone(),
            avatar_url: user_info.avatar_url.clone(),
            access_token,
            refresh_token: None,
            expires_at,
            authenticated_at: now,
        }
    }
}

/// OAuth 存储状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthStorageState {
    /// 存储格式版本
    #[serde(default)]
    pub version: u32,
    /// 账号列表（key = 账号 ID）
    #[serde(default)]
    pub accounts: HashMap<String, OAuthAccount>,
    /// 默认账号 ID
    #[serde(default)]
    pub default_account_id: Option<String>,
}

impl Default for OAuthStorageState {
    fn default() -> Self {
        Self {
            version: 1,
            accounts: HashMap::new(),
            default_account_id: None,
        }
    }
}

/// OAuth 存储管理器
///
/// 提供通用的 OAuth 账号存储和管理
pub struct OAuthStorage {
    /// 提供商 ID
    provider_id: OAuthProviderId,
    /// 账号数据
    accounts: Arc<RwLock<HashMap<String, OAuthAccount>>>,
    /// 默认账号 ID
    default_account_id: Arc<RwLock<Option<String>>>,
    /// 存储路径
    storage_path: PathBuf,
}

impl OAuthStorage {
    /// 创建新的存储管理器
    pub fn new(provider_id: OAuthProviderId, data_dir: PathBuf) -> Self {
        let storage_path = data_dir.join(provider_id.storage_filename());

        let storage = Self {
            provider_id,
            accounts: Arc::new(RwLock::new(HashMap::new())),
            default_account_id: Arc::new(RwLock::new(None)),
            storage_path,
        };

        // 尝试从磁盘加载
        let _ = storage.load_from_disk_sync();

        storage
    }

    /// 获取提供商 ID
    pub fn provider_id(&self) -> OAuthProviderId {
        self.provider_id
    }

    /// 添加账号
    pub async fn add_account(&self, account: OAuthAccount) {
        let account_id = account.id.clone();

        {
            let mut accounts = self.accounts.write().await;
            accounts.insert(account_id.clone(), account);
        }

        {
            let mut default_account_id = self.default_account_id.write().await;
            if default_account_id.is_none() {
                *default_account_id = Some(account_id);
            }
        }

        // 保存到磁盘
        let _ = self.save_to_disk().await;
    }

    /// 移除账号
    pub async fn remove_account(&self, account_id: &str) -> Result<(), String> {
        {
            let mut accounts = self.accounts.write().await;
            if accounts.remove(account_id).is_none() {
                return Err(format!("Account not found: {account_id}"));
            }
        }

        // 更新默认账号
        {
            let mut default_account_id = self.default_account_id.write().await;
            if default_account_id.as_deref() == Some(account_id) {
                let accounts = self.accounts.read().await;
                *default_account_id = accounts.keys().max().cloned();
            }
        }

        // 保存到磁盘
        self.save_to_disk().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 获取账号
    pub async fn get_account(&self, account_id: &str) -> Option<OAuthAccount> {
        let accounts = self.accounts.read().await;
        accounts.get(account_id).cloned()
    }

    /// 获取默认账号
    pub async fn get_default_account(&self) -> Option<OAuthAccount> {
        let default_account_id = self.default_account_id.read().await;
        if let Some(ref id) = *default_account_id {
            let accounts = self.accounts.read().await;
            accounts.get(id).cloned()
        } else {
            None
        }
    }

    /// 列出所有账号
    pub async fn list_accounts(&self) -> Vec<OAuthAccount> {
        let accounts = self.accounts.read().await;
        let default_account_id = self.default_account_id.read().await;
        let mut account_list: Vec<OAuthAccount> = accounts.values().cloned().collect();

        // 排序：默认账号优先，然后按认证时间倒序
        account_list.sort_by(|a, b| {
            let a_default = default_account_id.as_deref() == Some(&a.id);
            let b_default = default_account_id.as_deref() == Some(&b.id);
            b_default
                .cmp(&a_default)
                .then_with(|| b.authenticated_at.cmp(&a.authenticated_at))
        });

        account_list
    }

    /// 设置默认账号
    pub async fn set_default_account(&self, account_id: &str) -> Result<(), String> {
        {
            let accounts = self.accounts.read().await;
            if !accounts.contains_key(account_id) {
                return Err(format!("Account not found: {account_id}"));
            }
        }

        {
            let mut default_account_id = self.default_account_id.write().await;
            *default_account_id = Some(account_id.to_string());
        }

        // 保存到磁盘
        self.save_to_disk().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 更新账号
    pub async fn update_account(&self, account_id: &str, update: OAuthAccountUpdate) -> Result<(), String> {
        {
            let mut accounts = self.accounts.write().await;
            let account = accounts.get_mut(account_id)
                .ok_or_else(|| format!("Account not found: {account_id}"))?;

            if let Some(access_token) = update.access_token {
                account.access_token = access_token;
            }
            if let Some(refresh_token) = update.refresh_token {
                account.refresh_token = Some(refresh_token);
            }
            if let Some(expires_at) = update.expires_at {
                account.expires_at = Some(expires_at);
            }
        }

        // 保存到磁盘
        self.save_to_disk().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 检查是否已认证
    pub async fn is_authenticated(&self) -> bool {
        let accounts = self.accounts.read().await;
        !accounts.is_empty()
    }

    /// 清除所有账号
    pub async fn clear(&self) -> Result<(), String> {
        {
            let mut accounts = self.accounts.write().await;
            accounts.clear();
        }
        {
            let mut default_account_id = self.default_account_id.write().await;
            default_account_id.take();
        }

        // 删除存储文件
        if self.storage_path.exists() {
            fs::remove_file(&self.storage_path)
                .map_err(|e| format!("Failed to remove storage file: {e}"))?;
        }

        Ok(())
    }

    /// 获取默认账号 ID
    pub async fn get_default_account_id(&self) -> Option<String> {
        self.default_account_id.read().await.clone()
    }

    /// 从磁盘加载
    fn load_from_disk_sync(&self) -> Result<(), String> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.storage_path)
            .map_err(|e| format!("Failed to read storage: {e}"))?;

        let state: OAuthStorageState = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse storage: {e}"))?;

        // 更新内存状态
        {
            let mut accounts = self.accounts.try_write()
                .map_err(|_| "Storage is busy")?;
            *accounts = state.accounts;
        }
        {
            let mut default_account_id = self.default_account_id.try_write()
                .map_err(|_| "Storage is busy")?;
            *default_account_id = state.default_account_id;
        }

        Ok(())
    }

    /// 保存到磁盘
    async fn save_to_disk(&self) -> Result<(), String> {
        let accounts = self.accounts.read().await.clone();
        let default_account_id = self.default_account_id.read().await.clone();

        let state = OAuthStorageState {
            version: 1,
            accounts,
            default_account_id,
        };

        let content = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize: {e}"))?;

        // 原子写入
        self.write_store_atomic(&content)?;

        Ok(())
    }

    /// 原子写入文件
    fn write_store_atomic(&self, content: &str) -> Result<(), String> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }

        let parent = self.storage_path.parent()
            .ok_or_else(|| "Invalid storage path".to_string())?;
        let file_name = self.storage_path.file_name()
            .ok_or_else(|| "Invalid storage filename".to_string())?
            .to_string_lossy()
            .to_string();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_path = parent.join(format!("{file_name}.tmp.{ts}"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&tmp_path)
                .map_err(|e| format!("Failed to create temp file: {e}"))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write temp file: {e}"))?;
            file.flush()
                .map_err(|e| format!("Failed to flush temp file: {e}"))?;

            fs::rename(&tmp_path, &self.storage_path)
                .map_err(|e| format!("Failed to rename temp file: {e}"))?;
            fs::set_permissions(&self.storage_path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("Failed to set permissions: {e}"))?;
        }

        #[cfg(windows)]
        {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&tmp_path)
                .map_err(|e| format!("Failed to create temp file: {e}"))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write temp file: {e}"))?;
            file.flush()
                .map_err(|e| format!("Failed to flush temp file: {e}"))?;

            if self.storage_path.exists() {
                let _ = fs::remove_file(&self.storage_path);
            }
            fs::rename(&tmp_path, &self.storage_path)
                .map_err(|e| format!("Failed to rename temp file: {e}"))?;
        }

        Ok(())
    }
}

/// OAuth 账号更新
pub struct OAuthAccountUpdate {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

impl OAuthAccountUpdate {
    pub fn new() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            expires_at: None,
        }
    }

    pub fn with_access_token(mut self, token: String) -> Self {
        self.access_token = Some(token);
        self
    }

    pub fn with_refresh_token(mut self, token: String) -> Self {
        self.refresh_token = Some(token);
        self
    }

    pub fn with_expires_at(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

impl Default for OAuthAccountUpdate {
    fn default() -> Self {
        Self::new()
    }
}
```

#### 创建 oauth/manager.rs - OAuth 管理器

```rust
//! OAuth 管理器
//!
//! 统一的 OAuth 认证管理器，协调所有 OAuth 提供商的认证操作

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::proxy::providers::oauth::provider::{DeviceCodeResponse, OAuthError, TokenResponse};
use crate::proxy::providers::oauth::provider_id::OAuthProviderId;
use crate::proxy::providers::oauth::storage::{OAuthAccount, OAuthStorage};

/// OAuth 账号公开信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthAccountInfo {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
}

impl From<&OAuthAccount> for OAuthAccountInfo {
    fn from(account: &OAuthAccount) -> Self {
        Self {
            id: account.id.clone(),
            provider: "".to_string(),
            login: account.login.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            authenticated_at: account.authenticated_at,
            is_default: false,
        }
    }
}

impl OAuthAccountInfo {
    pub fn from_account(account: &OAuthAccount, provider_id: &OAuthProviderId) -> Self {
        Self {
            id: account.id.clone(),
            provider: provider_id.as_str().to_string(),
            login: account.login.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            authenticated_at: account.authenticated_at,
            is_default: false,
        }
    }
}

/// OAuth 设备码响应（带提供商信息）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

impl From<(OAuthProviderId, DeviceCodeResponse)> for OAuthDeviceCodeResponse {
    fn from((provider, response): (OAuthProviderId, DeviceCodeResponse)) -> Self {
        Self {
            provider: provider.as_str().to_string(),
            device_code: response.device_code,
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            expires_in: response.expires_in,
            interval: response.interval,
        }
    }
}

/// OAuth 认证状态
#[derive(Debug, Clone)]
pub struct OAuthAuthStatus {
    pub provider: OAuthProviderId,
    pub authenticated: bool,
    pub default_account_id: Option<String>,
    pub accounts: Vec<OAuthAccountInfo>,
}

impl OAuthAuthStatus {
    pub fn new(provider: OAuthProviderId) -> Self {
        Self {
            provider,
            authenticated: false,
            default_account_id: None,
            accounts: Vec::new(),
        }
    }
}

/// OAuth 管理器
///
/// 管理所有 OAuth 提供商的认证状态和操作
pub struct OAuthManager {
    /// 提供商存储（每个提供商一个存储实例）
    storages: Arc<RwLock<HashMap<OAuthProviderId, Arc<OAuthStorage>>>>,
    /// 数据目录
    data_dir: PathBuf,
}

impl OAuthManager {
    /// 创建新的 OAuth 管理器
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            storages: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// 获取或创建存储
    pub async fn get_storage(&self, provider_id: OAuthProviderId) -> Arc<OAuthStorage> {
        // 快速路径：检查缓存
        {
            let storages = self.storages.read().await;
            if let Some(storage) = storages.get(&provider_id) {
                return Arc::clone(storage);
            }
        }

        // 创建新存储
        let storage = Arc::new(OAuthStorage::new(provider_id, self.data_dir.clone()));

        // 缓存
        {
            let mut storages = self.storages.write().await;
            storages.insert(provider_id, Arc::clone(&storage));
        }

        storage
    }

    /// 获取认证状态
    pub async fn get_auth_status(&self, provider_id: OAuthProviderId) -> OAuthAuthStatus {
        let storage = self.get_storage(provider_id).await;
        let accounts = storage.list_accounts().await;
        let default_account_id = storage.get_default_account_id().await;

        let provider = provider_id.as_str().to_string();
        let account_infos: Vec<OAuthAccountInfo> = accounts
            .iter()
            .map(|a| {
                let mut info = OAuthAccountInfo::from_account(a, &provider_id);
                info.provider = provider.clone();
                info.is_default = Some(&a.id) == default_account_id.as_ref();
                info
            })
            .collect();

        OAuthAuthStatus {
            provider: provider_id,
            authenticated: !accounts.is_empty(),
            default_account_id,
            accounts: account_infos,
        }
    }

    /// 移除账号
    pub async fn remove_account(&self, provider_id: OAuthProviderId, account_id: &str) -> Result<(), String> {
        let storage = self.get_storage(provider_id).await;
        storage.remove_account(account_id).await
    }

    /// 设置默认账号
    pub async fn set_default_account(&self, provider_id: OAuthProviderId, account_id: &str) -> Result<(), String> {
        let storage = self.get_storage(provider_id).await;
        storage.set_default_account(account_id).await
    }

    /// 清除所有认证
    pub async fn clear_auth(&self, provider_id: OAuthProviderId) -> Result<(), String> {
        let storage = self.get_storage(provider_id).await;
        storage.clear().await
    }

    /// 列出所有账号
    pub async fn list_accounts(&self, provider_id: OAuthProviderId) -> Vec<OAuthAccountInfo> {
        let storage = self.get_storage(provider_id).await;
        let accounts = storage.list_accounts().await;
        let default_account_id = storage.get_default_account_id().await;

        let provider = provider_id.as_str().to_string();
        accounts
            .into_iter()
            .map(|a| {
                let mut info = OAuthAccountInfo::from_account(&a, &provider_id);
                info.provider = provider.clone();
                info.is_default = Some(&a.id) == default_account_id.as_ref();
                info
            })
            .collect()
    }

    /// 检查是否已认证
    pub async fn is_authenticated(&self, provider_id: OAuthProviderId) -> bool {
        let storage = self.get_storage(provider_id).await;
        storage.is_authenticated().await
    }

    /// 获取提供商数据目录
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}
```

### 3.2 第二阶段：创建各厂商配置

#### oauth/github_copilot.rs

```rust
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
pub const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";

/// GitHub User API URL
pub const GITHUB_USER_URL: &str = "https://api.github.com/user";

/// Copilot 使用量 API URL
pub const COPILOT_USAGE_URL: &str = "https://api.github.com/copilot_internal/user";

/// 默认 Copilot API 端点
pub const DEFAULT_COPILOT_API_ENDPOINT: &str = "https://api.githubcopilot.com";

/// Copilot API Header 常量
pub const COPILOT_EDITOR_VERSION: &str = "vscode/1.110.1";
pub const COPILOT_PLUGIN_VERSION: &str = "copilot-chat/0.38.2";
pub const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.38.2";
pub const COPILOT_API_VERSION: &str = "2025-10-01";

/// GitHub Copilot OAuth 配置
#[derive(Debug, Clone)]
pub struct GitHubCopilotOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for GitHubCopilotOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: GITHUB_CLIENT_ID.to_string(),
            scopes: vec!["read:user".to_string()],
        }
    }
}

impl GitHubCopilotOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec!["read:user".to_string()],
        }
    }
}
```

#### oauth/openai.rs

```rust
//! OpenAI OAuth 配置
//!
//! 定义 OpenAI 的 OAuth 配置信息

use super::provider_id::OAuthProviderId;

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
pub const OPENAI_API_BASE_URL: &str = "https://api.openai.com";

/// OpenAI OAuth 配置
#[derive(Debug, Clone)]
pub struct OpenAIOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for OpenAIOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: OPENAI_CLIENT_ID.to_string(),
            scopes: vec![
                "openid".to_string(),
                "model.read".to_string(),
                "offline".to_string(),
            ],
        }
    }
}

impl OpenAIOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec![
                "openid".to_string(),
                "model.read".to_string(),
                "offline".to_string(),
            ],
        }
    }
}
```

#### oauth/google_gemini.rs

```rust
//! Google Gemini OAuth 配置
//!
//! 定义 Google Gemini 的 OAuth 配置信息

use super::provider_id::OAuthProviderId;

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
pub const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// Google OAuth 配置
#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: GOOGLE_CLIENT_ID.to_string(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
                "https://www.googleapis.com/auth/generative-language.retriever".to_string(),
            ],
        }
    }
}

impl GoogleOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
                "https://www.googleapis.com/auth/generative-language.retriever".to_string(),
            ],
        }
    }
}
```

#### oauth/alibaba_qwen.rs

```rust
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
pub const DASHSCOPE_API_BASE_URL: &str = "https://dashscope.aliyuncs.com";

/// 阿里云 OAuth 配置
#[derive(Debug, Clone)]
pub struct AlibabaOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for AlibabaOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: ALIBABA_CLIENT_ID.to_string(),
            scopes: vec!["openapi".to_string()],
        }
    }
}

impl AlibabaOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec!["openapi".to_string()],
        }
    }
}
```

#### oauth/moonshot.rs

```rust
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
pub const MOONSHOT_API_BASE_URL: &str = "https://api.moonshot.cn";

/// Moonshot OAuth 配置
#[derive(Debug, Clone)]
pub struct MoonshotOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for MoonshotOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: MOONSHOT_CLIENT_ID.to_string(),
            scopes: vec![
                "user.info".to_string(),
                "chatplt.compact".to_string(),
            ],
        }
    }
}

impl MoonshotOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec![
                "user.info".to_string(),
                "chatplt.compact".to_string(),
            ],
        }
    }
}
```

#### oauth/minimax.rs

```rust
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
pub const MINIMAX_API_BASE_URL: &str = "https://api.minimax.chat";

/// MiniMax OAuth 配置
#[derive(Debug, Clone)]
pub struct MiniMaxOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for MiniMaxOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: MINIMAX_CLIENT_ID.to_string(),
            scopes: vec![
                "user.base".to_string(),
                "chat.default".to_string(),
            ],
        }
    }
}

impl MiniMaxOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec![
                "user.base".to_string(),
                "chat.default".to_string(),
            ],
        }
    }
}
```

#### oauth/volcengine.rs

```rust
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
pub const VOLCENGINE_API_BASE_URL: &str = "https://ark.cn-beijing.volces.com";

/// VolcEngine OAuth 配置
#[derive(Debug, Clone)]
pub struct VolcEngineOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl Default for VolcEngineOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: VOLCENGINE_CLIENT_ID.to_string(),
            scopes: vec!["ark".to_string()],
        }
    }
}

impl VolcEngineOAuthConfig {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            scopes: vec!["ark".to_string()],
        }
    }
}
```

### 3.3 第三阶段：OAuthManager 核心实现

#### oauth/manager.rs

`OAuthManager` 是统一管理 7 个 OAuth 厂商的核心组件，包含 `start_login` 和 `poll_for_account` 两个关键方法。

**结构体定义：**

```rust
pub struct OAuthManager {
    storages: Arc<RwLock<HashMap<OAuthProviderId, Arc<OAuthStorage>>>>,
    data_dir: PathBuf,
    http_client: reqwest::Client,
    copilot_auth: Option<Arc<CopilotAuthManager>>,
}
```

**start_login 方法：**

```rust
pub async fn start_login(&self, provider_id: OAuthProviderId)
    -> Result<OAuthDeviceCodeResponse, OAuthError>
```

- GitHub Copilot：调用 `CopilotAuthManager::start_device_flow()`
- 其他厂商：HTTP POST 到 `device_code_url`

**poll_for_account 方法：**

```rust
pub async fn poll_for_account(
    &self,
    provider_id: OAuthProviderId,
    device_code: &str,
) -> Result<Option<OAuthAccountInfo>, OAuthError>
```

- GitHub Copilot：调用 `CopilotAuthManager::poll_for_token()`
- 其他厂商：HTTP POST 到 `token_url`，解析 user_info，保存账号

**Provider 配置（get_provider_config）：**

```rust
fn get_provider_config(&self, provider_id: OAuthProviderId) -> ProviderOAuthConfig
```

每个厂商的配置内联在此方法中，包括 client_id、device_code_url、token_url、user_info_url、scopes。

use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use super::provider_id::OAuthProviderId;
use super::provider::{DeviceCodeResponse, OAuthError, OAuthUserInfo, TokenResponse};
use crate::proxy::providers::oauth::github_copilot::{
    COPILOT_USER_AGENT, COPILOT_EDITOR_VERSION, COPILOT_PLUGIN_VERSION,
};
use crate::proxy::providers::oauth::storage::OAuthAccount;

/// OAuth 流程处理器
pub struct OAuthProcessor {
    http_client: Client,
}

impl OAuthProcessor {
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self { http_client }
    }

    /// 启动设备码授权流程
    pub async fn start_device_flow(
        &self,
        provider_id: OAuthProviderId,
    ) -> Result<DeviceCodeResponse, OAuthError> {
        match provider_id {
            OAuthProviderId::GitHubCopilot => self.github_copilot_device_flow().await,
            OAuthProviderId::OpenAI => self.openai_device_flow().await,
            OAuthProviderId::GoogleGemini => self.google_device_flow().await,
            OAuthProviderId::AlibabaQwen => self.alibaba_device_flow().await,
            OAuthProviderId::MoonshotKimi => self.moonshot_device_flow().await,
            OAuthProviderId::MiniMax => self.minimax_device_flow().await,
            OAuthProviderId::VolcEngineArk => self.volcengine_device_flow().await,
        }
    }

    /// 轮询获取 Token
    pub async fn poll_for_token(
        &self,
        provider_id: OAuthProviderId,
        device_code: &str,
    ) -> Result<TokenResponse, OAuthError> {
        match provider_id {
            OAuthProviderId::GitHubCopilot => self.github_copilot_poll_token(device_code).await,
            OAuthProviderId::OpenAI => self.openai_poll_token(device_code).await,
            OAuthProviderId::GoogleGemini => self.google_poll_token(device_code).await,
            OAuthProviderId::AlibabaQwen => self.alibaba_poll_token(device_code).await,
            OAuthProviderId::MoonshotKimi => self.moonshot_poll_token(device_code).await,
            OAuthProviderId::MiniMax => self.minimax_poll_token(device_code).await,
            OAuthProviderId::VolcEngineArk => self.volcengine_poll_token(device_code).await,
        }
    }

    /// 获取用户信息
    pub async fn get_user_info(
        &self,
        provider_id: OAuthProviderId,
        access_token: &str,
    ) -> Result<OAuthUserInfo, OAuthError> {
        match provider_id {
            OAuthProviderId::GitHubCopilot => self.github_user_info(access_token).await,
            OAuthProviderId::OpenAI => self.openai_user_info(access_token).await,
            OAuthProviderId::GoogleGemini => self.google_user_info(access_token).await,
            OAuthProviderId::AlibabaQwen => self.alibaba_user_info(access_token).await,
            OAuthProviderId::MoonshotKimi => self.moonshot_user_info(access_token).await,
            OAuthProviderId::MiniMax => self.minimax_user_info(access_token).await,
            OAuthProviderId::VolcEngineArk => self.volcengine_user_info(access_token).await,
        }
    }

    /// 检查订阅状态
    pub async fn check_subscription(
        &self,
        provider_id: OAuthProviderId,
        token: &str,
    ) -> Result<bool, OAuthError> {
        match provider_id {
            OAuthProviderId::GitHubCopilot => self.github_check_subscription(token).await,
            // 其他提供商默认假设已订阅
            _ => Ok(true),
        }
    }

    // ==================== GitHub Copilot ====================

    const GITHUB_CLIENT_ID: &'static str = "Iv1.b507a08c87ecfe98";
    const GITHUB_DEVICE_CODE_URL: &'static str = "https://github.com/login/device/code";
    const GITHUB_TOKEN_URL: &'static str = "https://github.com/login/oauth/access_token";
    const GITHUB_USER_URL: &'static str = "https://api.github.com/user";
    const COPILOT_TOKEN_URL: &'static str = "https://api.github.com/copilot_internal/v2/token";

    async fn github_copilot_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        let response = self
            .http_client
            .post(Self::GITHUB_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .header("User-Agent", COPILOT_USER_AGENT)
            .form(&[
                ("client_id", Self::GITHUB_CLIENT_ID),
                ("scope", "read:user"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthError::NetworkError(format!(
                "GitHub 设备码请求失败: {status} - {text}"
            )));
        }

        let device_code: GitHubDeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(DeviceCodeResponse {
            device_code: device_code.device_code,
            user_code: device_code.user_code,
            verification_uri: device_code.verification_uri,
            expires_in: device_code.expires_in,
            interval: device_code.interval,
        })
    }

    async fn github_copilot_poll_token(&self, device_code: &str) -> Result<TokenResponse, OAuthError> {
        let response = self
            .http_client
            .post(Self::GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .header("User-Agent", COPILOT_USER_AGENT)
            .form(&[
                ("client_id", Self::GITHUB_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let oauth_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        if let Some(error) = oauth_response.error {
            return match error.as_str() {
                "authorization_pending" => Err(OAuthError::AuthorizationPending),
                "slow_down" => Err(OAuthError::AuthorizationPending),
                "expired_token" => Err(OAuthError::ExpiredToken),
                "access_denied" => Err(OAuthError::AccessDenied),
                _ => Err(OAuthError::TokenExchangeFailed(format!(
                    "{}: {}",
                    error,
                    oauth_response.error_description.unwrap_or_default()
                ))),
            };
        }

        let access_token = oauth_response.access_token
            .ok_or_else(|| OAuthError::ParseError("缺少 access_token".to_string()))?;

        Ok(TokenResponse {
            access_token,
            token_type: oauth_response.token_type.unwrap_or_default(),
            expires_in: oauth_response.expires_in,
            refresh_token: oauth_response.refresh_token,
            scope: oauth_response.scope,
            id_token: oauth_response.id_token,
        })
    }

    async fn github_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let response = self
            .http_client
            .get(Self::GITHUB_USER_URL)
            .header("Authorization", format!("token {access_token}"))
            .header("User-Agent", COPILOT_USER_AGENT)
            .header("Editor-Version", COPILOT_EDITOR_VERSION)
            .header("Editor-Plugin-Version", COPILOT_PLUGIN_VERSION)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OAuthError::TokenInvalid);
        }

        let user: GitHubUser = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(OAuthUserInfo {
            id: user.id.to_string(),
            login: user.login,
            email: user.email,
            avatar_url: user.avatar_url,
            raw: serde_json::json!({}),
        })
    }

    async fn github_check_subscription(&self, github_token: &str) -> Result<bool, OAuthError> {
        let response = self
            .http_client
            .get(Self::COPILOT_TOKEN_URL)
            .header("Authorization", format!("token {github_token}"))
            .header("User-Agent", COPILOT_USER_AGENT)
            .header("Editor-Version", COPILOT_EDITOR_VERSION)
            .header("Editor-Plugin-Version", COPILOT_PLUGIN_VERSION)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(OAuthError::TokenInvalid);
        }

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(OAuthError::NoSubscription);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "{status}: {text}"
            )));
        }

        let _: CopilotTokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(true)
    }

    // ==================== OpenAI ====================

    const OPENAI_CLIENT_ID: &'static str = "YOUR_OPENAI_CLIENT_ID";
    const OPENAI_DEVICE_CODE_URL: &'static str = "https://openai.com/oauth/device/code";
    const OPENAI_TOKEN_URL: &'static str = "https://openai.com/oauth/token";
    const OPENAI_USER_URL: &'static str = "https://api.openai.com/v1/user";

    async fn openai_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        let response = self
            .http_client
            .post(Self::OPENAI_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", Self::OPENAI_CLIENT_ID),
                ("scope", "openid model.read offline"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthError::NetworkError(format!(
                "OpenAI 设备码请求失败: {status} - {text}"
            )));
        }

        let device_code: GenericDeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(DeviceCodeResponse {
            device_code: device_code.device_code,
            user_code: device_code.user_code,
            verification_uri: device_code.verification_uri,
            expires_in: device_code.expires_in,
            interval: device_code.interval,
        })
    }

    async fn openai_poll_token(&self, device_code: &str) -> Result<TokenResponse, OAuthError> {
        let response = self
            .http_client
            .post(Self::OPENAI_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", Self::OPENAI_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        self.parse_token_response(response).await
    }

    async fn openai_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let response = self
            .http_client
            .get(Self::OPENAI_USER_URL)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OAuthError::TokenInvalid);
        }

        let user: OpenAIUser = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(OAuthUserInfo {
            id: user.id,
            login: user.name.unwrap_or_else(|| user.email.clone().unwrap_or_default()),
            email: Some(user.email),
            avatar_url: None,
            raw: serde_json::json!({}),
        })
    }

    // ==================== Google ====================

    const GOOGLE_CLIENT_ID: &'static str = "YOUR_GOOGLE_CLIENT_ID";
    const GOOGLE_DEVICE_CODE_URL: &'static str = "https://oauth2.googleapis.com/device/code";
    const GOOGLE_TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    const GOOGLE_USER_URL: &'static str = "https://www.googleapis.com/oauth2/v3/userinfo";

    async fn google_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        let response = self
            .http_client
            .post(Self::GOOGLE_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", Self::GOOGLE_CLIENT_ID),
                ("scope", "openid email profile"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthError::NetworkError(format!(
                "Google 设备码请求失败: {status} - {text}"
            )));
        }

        let device_code: GenericDeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(DeviceCodeResponse {
            device_code: device_code.device_code,
            user_code: device_code.user_code,
            verification_uri: device_code.verification_uri,
            expires_in: device_code.expires_in,
            interval: device_code.interval,
        })
    }

    async fn google_poll_token(&self, device_code: &str) -> Result<TokenResponse, OAuthError> {
        let response = self
            .http_client
            .post(Self::GOOGLE_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", Self::GOOGLE_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        self.parse_token_response(response).await
    }

    async fn google_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let response = self
            .http_client
            .get(Self::GOOGLE_USER_URL)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OAuthError::TokenInvalid);
        }

        let user: GoogleUser = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(OAuthUserInfo {
            id: user.sub,
            login: user.name.unwrap_or_else(|| user.email.clone().unwrap_or_default()),
            email: Some(user.email),
            avatar_url: user.picture,
            raw: serde_json::json!({}),
        })
    }

    // ==================== 通用方法 ====================

    async fn parse_token_response(&self, response: reqwest::Response) -> Result<TokenResponse, OAuthError> {
        let oauth_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        if let Some(error) = oauth_response.error {
            return match error.as_str() {
                "authorization_pending" => Err(OAuthError::AuthorizationPending),
                "slow_down" => Err(OAuthError::AuthorizationPending),
                "expired_token" => Err(OAuthError::ExpiredToken),
                "access_denied" => Err(OAuthError::AccessDenied),
                _ => Err(OAuthError::TokenExchangeFailed(format!(
                    "{}: {}",
                    error,
                    oauth_response.error_description.unwrap_or_default()
                ))),
            };
        }

        let access_token = oauth_response.access_token
            .ok_or_else(|| OAuthError::ParseError("缺少 access_token".to_string()))?;

        Ok(TokenResponse {
            access_token,
            token_type: oauth_response.token_type.unwrap_or_default(),
            expires_in: oauth_response.expires_in,
            refresh_token: oauth_response.refresh_token,
            scope: oauth_response.scope,
            id_token: oauth_response.id_token,
        })
    }

    // ==================== 占位实现 ====================

    // 其他提供商的设备码 URL（占位）
    const ALIBABA_DEVICE_CODE_URL: &'static str = "https://oauth.aliyun.com/device/code";
    const MOONSHOT_DEVICE_CODE_URL: &'static str = "https://platform.moonshot.cn/oauth/device/code";
    const MINIMAX_DEVICE_CODE_URL: &'static str = "https://api.minimax.chat/oauth/device/code";
    const VOLCENGINE_DEVICE_CODE_URL: &'static str = "https://ark.cn-beijing.volces.com/oauth/device/code";

    async fn alibaba_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("alibaba_qwen".to_string()))
    }

    async fn moonshot_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("moonshot_kimi".to_string()))
    }

    async fn minimax_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("minimax".to_string()))
    }

    async fn volcengine_device_flow(&self) -> Result<DeviceCodeResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("volcengine_ark".to_string()))
    }

    async fn alibaba_poll_token(&self, _device_code: &str) -> Result<TokenResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("alibaba_qwen".to_string()))
    }

    async fn moonshot_poll_token(&self, _device_code: &str) -> Result<TokenResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("moonshot_kimi".to_string()))
    }

    async fn minimax_poll_token(&self, _device_code: &str) -> Result<TokenResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("minimax".to_string()))
    }

    async fn volcengine_poll_token(&self, _device_code: &str) -> Result<TokenResponse, OAuthError> {
        Err(OAuthError::UnsupportedProvider("volcengine_ark".to_string()))
    }

    async fn alibaba_user_info(&self, _access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        Err(OAuthError::UnsupportedProvider("alibaba_qwen".to_string()))
    }

    async fn moonshot_user_info(&self, _access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        Err(OAuthError::UnsupportedProvider("moonshot_kimi".to_string()))
    }

    async fn minimax_user_info(&self, _access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        Err(OAuthError::UnsupportedProvider("minimax".to_string()))
    }

    async fn volcengine_user_info(&self, _access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        Err(OAuthError::UnsupportedProvider("volcengine_ark".to_string()))
    }
}

impl Default for OAuthProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 类型定义 ====================

#[derive(Debug, Deserialize)]
struct GitHubDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct GenericDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
    id_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
    id: u64,
    avatar_url: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUser {
    id: String,
    name: Option<String>,
    email: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUser {
    sub: String,
    name: Option<String>,
    email: String,
    picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: i64,
}
```

### 3.4 第四阶段：命令层

#### commands/auth.rs

```rust
use tauri::State;
use std::sync::Arc;

use crate::proxy::providers::oauth::{
    OAuthAccountInfo, OAuthAuthStatus, OAuthDeviceCodeResponse, OAuthError, OAuthManager, OAuthProviderId,
};

/// OAuth 认证状态
pub struct OAuthAuthState(pub Arc<OAuthManager>);

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthAccount {
    pub id: String,
    pub provider: String,
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
}

impl From<&OAuthAccountInfo> for ManagedAuthAccount {
    fn from(info: &OAuthAccountInfo) -> Self {
        Self {
            id: info.id.clone(),
            provider: info.provider.clone(),
            login: info.login.clone(),
            email: info.email.clone(),
            avatar_url: info.avatar_url.clone(),
            authenticated_at: info.authenticated_at,
            is_default: info.is_default,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_error: Option<String>,
    pub accounts: Vec<ManagedAuthAccount>,
}

impl From<&OAuthAuthStatus> for ManagedAuthStatus {
    fn from(status: &OAuthAuthStatus) -> Self {
        Self {
            provider: status.provider.as_str().to_string(),
            authenticated: status.authenticated,
            default_account_id: status.default_account_id.clone(),
            migration_error: None,
            accounts: status.accounts.iter().map(ManagedAuthAccount::from).collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

impl From<&OAuthDeviceCodeResponse> for ManagedAuthDeviceCodeResponse {
    fn from(resp: &OAuthDeviceCodeResponse) -> Self {
        Self {
            provider: resp.provider.clone(),
            device_code: resp.device_code.clone(),
            user_code: resp.user_code.clone(),
            verification_uri: resp.verification_uri.clone(),
            expires_in: resp.expires_in,
            interval: resp.interval,
        }
    }
}

/// 将提供商字符串转换为 OAuthProviderId
fn parse_provider_id(provider: &str) -> Result<OAuthProviderId, String> {
    provider.parse::<OAuthProviderId>().map_err(|e| e.to_string())
}

fn map_oauth_error(e: OAuthError) -> String {
    match e {
        OAuthError::AuthorizationPending => "authorization_pending".to_string(),
        OAuthError::AccessDenied => "access_denied".to_string(),
        OAuthError::ExpiredToken => "expired_token".to_string(),
        OAuthError::NoSubscription => "no_subscription".to_string(),
        _ => e.to_string(),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_start_login(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<ManagedAuthDeviceCodeResponse, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let resp = state
        .0
        .start_login(provider_id)
        .await
        .map_err(map_oauth_error)?;
    Ok(ManagedAuthDeviceCodeResponse::from(&resp))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_poll_for_account(
    auth_provider: String,
    device_code: String,
    state: State<'_, OAuthAuthState>,
) -> Result<Option<ManagedAuthAccount>, String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    let account = state
        .0
        .poll_for_account(provider_id, &device_code)
        .await
        .map_err(map_oauth_error)?;
    Ok(account.map(|a| ManagedAuthAccount::from(&a)))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_list_accounts(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<Vec<ManagedAuthAccount>, String> {
    let provider_id = parse_provider_id(&auth_provider)?;

    let accounts = state.0.list_accounts(provider_id).await;
    Ok(accounts.iter().map(ManagedAuthAccount::from).collect())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_get_status(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<ManagedAuthStatus, String> {
    let provider_id = parse_provider_id(&auth_provider)?;

    let status = state.0.get_auth_status(provider_id).await;
    Ok(ManagedAuthStatus::from(&status))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_remove_account(
    auth_provider: String,
    account_id: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state
        .0
        .remove_account(provider_id, &account_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_set_default_account(
    auth_provider: String,
    account_id: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state
        .0
        .set_default_account(provider_id, &account_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_logout(
    auth_provider: String,
    state: State<'_, OAuthAuthState>,
) -> Result<(), String> {
    let provider_id = parse_provider_id(&auth_provider)?;
    state
        .0
        .clear_auth(provider_id)
        .await
        .map_err(|e| e.to_string())
}

/// 列出所有支持的 OAuth 提供商
#[tauri::command(rename_all = "camelCase")]
pub fn auth_list_providers() -> Vec<OAuthProviderInfo> {
    OAuthProviderId::all()
        .iter()
        .map(|id| OAuthProviderInfo {
            id: id.as_str().to_string(),
            name: id.display_name().to_string(),
            supports_device_code: id.supports_device_code(),
            requires_token_exchange: id.requires_token_exchange(),
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthProviderInfo {
    pub id: String,
    pub name: String,
    pub supports_device_code: bool,
    pub requires_token_exchange: bool,
}
```

### 3.5 第五阶段：前端实现

#### src/lib/api/auth.ts

```typescript
import { invoke } from "@tauri-apps/api/core";

/// OAuth 提供商 ID
export type OAuthProviderId =
  | "github_copilot"
  | "openai"
  | "google_gemini"
  | "alibaba_qwen"
  | "moonshot_kimi"
  | "minimax"
  | "volcengine_ark";

/// OAuth 提供商信息
export interface OAuthProviderInfo {
  id: string;
  name: string;
  supports_device_code: boolean;
  requires_token_exchange: boolean;
}

/// OAuth 账号信息
export interface OAuthAccount {
  id: string;
  provider: OAuthProviderId;
  login: string;
  email?: string | null;
  avatar_url?: string | null;
  authenticated_at: number;
  is_default: boolean;
}

/// OAuth 认证状态
export interface OAuthAuthStatus {
  provider: OAuthProviderId;
  authenticated: boolean;
  default_account_id: string | null;
  migration_error?: string | null;
  accounts: OAuthAccount[];
}

/// OAuth 设备码响应
export interface OAuthDeviceCodeResponse {
  provider: OAuthProviderId;
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

// ==================== API 函数 ====================

/// 列出所有支持的 OAuth 提供商
export async function authListProviders(): Promise<OAuthProviderInfo[]> {
  return invoke<OAuthProviderInfo[]>("auth_list_providers");
}

/// 启动 OAuth 登录流程
export async function authStartLogin(
  authProvider: OAuthProviderId,
): Promise<OAuthDeviceCodeResponse> {
  return invoke<OAuthDeviceCodeResponse>("auth_start_login", {
    authProvider,
  });
}

/// 轮询检查账号是否授权完成
export async function authPollForAccount(
  authProvider: OAuthProviderId,
  deviceCode: string,
): Promise<OAuthAccount | null> {
  return invoke<OAuthAccount | null>("auth_poll_for_account", {
    authProvider,
    deviceCode,
  });
}

/// 列出所有已认证的账号
export async function authListAccounts(
  authProvider: OAuthProviderId,
): Promise<OAuthAccount[]> {
  return invoke<OAuthAccount[]>("auth_list_accounts", {
    authProvider,
  });
}

/// 获取 OAuth 认证状态
export async function authGetStatus(
  authProvider: OAuthProviderId,
): Promise<OAuthAuthStatus> {
  return invoke<OAuthAuthStatus>("auth_get_status", {
    authProvider,
  });
}

/// 移除指定账号
export async function authRemoveAccount(
  authProvider: OAuthProviderId,
  accountId: string,
): Promise<void> {
  return invoke("auth_remove_account", {
    authProvider,
    accountId,
  });
}

/// 设置默认账号
export async function authSetDefaultAccount(
  authProvider: OAuthProviderId,
  accountId: string,
): Promise<void> {
  return invoke("auth_set_default_account", {
    authProvider,
    accountId,
  });
}

/// 登出（清除所有账号）
export async function authLogout(
  authProvider: OAuthProviderId,
): Promise<void> {
  return invoke("auth_logout", {
    authProvider,
  });
}

/// 兼容性别名
export type ManagedAuthProvider = OAuthProviderId;
export type ManagedAuthAccount = OAuthAccount;
export type ManagedAuthStatus = OAuthAuthStatus;
export type ManagedAuthDeviceCodeResponse = OAuthDeviceCodeResponse;

/// 兼容旧版 API
export const authApi = {
  authListProviders,
  authStartLogin,
  authPollForAccount,
  authListAccounts,
  authGetStatus,
  authRemoveAccount,
  authSetDefaultAccount,
  authLogout,
  // 兼容旧版
  authStartLogin: authStartLogin,
  authPollForAccount: authPollForAccount,
  authListAccounts: authListAccounts,
  authGetStatus: authGetStatus,
  authRemoveAccount: authRemoveAccount,
  authSetDefaultAccount: authSetDefaultAccount,
  authLogout: authLogout,
};
```

#### src/components/providers/forms/hooks/useOAuthAuth.ts

```typescript
import { useState, useCallback, useRef, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { authApi, settingsApi } from "@/lib/api";
import { copyText } from "@/lib/clipboard";
import type {
  OAuthAccount,
  OAuthAuthStatus,
  OAuthDeviceCodeResponse,
  OAuthProviderId,
} from "@/lib/api";

type PollingState = "idle" | "polling" | "success" | "error";

export interface UseOAuthAuthOptions {
  /** OAuth 提供商 ID */
  provider: OAuthProviderId;
  /** 设备码轮询间隔加成（秒），避免 slow_down */
  pollingIntervalBuffer?: number;
  /** 最小轮询间隔（秒） */
  minPollingInterval?: number;
}

export interface UseOAuthAuthReturn {
  /** 认证状态 */
  authStatus: OAuthAuthStatus | undefined;
  /** 是否正在加载状态 */
  isLoadingStatus: boolean;
  /** 所有已认证账号 */
  accounts: OAuthAccount[];
  /** 是否有任意账号 */
  hasAnyAccount: boolean;
  /** 是否已认证 */
  isAuthenticated: boolean;
  /** 默认账号 ID */
  defaultAccountId: string | null;
  /** 迁移错误 */
  migrationError: string | null;
  /** 轮询状态 */
  pollingState: PollingState;
  /** 设备码响应 */
  deviceCode: OAuthDeviceCodeResponse | null;
  /** 错误信息 */
  error: string | null;
  /** 是否正在轮询 */
  isPolling: boolean;
  /** 是否正在添加账号 */
  isAddingAccount: boolean;
  /** 是否正在移除账号 */
  isRemovingAccount: boolean;
  /** 是否正在设置默认账号 */
  isSettingDefaultAccount: boolean;
  /** 开始认证 */
  startAuth: () => void;
  /** 添加账号（别名） */
  addAccount: () => void;
  /** 取消认证 */
  cancelAuth: () => void;
  /** 登出（清除所有账号） */
  logout: () => void;
  /** 移除指定账号 */
  removeAccount: (accountId: string) => void;
  /** 设置默认账号 */
  setDefaultAccount: (accountId: string) => void;
  /** 刷新状态 */
  refetchStatus: () => void;
}

export function useOAuthAuth(options: UseOAuthAuthOptions): UseOAuthAuthReturn {
  const { provider, pollingIntervalBuffer = 3, minPollingInterval = 8 } = options;
  const queryClient = useQueryClient();
  const queryKey = ["oauth-auth-status", provider];

  const [pollingState, setPollingState] = useState<PollingState>("idle");
  const [deviceCode, setDeviceCode] = useState<OAuthDeviceCodeResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pollingIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const pollingTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const {
    data: authStatus,
    isLoading: isLoadingStatus,
    refetch: refetchStatus,
  } = useQuery<OAuthAuthStatus>({
    queryKey,
    queryFn: () => authApi.authGetStatus(provider),
    staleTime: 30000,
  });

  const stopPolling = useCallback(() => {
    if (pollingIntervalRef.current) {
      clearInterval(pollingIntervalRef.current);
      pollingIntervalRef.current = null;
    }
    if (pollingTimeoutRef.current) {
      clearTimeout(pollingTimeoutRef.current);
      pollingTimeoutRef.current = null;
    }
  }, []);

  useEffect(() => {
    return () => {
      stopPolling();
    };
  }, [stopPolling]);

  const startLoginMutation = useMutation({
    mutationFn: () => authApi.authStartLogin(provider),
    onSuccess: async (response) => {
      setDeviceCode(response);
      setPollingState("polling");
      setError(null);

      // 复制用户码
      try {
        await copyText(response.user_code);
      } catch (e) {
        console.debug("[OAuthAuth] Failed to copy user code:", e);
      }

      // 打开验证 URL
      try {
        await settingsApi.openExternal(response.verification_uri);
      } catch (e) {
        console.debug("[OAuthAuth] Failed to open browser:", e);
      }

      // 计算轮询间隔
      const interval = Math.max((response.interval || 5) + pollingIntervalBuffer, minPollingInterval) * 1000;
      const expiresAt = Date.now() + response.expires_in * 1000;

      const pollOnce = async () => {
        if (Date.now() > expiresAt) {
          stopPolling();
          setPollingState("error");
          setError("Device code expired. Please try again.");
          return;
        }

        try {
          const newAccount = await authApi.authPollForAccount(provider, response.device_code);
          if (newAccount) {
            stopPolling();
            setPollingState("success");
            await refetchStatus();
            await queryClient.invalidateQueries({ queryKey });
            setPollingState("idle");
            setDeviceCode(null);
          }
        } catch (e) {
          const errorMessage = e instanceof Error ? e.message : String(e);
          // authorization_pending 和 slow_down 是正常的轮询状态，不显示错误
          if (
            !errorMessage.includes("authorization_pending") &&
            !errorMessage.includes("slow_down")
          ) {
            stopPolling();
            setPollingState("error");
            setError(errorMessage);
          }
        }
      };

      void pollOnce();
      pollingIntervalRef.current = setInterval(pollOnce, interval);
      pollingTimeoutRef.current = setTimeout(() => {
        stopPolling();
        setPollingState("error");
        setError("Device code expired. Please try again.");
      }, response.expires_in * 1000);
    },
    onError: (e) => {
      setPollingState("error");
      setError(e instanceof Error ? e.message : String(e));
    },
  });

  const logoutMutation = useMutation({
    mutationFn: () => authApi.authLogout(provider),
    onSuccess: async () => {
      setPollingState("idle");
      setDeviceCode(null);
      setError(null);
      queryClient.setQueryData(queryKey, {
        provider,
        authenticated: false,
        default_account_id: null,
        accounts: [],
      });
      await queryClient.invalidateQueries({ queryKey });
    },
    onError: async (e) => {
      console.error("[OAuthAuth] Failed to logout:", e);
      setError(e instanceof Error ? e.message : String(e));
      await refetchStatus();
    },
  });

  const removeAccountMutation = useMutation({
    mutationFn: (accountId: string) => authApi.authRemoveAccount(provider, accountId),
    onSuccess: async () => {
      setPollingState("idle");
      setDeviceCode(null);
      setError(null);
      await refetchStatus();
      await queryClient.invalidateQueries({ queryKey });
    },
    onError: (e) => {
      console.error("[OAuthAuth] Failed to remove account:", e);
      setError(e instanceof Error ? e.message : String(e));
    },
  });

  const setDefaultAccountMutation = useMutation({
    mutationFn: (accountId: string) => authApi.authSetDefaultAccount(provider, accountId),
    onSuccess: async () => {
      await refetchStatus();
      await queryClient.invalidateQueries({ queryKey });
    },
    onError: (e) => {
      console.error("[OAuthAuth] Failed to set default account:", e);
      setError(e instanceof Error ? e.message : String(e));
    },
  });

  const startAuth = useCallback(() => {
    setPollingState("idle");
    setDeviceCode(null);
    setError(null);
    stopPolling();
    startLoginMutation.mutate();
  }, [startLoginMutation, stopPolling]);

  const cancelAuth = useCallback(() => {
    stopPolling();
    setPollingState("idle");
    setDeviceCode(null);
    setError(null);
  }, [stopPolling]);

  const logout = useCallback(() => {
    logoutMutation.mutate();
  }, [logoutMutation]);

  const removeAccount = useCallback(
    (accountId: string) => {
      removeAccountMutation.mutate(accountId);
    },
    [removeAccountMutation],
  );

  const setDefaultAccount = useCallback(
    (accountId: string) => {
      setDefaultAccountMutation.mutate(accountId);
    },
    [setDefaultAccountMutation],
  );

  const accounts = authStatus?.accounts ?? [];

  return {
    authStatus,
    isLoadingStatus,
    accounts,
    hasAnyAccount: accounts.length > 0,
    isAuthenticated: authStatus?.authenticated ?? false,
    defaultAccountId: authStatus?.default_account_id ?? null,
    migrationError: authStatus?.migration_error ?? null,
    pollingState,
    deviceCode,
    error,
    isPolling: pollingState === "polling",
    isAddingAccount: startLoginMutation.isPending || pollingState === "polling",
    isRemovingAccount: removeAccountMutation.isPending,
    isSettingDefaultAccount: setDefaultAccountMutation.isPending,
    startAuth,
    addAccount: startAuth,
    cancelAuth,
    logout,
    removeAccount,
    setDefaultAccount,
    refetchStatus,
  };
}
```

#### src/components/providers/forms/OAuthAuthSection.tsx

```tsx
import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Loader2,
  Github,
  LogOut,
  Copy,
  Check,
  ExternalLink,
  Plus,
  X,
  User,
  Bot,
} from "lucide-react";
import { useOAuthAuth } from "./hooks/useOAuthAuth";
import { copyText } from "@/lib/clipboard";
import type { OAuthAccount, OAuthProviderId } from "@/lib/api";

interface OAuthProviderConfig {
  id: OAuthProviderId;
  name: string;
  icon: React.ReactNode;
  description?: string;
}

interface OAuthAuthSectionProps {
  className?: string;
  /** 当前选中的账号 ID */
  selectedAccountId?: string | null;
  /** 账号选择回调 */
  onAccountSelect?: (accountId: string | null) => void;
  /** OAuth 提供商配置 */
  providerConfig: OAuthProviderConfig;
}

const DEFAULT_PROVIDER_CONFIGS: Record<OAuthProviderId, OAuthProviderConfig> = {
  github_copilot: {
    id: "github_copilot",
    name: "GitHub Copilot",
    icon: <Github className="h-5 w-5" />,
  },
  openai: {
    id: "openai",
    name: "OpenAI",
    icon: <Bot className="h-5 w-5" />,
  },
  google_gemini: {
    id: "google_gemini",
    name: "Google Gemini",
    icon: <Bot className="h-5 w-5" />,
  },
  alibaba_qwen: {
    id: "alibaba_qwen",
    name: "通义千问",
    icon: <Bot className="h-5 w-5" />,
  },
  moonshot_kimi: {
    id: "moonshot_kimi",
    name: "Moonshot Kimi",
    icon: <Bot className="h-5 w-5" />,
  },
  minimax: {
    id: "minimax",
    name: "MiniMax",
    icon: <Bot className="h-5 w-5" />,
  },
  volcengine_ark: {
    id: "volcengine_ark",
    name: "火山引擎 Ark",
    icon: <Bot className="h-5 w-5" />,
  },
};

/**
 * 通用 OAuth 认证区块组件
 *
 * 支持所有 OAuth 提供商的认证状态显示和多账号管理。
 */
export const OAuthAuthSection: React.FC<OAuthAuthSectionProps> = ({
  className,
  selectedAccountId,
  onAccountSelect,
  providerConfig,
}) => {
  const { t } = useTranslation();
  const [copied, setCopied] = React.useState(false);

  const {
    accounts,
    defaultAccountId,
    migrationError,
    hasAnyAccount,
    pollingState,
    deviceCode,
    error,
    isPolling,
    isAddingAccount,
    isRemovingAccount,
    isSettingDefaultAccount,
    addAccount,
    removeAccount,
    setDefaultAccount,
    cancelAuth,
    logout,
  } = useOAuthAuth({ provider: providerConfig.id });

  // 复制用户码
  const copyUserCode = async () => {
    if (deviceCode?.user_code) {
      await copyText(deviceCode.user_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // 处理账号选择
  const handleAccountSelect = (value: string) => {
    onAccountSelect?.(value === "none" ? null : value);
  };

  // 处理移除账号
  const handleRemoveAccount = (accountId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    removeAccount(accountId);
    // 如果移除的是当前选中的账号，清除选择
    if (selectedAccountId === accountId) {
      onAccountSelect?.(null);
    }
  };

  // 渲染账号头像
  const renderAvatar = (account: OAuthAccount) => {
    return <OAuthAccountAvatar account={account} />;
  };

  return (
    <div className={`space-y-4 ${className || ""}`}>
      {/* 认证状态标题 */}
      <div className="flex items-center justify-between">
        <Label>{providerConfig.name} 认证</Label>
        <Badge
          variant={hasAnyAccount ? "default" : "secondary"}
          className={hasAnyAccount ? "bg-green-500 hover:bg-green-600" : ""}
        >
          {hasAnyAccount
            ? t("oauth.accountCount", {
                count: accounts.length,
                defaultValue: `${accounts.length} 个账号`,
              })
            : t("oauth.notAuthenticated", "未认证")}
        </Badge>
      </div>

      {migrationError && (
        <p className="text-sm text-amber-600 dark:text-amber-400">
          {t("oauth.migrationFailed", {
            error: migrationError,
            defaultValue: `迁移失败：${migrationError}`,
          })}
        </p>
      )}

      {/* 账号选择器（有账号时显示） */}
      {hasAnyAccount && onAccountSelect && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("oauth.selectAccount", "选择账号")}
          </Label>
          <Select
            value={selectedAccountId || "none"}
            onValueChange={handleAccountSelect}
          >
            <SelectTrigger>
              <SelectValue
                placeholder={t(
                  "oauth.selectAccountPlaceholder",
                  "选择一个账号",
                )}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">
                <span className="text-muted-foreground">
                  {t("oauth.useDefaultAccount", "使用默认账号")}
                </span>
              </SelectItem>
              {accounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  <div className="flex items-center gap-2">
                    {renderAvatar(account)}
                    <span>{account.login}</span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      {/* 已登录账号列表 */}
      {hasAnyAccount && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("oauth.loggedInAccounts", "已登录账号")}
          </Label>
          <div className="space-y-1">
            {accounts.map((account) => (
              <div
                key={account.id}
                className="flex items-center justify-between p-2 rounded-md border bg-muted/30"
              >
                <div className="flex items-center gap-2">
                  {renderAvatar(account)}
                  <span className="text-sm font-medium">{account.login}</span>
                  {defaultAccountId === account.id && (
                    <Badge variant="secondary" className="text-xs">
                      {t("oauth.defaultAccount", "默认")}
                    </Badge>
                  )}
                  {selectedAccountId === account.id && (
                    <Badge variant="outline" className="text-xs">
                      {t("oauth.selected", "已选中")}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  {defaultAccountId !== account.id && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs text-muted-foreground"
                      onClick={() => setDefaultAccount(account.id)}
                      disabled={isSettingDefaultAccount}
                    >
                      {t("oauth.setAsDefault", "设为默认")}
                    </Button>
                  )}
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-muted-foreground hover:text-red-500"
                    onClick={(e) => handleRemoveAccount(account.id, e)}
                    disabled={isRemovingAccount}
                    title={t("oauth.removeAccount", "移除账号")}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 未认证状态 - 登录按钮 */}
      {!hasAnyAccount && pollingState === "idle" && (
        <Button
          type="button"
          onClick={addAccount}
          className="w-full"
          variant="outline"
        >
          {providerConfig.icon}
          <span className="ml-2">
            {t("oauth.login", { name: providerConfig.name, defaultValue: `使用 ${providerConfig.name} 登录` })}
          </span>
        </Button>
      )}

      {/* 已有账号 - 添加更多账号按钮 */}
      {hasAnyAccount && pollingState === "idle" && (
        <Button
          type="button"
          onClick={addAccount}
          className="w-full"
          variant="outline"
          disabled={isAddingAccount}
        >
          <Plus className="mr-2 h-4 w-4" />
          {t("oauth.addAnotherAccount", "添加其他账号")}
        </Button>
      )}

      {/* 轮询中状态 */}
      {isPolling && deviceCode && (
        <div className="space-y-3 p-4 rounded-lg border border-border bg-muted/50">
          <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("oauth.waitingForAuth", "等待授权中...")}
          </div>

          {/* 用户码 */}
          <div className="text-center">
            <p className="text-xs text-muted-foreground mb-1">
              {t("oauth.enterCode", "在浏览器中输入以下代码：")}
            </p>
            <div className="flex items-center justify-center gap-2">
              <code className="text-2xl font-mono font-bold tracking-wider bg-background px-4 py-2 rounded border">
                {deviceCode.user_code}
              </code>
              <Button
                type="button"
                size="icon"
                variant="ghost"
                onClick={copyUserCode}
                title={t("oauth.copyCode", "复制代码")}
              >
                {copied ? (
                  <Check className="h-4 w-4 text-green-500" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>

          {/* 验证链接 */}
          <div className="text-center">
            <a
              href={deviceCode.verification_uri}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-sm text-blue-500 hover:underline"
            >
              {deviceCode.verification_uri}
              <ExternalLink className="h-3 w-3" />
            </a>
          </div>

          {/* 取消按钮 */}
          <div className="text-center">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={cancelAuth}
            >
              {t("common.cancel", "取消")}
            </Button>
          </div>
        </div>
      )}

      {/* 错误状态 */}
      {pollingState === "error" && error && (
        <div className="space-y-2">
          <p className="text-sm text-red-500">{error}</p>
          <div className="flex gap-2">
            <Button
              type="button"
              onClick={addAccount}
              variant="outline"
              size="sm"
            >
              {t("oauth.retry", "重试")}
            </Button>
            <Button
              type="button"
              onClick={cancelAuth}
              variant="ghost"
              size="sm"
            >
              {t("common.cancel", "取消")}
            </Button>
          </div>
        </div>
      )}

      {/* 注销所有账号按钮 */}
      {hasAnyAccount && accounts.length > 1 && (
        <Button
          type="button"
          variant="outline"
          onClick={logout}
          className="w-full text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-950"
        >
          <LogOut className="mr-2 h-4 w-4" />
          {t("oauth.logoutAll", "注销所有账号")}
        </Button>
      )}
    </div>
  );
};

const OAuthAccountAvatar: React.FC<{ account: OAuthAccount }> = ({
  account,
}) => {
  const [failed, setFailed] = React.useState(false);

  if (!account.avatar_url || failed) {
    return <User className="h-5 w-5 text-muted-foreground" />;
  }

  return (
    <img
      src={account.avatar_url}
      alt={account.login}
      className="h-5 w-5 rounded-full"
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
    />
  );
};

export default OAuthAuthSection;
```

#### src/components/settings/OAuthProviderSection.tsx

```tsx
import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Loader2,
  Bot,
  LogOut,
  Copy,
  Check,
  ExternalLink,
  Plus,
  X,
  User,
} from "lucide-react";
import { useOAuthAuth } from "@/components/providers/forms/hooks/useOAuthAuth";
import { copyText } from "@/lib/clipboard";
import type { OAuthAccount, OAuthProviderId } from "@/lib/api";

interface OAuthProviderSectionProps {
  providerId: OAuthProviderId;
  className?: string;
  /** 当前选中的账号 ID */
  selectedAccountId?: string | null;
  /** 账号选择回调 */
  onAccountSelect?: (accountId: string | null) => void;
}

// OAuth 提供商名称映射
const PROVIDER_NAMES: Record<OAuthProviderId, string> = {
  github_copilot: "GitHub",
  openai: "OpenAI",
  google_gemini: "Google Gemini",
  alibaba_qwen: "通义千问",
  moonshot_kimi: "Moonshot Kimi",
  minimax: "MiniMax",
  volcengine_ark: "火山引擎 Ark",
};

/**
 * 通用 OAuth 提供商认证区块
 *
 * 支持所有 OAuth 提供商的认证状态显示和多账号管理。
 */
export const OAuthProviderSection: React.FC<OAuthProviderSectionProps> = ({
  providerId,
  className,
  selectedAccountId,
  onAccountSelect,
}) => {
  const { t } = useTranslation();
  const [copied, setCopied] = React.useState(false);

  const {
    accounts,
    defaultAccountId,
    hasAnyAccount,
    pollingState,
    deviceCode,
    error,
    isPolling,
    isAddingAccount,
    isRemovingAccount,
    isSettingDefaultAccount,
    addAccount,
    removeAccount,
    setDefaultAccount,
    cancelAuth,
    logout,
  } = useOAuthAuth({ provider: providerId });

  const providerName = PROVIDER_NAMES[providerId] || providerId;

  // 复制用户码
  const copyUserCode = async () => {
    if (deviceCode?.user_code) {
      await copyText(deviceCode.user_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // 处理账号选择
  const handleAccountSelect = (value: string) => {
    onAccountSelect?.(value === "none" ? null : value);
  };

  // 处理移除账号
  const handleRemoveAccount = (accountId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    removeAccount(accountId);
    if (selectedAccountId === accountId) {
      onAccountSelect?.(null);
    }
  };

  return (
    <div className={`space-y-4 ${className || ""}`}>
      {/* 认证状态标题 */}
      <div className="flex items-center justify-between">
        <Label>{providerName} 认证</Label>
        <Badge
          variant={hasAnyAccount ? "default" : "secondary"}
          className={hasAnyAccount ? "bg-green-500 hover:bg-green-600" : ""}
        >
          {hasAnyAccount
            ? t("oauth.accountCount", {
                count: accounts.length,
                defaultValue: `${accounts.length} 个账号`,
              })
            : t("oauth.notAuthenticated", "未认证")}
        </Badge>
      </div>

      {/* 账号选择器（有账号时显示） */}
      {hasAnyAccount && onAccountSelect && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("oauth.selectAccount", "选择账号")}
          </Label>
          <Select
            value={selectedAccountId || "none"}
            onValueChange={handleAccountSelect}
          >
            <SelectTrigger>
              <SelectValue
                placeholder={t(
                  "oauth.selectAccountPlaceholder",
                  "选择一个账号",
                )}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">
                <span className="text-muted-foreground">
                  {t("oauth.useDefaultAccount", "使用默认账号")}
                </span>
              </SelectItem>
              {accounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  <div className="flex items-center gap-2">
                    <OAuthAccountAvatar account={account} />
                    <span>{account.login}</span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      {/* 已登录账号列表 */}
      {hasAnyAccount && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("oauth.loggedInAccounts", "已登录账号")}
          </Label>
          <div className="space-y-1">
            {accounts.map((account) => (
              <div
                key={account.id}
                className="flex items-center justify-between p-2 rounded-md border bg-muted/30"
              >
                <div className="flex items-center gap-2">
                  <OAuthAccountAvatar account={account} />
                  <span className="text-sm font-medium">{account.login}</span>
                  {defaultAccountId === account.id && (
                    <Badge variant="secondary" className="text-xs">
                      {t("oauth.defaultAccount", "默认")}
                    </Badge>
                  )}
                  {selectedAccountId === account.id && (
                    <Badge variant="outline" className="text-xs">
                      {t("oauth.selected", "已选中")}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  {defaultAccountId !== account.id && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs text-muted-foreground"
                      onClick={() => setDefaultAccount(account.id)}
                      disabled={isSettingDefaultAccount}
                    >
                      {t("oauth.setAsDefault", "设为默认")}
                    </Button>
                  )}
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-muted-foreground hover:text-red-500"
                    onClick={(e) => handleRemoveAccount(account.id, e)}
                    disabled={isRemovingAccount}
                    title={t("oauth.removeAccount", "移除账号")}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 未认证状态 - 登录按钮 */}
      {!hasAnyAccount && pollingState === "idle" && (
        <Button
          type="button"
          onClick={addAccount}
          className="w-full"
          variant="outline"
        >
          <Bot className="mr-2 h-4 w-4" />
          {t("oauth.login", { name: providerName, defaultValue: `使用 ${providerName} 登录` })}
        </Button>
      )}

      {/* 已有账号 - 添加更多账号按钮 */}
      {hasAnyAccount && pollingState === "idle" && (
        <Button
          type="button"
          onClick={addAccount}
          className="w-full"
          variant="outline"
          disabled={isAddingAccount}
        >
          <Plus className="mr-2 h-4 w-4" />
          {t("oauth.addAnotherAccount", "添加其他账号")}
        </Button>
      )}

      {/* 轮询中状态 */}
      {isPolling && deviceCode && (
        <div className="space-y-3 p-4 rounded-lg border border-border bg-muted/50">
          <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("oauth.waitingForAuth", "等待授权中...")}
          </div>

          {/* 用户码 */}
          <div className="text-center">
            <p className="text-xs text-muted-foreground mb-1">
              {t("oauth.enterCode", "在浏览器中输入以下代码：")}
            </p>
            <div className="flex items-center justify-center gap-2">
              <code className="text-2xl font-mono font-bold tracking-wider bg-background px-4 py-2 rounded border">
                {deviceCode.user_code}
              </code>
              <Button
                type="button"
                size="icon"
                variant="ghost"
                onClick={copyUserCode}
                title={t("oauth.copyCode", "复制代码")}
              >
                {copied ? (
                  <Check className="h-4 w-4 text-green-500" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>

          {/* 验证链接 */}
          <div className="text-center">
            <a
              href={deviceCode.verification_uri}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-sm text-blue-500 hover:underline"
            >
              {deviceCode.verification_uri}
              <ExternalLink className="h-3 w-3" />
            </a>
          </div>

          {/* 取消按钮 */}
          <div className="text-center">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={cancelAuth}
            >
              {t("common.cancel", "取消")}
            </Button>
          </div>
        </div>
      )}

      {/* 错误状态 */}
      {pollingState === "error" && error && (
        <div className="space-y-2">
          <p className="text-sm text-red-500">{error}</p>
          <div className="flex gap-2">
            <Button
              type="button"
              onClick={addAccount}
              variant="outline"
              size="sm"
            >
              {t("oauth.retry", "重试")}
            </Button>
            <Button
              type="button"
              onClick={cancelAuth}
              variant="ghost"
              size="sm"
            >
              {t("common.cancel", "取消")}
            </Button>
          </div>
        </div>
      )}

      {/* 注销所有账号按钮 */}
      {hasAnyAccount && accounts.length > 1 && (
        <Button
          type="button"
          variant="outline"
          onClick={logout}
          className="w-full text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-950"
        >
          <LogOut className="mr-2 h-4 w-4" />
          {t("oauth.logoutAll", "注销所有账号")}
        </Button>
      )}
    </div>
  );
};

const OAuthAccountAvatar: React.FC<{ account: OAuthAccount }> = ({
  account,
}) => {
  const [failed, setFailed] = React.useState(false);

  if (!account.avatar_url || failed) {
    return <User className="h-5 w-5 text-muted-foreground" />;
  }

  return (
    <img
      src={account.avatar_url}
      alt={account.login}
      className="h-5 w-5 rounded-full"
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
    />
  );
};

export default OAuthProviderSection;
```

#### src/components/settings/AuthCenterPanel.tsx

```tsx
import { Github, ShieldCheck, Bot, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { CopilotAuthSection } from "@/components/providers/forms/CopilotAuthSection";
import { OAuthProviderSection } from "@/components/settings/OAuthProviderSection";
import type { OAuthProviderId } from "@/lib/api";

// OAuth 提供商配置
const OAUTH_PROVIDERS: { id: OAuthProviderId; name: string; icon: React.ReactNode; description?: string }[] = [
  {
    id: "github_copilot",
    name: "GitHub Copilot",
    icon: <Github className="h-5 w-5" />,
    description: "管理 GitHub Copilot 账号",
  },
  {
    id: "openai",
    name: "OpenAI",
    icon: <Bot className="h-5 w-5" />,
    description: "OpenAI API OAuth 认证",
  },
  {
    id: "google_gemini",
    name: "Google Gemini",
    icon: <Bot className="h-5 w-5" />,
    description: "Google Gemini API OAuth 认证",
  },
  {
    id: "alibaba_qwen",
    name: "通义千问",
    icon: <Bot className="h-5 w-5" />,
    description: "阿里云通义千问 OAuth 认证",
  },
  {
    id: "moonshot_kimi",
    name: "Moonshot Kimi",
    icon: <Bot className="h-5 w-5" />,
    description: "Moonshot AI Kimi OAuth 认证",
  },
  {
    id: "minimax",
    name: "MiniMax",
    icon: <Bot className="h-5 w-5" />,
    description: "MiniMax API OAuth 认证",
  },
  {
    id: "volcengine_ark",
    name: "火山引擎 Ark",
    icon: <Bot className="h-5 w-5" />,
    description: "字节火山引擎 Ark OAuth 认证",
  },
];

export function AuthCenterPanel() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5 text-primary" />
              <h3 className="text-base font-semibold">
                {t("settings.authCenter.title", {
                  defaultValue: "OAuth 认证中心",
                })}
              </h3>
            </div>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.description", {
                defaultValue:
                  "集中管理跨应用复用的 OAuth 账号。Provider 只绑定这些认证源，不再重复登录。",
              })}
            </p>
          </div>
          <Badge variant="secondary">
            {t("settings.authCenter.beta", { defaultValue: "Beta" })}
          </Badge>
        </div>
      </section>

      {/* GitHub Copilot 单独展示 */}
      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="mb-4 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
            <Github className="h-5 w-5" />
          </div>
          <div>
            <h4 className="font-medium">GitHub Copilot</h4>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.copilotDescription", {
                defaultValue:
                  "管理 GitHub Copilot 账号、默认账号以及供 Claude / Codex / Gemini 绑定的托管凭据。",
              })}
            </p>
          </div>
        </div>

        <CopilotAuthSection />
      </section>

      {/* 其他 OAuth 提供商 */}
      {OAUTH_PROVIDERS.filter(p => p.id !== "github_copilot").map((provider) => (
        <section key={provider.id} className="rounded-xl border border-border/60 bg-card/60 p-6">
          <div className="mb-4 flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
              {provider.icon}
            </div>
            <div>
              <h4 className="font-medium">{provider.name}</h4>
              {provider.description && (
                <p className="text-sm text-muted-foreground">
                  {provider.description}
                </p>
              )}
            </div>
          </div>

          <OAuthProviderSection providerId={provider.id} />
        </section>
      ))}
    </div>
  );
}
```

### 3.6 lib.rs 集成

在 `lib.rs` 中初始化 CopilotAuthManager 和 OAuthManager（带 Copilot 集成）：

```rust
use crate::proxy::providers::oauth::OAuthManager;
use crate::proxy::providers::copilot_auth::CopilotAuthManager;
use crate::commands::auth::OAuthAuthState;

// 创建 CopilotAuthManager
let copilot_auth = CopilotAuthManager::new(app_config_dir.clone());

// 创建 OAuthManager（含 Copilot 集成）
let oauth_manager = OAuthManager::new_with_copilot(app_config_dir, copilot_auth);
app.manage(OAuthAuthState(Arc::new(oauth_manager)));
```

---

## 4. 遇到的问题和解决方案

### 4.1 OAuthProvider trait 不是 dyn compatible

**问题**：最初设计的 trait 有 async 方法，无法用于 `dyn OAuthProvider`。

**解决方案**：移除 trait，使用简化的 OAuthManager 设计，将流程处理分离到 OAuthProcessor。

### 4.2 模块路径错误 (unresolved import)

**问题**：`use super::provider` 失败。

**解决方案**：使用完整路径 `use crate::proxy::providers::oauth::provider::{...}`

### 4.3 auth 模块是 private

**问题**：`mod auth` 需要改为 `pub mod auth`。

**解决方案**：在 `commands/mod.rs` 中添加 `pub mod auth;`

### 4.4 storage_filename 返回 &str 但声明返回 String

**问题**：match 分支返回值类型不一致。

**解决方案**：在 match 分支中添加 `.to_string()`

### 4.5 serde borrow 错误

**问题**：在处理 user 信息时 move 了字段后又用于 raw json。

**解决方案**：将配置文件简化为只包含常量，不包含异步逻辑。

---

## 5. 最终结果

### 5.1 编译状态

- 编译通过 ✓
- 零警告 ✓（`processor.rs` 已删除，无孤立代码）

### 5.2 支持的 OAuth 厂商

| 厂商 | Provider ID | 配置文件 | 状态 |
|------|-------------|---------|------|
| GitHub Copilot | `github_copilot` | github_copilot.rs | 完整实现（复用 CopilotAuthManager） |
| OpenAI | `openai` | openai.rs | 完整实现，Client ID 待填 |
| Google Gemini | `google_gemini` | google_gemini.rs | 完整实现，Client ID 待填 |
| 阿里巴巴通义千问 | `alibaba_qwen` | alibaba_qwen.rs | 完整实现，Client ID 待填 |
| Moonshot (Kimi) | `moonshot_kimi` | moonshot.rs | 完整实现，Client ID 待填 |
| MiniMax | `minimax` | minimax.rs | 完整实现，Client ID 待填 |
| VolcEngine Ark | `volcengine_ark` | volcengine.rs | 完整实现，Client ID 待填 |

### 5.3 认证流程实现

#### 设备码启动（start_login）

- **GitHub Copilot**：调用 `CopilotAuthManager::start_device_flow()`，复用已有实现
- **其他厂商**：HTTP POST `device_code_url`，解析 `DeviceCodeResponse`

#### Token 轮询（poll_for_account）

- **GitHub Copilot**：调用 `CopilotAuthManager::poll_for_token()`，自动完成 GitHub token → Copilot token 交换
- **其他厂商**：HTTP POST `token_url`，处理 OAuth 错误码，调用 user_info API，保存账号

### 5.4 文件清单

**新建文件：**
- `src-tauri/src/proxy/providers/oauth/mod.rs`
- `src-tauri/src/proxy/providers/oauth/provider_id.rs`
- `src-tauri/src/proxy/providers/oauth/provider.rs`
- `src-tauri/src/proxy/providers/oauth/storage.rs`
- `src-tauri/src/proxy/providers/oauth/manager.rs`
- `src-tauri/src/proxy/providers/oauth/github_copilot.rs`
- `src-tauri/src/proxy/providers/oauth/openai.rs`
- `src-tauri/src/proxy/providers/oauth/google_gemini.rs`
- `src-tauri/src/proxy/providers/oauth/alibaba_qwen.rs`
- `src-tauri/src/proxy/providers/oauth/moonshot.rs`
- `src-tauri/src/proxy/providers/oauth/minimax.rs`
- `src-tauri/src/proxy/providers/oauth/volcengine.rs`
- `src/lib/api/auth.ts`
- `src/components/providers/forms/hooks/useOAuthAuth.ts`
- `src/components/providers/forms/OAuthAuthSection.tsx`
- `src/components/settings/OAuthProviderSection.tsx`
- `src/components/settings/AuthCenterPanel.tsx`

**修改文件：**
- `src-tauri/src/commands/auth.rs`
- `src-tauri/src/proxy/providers/mod.rs`
- `src-tauri/src/proxy/providers/auth.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`

**已删除文件：**
- `src-tauri/src/proxy/providers/oauth/processor.rs`（孤立文件，未被任何模块引用）

---

## 6. 待完成事项

1. **获取真实 Client ID** - 从各平台申请 OAuth 应用获取真实的 Client ID（目前各厂商均为占位符）
2. **实现 Token 刷新** - 为各提供商实现 access token 刷新逻辑（refresh_token 流程）
3. **集成测试** - 编写完整的 OAuth 流程测试
4. **API Base URL 配置** - 各厂商的 API_BASE_URL 常量已标记 `#[allow(dead_code)]`，实现代理层时启用

---

## 7. 参考资料

- [OAuth 2.0 Device Authorization Grant (RFC 8628)](https://datatracker.ietf.org/doc/html/rfc8628)
- [GitHub OAuth Apps](https://docs.github.com/en/developers/apps/authorizing-oauth-apps)
- [OpenAI OAuth](https://platform.openai.com/docs/api-reference/authentication)
- [Google OAuth 2.0 for Mobile & Desktop Apps](https://developers.google.com/identity/protocols/oauth2/native-app)
