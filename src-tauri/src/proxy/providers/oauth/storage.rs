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

    // ==================== 内部方法 ====================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_account_from_user_info() {
        let user_info = OAuthUserInfo {
            id: "12345".to_string(),
            login: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            raw: serde_json::json!({}),
        };

        let account = OAuthAccount::from_user_info(&user_info, "token123".to_string(), Some(1234567890));

        assert_eq!(account.id, "12345");
        assert_eq!(account.login, "testuser");
        assert_eq!(account.email, Some("test@example.com".to_string()));
        assert_eq!(account.access_token, "token123");
        assert_eq!(account.expires_at, Some(1234567890));
    }

    #[test]
    fn test_oauth_account_is_expiring_soon() {
        let now = chrono::Utc::now().timestamp();

        // 未过期
        let account = OAuthAccount {
            id: "12345".to_string(),
            login: "test".to_string(),
            email: None,
            avatar_url: None,
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: Some(now + 3600),
            authenticated_at: now,
        };
        assert!(!account.is_token_expiring_soon());

        // 即将过期（30秒后）
        let account = OAuthAccount {
            expires_at: Some(now + 30),
            ..account
        };
        assert!(account.is_token_expiring_soon());

        // 无过期时间
        let account = OAuthAccount {
            expires_at: None,
            ..account
        };
        assert!(!account.is_token_expiring_soon());
    }

    #[test]
    fn test_oauth_storage_state_default() {
        let state = OAuthStorageState::default();
        assert_eq!(state.version, 1);
        assert!(state.accounts.is_empty());
        assert!(state.default_account_id.is_none());
    }

    #[test]
    fn test_oauth_account_update() {
        let update = OAuthAccountUpdate::new()
            .with_access_token("new_token".to_string())
            .with_refresh_token("refresh_token".to_string())
            .with_expires_at(1234567890);

        assert_eq!(update.access_token, Some("new_token".to_string()));
        assert_eq!(update.refresh_token, Some("refresh_token".to_string()));
        assert_eq!(update.expires_at, Some(1234567890));
    }
}
