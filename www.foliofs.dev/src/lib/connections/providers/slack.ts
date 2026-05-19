import "server-only";

import type { ConnectorDefinition, OAuthAccount, OAuthTokenLike } from "@/lib/connections/registry";
import { requireEnv } from "@/lib/connections/registry";

type SlackTokenResponse = {
  ok?: boolean;
  access_token?: string;
  token_type?: string;
  scope?: string;
  team?: {
    id?: string;
    name?: string;
  };
  error?: string;
};

type SlackAuthTestResponse = {
  ok?: boolean;
  url?: string;
  team?: string;
  user?: string;
  team_id?: string;
  user_id?: string;
  error?: string;
};

export const slackConnector: ConnectorDefinition = {
  provider: "slack",
  displayName: "Slack",
  description: "Sync channels and recent messages into markdown files in your folio.",
  defaultScopes:
    "channels:read,channels:history,groups:read,groups:history,users:read,team:read",
  scopeEnvVar: "SLACK_OAUTH_SCOPES",

  buildAuthorizeUrl({ state, redirectUri, scopes }) {
    const url = new URL("https://slack.com/oauth/v2/authorize");
    url.searchParams.set("client_id", requireEnv("SLACK_OAUTH_CLIENT_ID"));
    url.searchParams.set("redirect_uri", redirectUri);
    url.searchParams.set("scope", scopes);
    url.searchParams.set("state", state);
    return url;
  },

  async exchangeCode({ code, redirectUri }): Promise<OAuthTokenLike> {
    const body = new URLSearchParams({
      client_id: requireEnv("SLACK_OAUTH_CLIENT_ID"),
      client_secret: requireEnv("SLACK_OAUTH_CLIENT_SECRET"),
      code,
      redirect_uri: redirectUri,
    });
    const response = await fetch("https://slack.com/api/oauth.v2.access", {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body,
    });

    if (!response.ok) {
      throw new Error(`Slack token exchange failed: ${response.status}`);
    }

    const payload = (await response.json()) as SlackTokenResponse;
    if (payload.error || payload.ok === false) {
      throw new Error(payload.error ?? "Slack token exchange failed");
    }
    if (!payload.access_token) {
      throw new Error("Slack token exchange did not return an access token");
    }

    return {
      accessToken: payload.access_token,
      tokenType: payload.token_type,
      scope: payload.scope,
    };
  },

  async fetchAccount(accessToken): Promise<OAuthAccount> {
    const response = await fetch("https://slack.com/api/auth.test", {
      headers: {
        accept: "application/json",
        authorization: `Bearer ${accessToken}`,
        "user-agent": "FolioFS",
      },
    });

    if (!response.ok) {
      throw new Error(`Slack workspace lookup failed: ${response.status}`);
    }

    const authTest = (await response.json()) as SlackAuthTestResponse;
    if (authTest.error || authTest.ok === false) {
      throw new Error(authTest.error ?? "Slack workspace lookup failed");
    }
    if (!authTest.team_id || !authTest.team) {
      throw new Error("Slack workspace lookup did not return a team");
    }

    return {
      id: authTest.team_id,
      login: authTest.team,
      displayName: authTest.team,
    };
  },
};
