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
  AlertCircle,
  Settings,
  Info,
} from "lucide-react";
import { useOAuthAuth } from "@/components/providers/forms/hooks/useOAuthAuth";
import { copyText } from "@/lib/clipboard";
import { authApi } from "@/lib/api/auth";
import type { OAuthAccount, OAuthProviderId } from "@/lib/api/auth";

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

// 每个 Provider 的环境变量名称
const PROVIDER_ENV_VARS: Record<OAuthProviderId, string> = {
  github_copilot: "CCSWITCH_GITHUB_CLIENT_ID",
  openai: "CCSWITCH_OPENAI_CLIENT_ID",
  google_gemini: "CCSWITCH_GOOGLE_CLIENT_ID",
  alibaba_qwen: "CCSWITCH_ALIBABA_CLIENT_ID",
  moonshot_kimi: "CCSWITCH_MOONSHOT_CLIENT_ID",
  minimax: "CCSWITCH_MINIMAX_CLIENT_ID",
  volcengine_ark: "CCSWITCH_VOLCENGINE_CLIENT_ID",
};

// 检测是否是配置相关的错误
function isConfigError(error: string | null): boolean {
  if (!error) return false;
  return error.includes("missing client ID") ||
         error.includes("not configured") ||
         error.includes("Client ID") ||
         error.includes("client ID");
}

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
  const [configuredClientIds, setConfiguredClientIds] = React.useState<Record<string, string>>({});
  const [isConfiguringClientId, setIsConfiguringClientId] = React.useState(false);
  const [clientIdInput, setClientIdInput] = React.useState("");

  // 加载已配置的 Client IDs
  React.useEffect(() => {
    authApi.authListClientIds().then(setConfiguredClientIds).catch(console.error);
  }, []);

  const currentClientId = configuredClientIds[providerId] || "";
  const hasClientIdConfigured = !!currentClientId && !currentClientId.startsWith("YOUR_");

  // 保存 Client ID
  const handleSaveClientId = async () => {
    try {
      await authApi.authSaveClientId(providerId, clientIdInput.trim());
      setConfiguredClientIds(prev => ({ ...prev, [providerId]: clientIdInput.trim() }));
      setClientIdInput("");
      setIsConfiguringClientId(false);
    } catch (e) {
      console.error("Failed to save client ID:", e);
    }
  };

  // 移除 Client ID
  const handleRemoveClientId = async () => {
    try {
      await authApi.authRemoveClientId(providerId);
      setConfiguredClientIds(prev => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      });
    } catch (e) {
      console.error("Failed to remove client ID:", e);
    }
  };

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

      {/* Client ID 配置区域 */}
      {pollingState === "idle" && (
        <div className="space-y-2">
          {!isConfiguringClientId ? (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Info className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="text-xs text-muted-foreground">
                  {hasClientIdConfigured
                    ? `${t("oauth.clientIdConfigured", "Client ID")}: ${currentClientId.slice(0, 8)}...`
                    : t("oauth.clientIdNotConfigured", "未配置 Client ID")}
                </span>
              </div>
              <div className="flex gap-1">
                {hasClientIdConfigured && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="h-6 text-xs text-muted-foreground hover:text-red-500"
                    onClick={handleRemoveClientId}
                  >
                    <X className="h-3 w-3 mr-0.5" />
                    {t("oauth.removeClientId", "移除")}
                  </Button>
                )}
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-6 text-xs"
                  onClick={() => {
                    setClientIdInput(currentClientId);
                    setIsConfiguringClientId(true);
                  }}
                >
                  <Settings className="h-3 w-3 mr-1" />
                  {hasClientIdConfigured ? t("oauth.changeClientId", "修改") : t("oauth.configureClientId", "配置")}
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-2 p-3 rounded-lg border border-border bg-muted/30">
              <div className="flex items-center justify-between">
                <Label className="text-sm">{providerName} Client ID</Label>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-6 text-xs"
                  onClick={() => {
                    setIsConfiguringClientId(false);
                    setClientIdInput("");
                  }}
                >
                  <X className="h-3 w-3" />
                </Button>
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={clientIdInput}
                  onChange={(e) => setClientIdInput(e.target.value)}
                  placeholder={t("oauth.clientIdPlaceholder", "输入 Client ID")}
                  className="flex-1 h-8 px-3 text-sm rounded-md border border-input bg-background text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                />
                <Button
                  type="button"
                  size="sm"
                  onClick={handleSaveClientId}
                  disabled={!clientIdInput.trim()}
                >
                  {t("common.save", "保存")}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {t("oauth.clientIdHint", {
                  defaultValue: `环境变量: ${PROVIDER_ENV_VARS[providerId]}`,
                })}
              </p>
            </div>
          )}
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
          {isConfigError(error) ? (
            /* 配置错误 - 更友好的提示 */
            <div className="p-3 rounded-lg border border-amber-200 bg-amber-50 dark:bg-amber-950/30 dark:border-amber-800">
              <div className="flex items-start gap-2">
                <AlertCircle className="h-4 w-4 text-amber-600 dark:text-amber-400 mt-0.5 flex-shrink-0" />
                <div className="space-y-1.5">
                  <p className="text-sm font-medium text-amber-800 dark:text-amber-200">
                    {t("oauth.configRequired", "需要配置 Client ID")}
                  </p>
                  <p className="text-xs text-amber-700 dark:text-amber-300">
                    {t("oauth.configHint", {
                      defaultValue: `请设置环境变量 ${PROVIDER_ENV_VARS[providerId]} 获取 OAuth Client ID`,
                    })}
                  </p>
                  <div className="flex items-center gap-1.5 mt-2">
                    <code className="text-xs bg-amber-100 dark:bg-amber-900/50 px-2 py-1 rounded font-mono">
                      {PROVIDER_ENV_VARS[providerId]}
                    </code>
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      className="h-6 text-xs"
                      onClick={() => copyText(PROVIDER_ENV_VARS[providerId])}
                    >
                      <Copy className="h-3 w-3" />
                    </Button>
                  </div>
                  <div className="flex gap-1.5 mt-2">
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
              </div>
            </div>
          ) : (
            /* 其他错误 */
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
