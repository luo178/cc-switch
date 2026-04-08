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
