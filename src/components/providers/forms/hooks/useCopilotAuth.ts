import type { GitHubAccount } from "@/lib/api";
import { useManagedAuth } from "./useManagedAuth";

export function useCopilotAuth(githubDomain?: string) {
  const managedAuth = useManagedAuth("github_copilot", githubDomain);
  // 顶层 accounts 同步补全 github_domain 和 avatar_url
  const accounts = managedAuth.accounts.map((a) => ({
    ...a,
    github_domain: a.github_domain ?? "github.com",
    avatar_url: a.avatar_url ?? null,
  })) as unknown as GitHubAccount[];
  const defaultAccount =
    accounts.find(
      (account) => account.id === managedAuth.defaultAccountId,
    ) ?? accounts[0];

  return {
    ...managedAuth,
    accounts,
    authStatus: managedAuth.authStatus
      ? {
          authenticated: managedAuth.authStatus.authenticated,
          username: defaultAccount?.login ?? null,
          // Managed auth status does not expose a single provider-wide token expiry.
          expires_at: null,
          default_account_id: managedAuth.defaultAccountId,
          migration_error: managedAuth.migrationError,
          accounts,
        }
      : undefined,
    // Managed auth status no longer exposes a single default token expiry.
    username: defaultAccount?.login ?? null,
  };
}
