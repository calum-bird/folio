import "server-only";

import type { ConnectorDefinition, OAuthAccount, OAuthTokenLike } from "@/lib/connections/registry";
import { requireEnv } from "@/lib/connections/registry";

type GitHubTokenResponse = {
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  token_type?: string;
  scope?: string;
  error?: string;
  error_description?: string;
};

type GitHubUser = {
  id: number;
  login: string;
  name: string | null;
};

export const githubConnector: ConnectorDefinition = {
  provider: "github",
  displayName: "GitHub",
  description: "Sync repositories and issues into markdown files in your folio.",
  defaultScopes: "read:user user:email repo",
  scopeEnvVar: "GITHUB_OAUTH_SCOPES",

  buildAuthorizeUrl({ state, redirectUri, scopes }) {
    const url = new URL("https://github.com/login/oauth/authorize");
    url.searchParams.set("client_id", requireEnv("GITHUB_OAUTH_CLIENT_ID"));
    url.searchParams.set("redirect_uri", redirectUri);
    url.searchParams.set("scope", scopes);
    url.searchParams.set("state", state);
    return url;
  },

  async exchangeCode({ code, redirectUri }): Promise<OAuthTokenLike> {
    const response = await fetch("https://github.com/login/oauth/access_token", {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
      },
      body: JSON.stringify({
        client_id: requireEnv("GITHUB_OAUTH_CLIENT_ID"),
        client_secret: requireEnv("GITHUB_OAUTH_CLIENT_SECRET"),
        code,
        redirect_uri: redirectUri,
      }),
    });

    if (!response.ok) {
      throw new Error(`GitHub token exchange failed: ${response.status}`);
    }

    const payload = (await response.json()) as GitHubTokenResponse;
    if (payload.error) {
      throw new Error(payload.error_description ?? payload.error);
    }
    if (!payload.access_token) {
      throw new Error("GitHub token exchange did not return an access token");
    }

    return {
      accessToken: payload.access_token,
      refreshToken: payload.refresh_token,
      tokenType: payload.token_type,
      scope: payload.scope,
      expiresInSeconds: payload.expires_in,
    };
  },

  async fetchAccount(accessToken): Promise<OAuthAccount> {
    const response = await fetch("https://api.github.com/user", {
      headers: {
        accept: "application/vnd.github+json",
        authorization: `Bearer ${accessToken}`,
        "x-github-api-version": "2022-11-28",
        "user-agent": "FolioFS",
      },
    });

    if (!response.ok) {
      throw new Error(`GitHub user lookup failed: ${response.status}`);
    }

    const user = (await response.json()) as GitHubUser;
    return {
      id: String(user.id),
      login: user.login,
      displayName: user.name ?? user.login,
    };
  },
};
