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
