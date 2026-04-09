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
};
