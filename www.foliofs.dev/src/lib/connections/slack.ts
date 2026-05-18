import "server-only";

import {
  ACTIVE_SYNC_PK,
  type ConnectionRecord,
  type ProviderTokenSecret,
  connectionPk,
  connectionSk,
  nextSyncTime,
  outputPrefixForConnection,
  syncGsiSk,
  userDirFromSubject,
} from "@/lib/connections/model";

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

type SlackWorkspace = {
  id: string;
  name: string;
  url?: string;
  user?: string;
};

export function slackAuthorizeUrl(state: string, redirectUri: string) {
  const scope = process.env.SLACK_OAUTH_SCOPES ?? "channels:read,channels:history,groups:read,groups:history,users:read,team:read";
  const url = new URL("https://slack.com/oauth/v2/authorize");
  url.searchParams.set("client_id", slackClientId());
  url.searchParams.set("redirect_uri", redirectUri);
  url.searchParams.set("scope", scope);
  url.searchParams.set("state", state);
  return url;
}

export async function exchangeSlackCode(code: string, redirectUri: string) {
  const body = new URLSearchParams({
    client_id: slackClientId(),
    client_secret: slackClientSecret(),
    code,
    redirect_uri: redirectUri,
  });
  const response = await fetch("https://slack.com/api/oauth.v2.access", {
    method: "POST",
    headers: {
      "content-type": "application/x-www-form-urlencoded",
    },
    body,
  });

  if (!response.ok) {
    throw new Error(`Slack token exchange failed: ${response.status}`);
  }

  const token = (await response.json()) as SlackTokenResponse;
  if (token.error || token.ok === false) {
    throw new Error(token.error ?? "Slack token exchange failed");
  }

  if (!token.access_token) {
    throw new Error("Slack token exchange did not return an access token");
  }

  return token;
}

export async function fetchSlackWorkspace(accessToken: string): Promise<SlackWorkspace> {
  const response = await fetch("https://slack.com/api/auth.test", {
    headers: slackApiHeaders(accessToken),
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
    name: authTest.team,
    url: authTest.url,
    user: authTest.user,
  };
}

export function buildSlackConnection(
  userId: string,
  workspace: SlackWorkspace,
  token: SlackTokenResponse,
  secretArn: string,
): ConnectionRecord {
  const now = new Date().toISOString();
  const connectionId = `slack-${workspace.id}`;
  const nextSyncAt = nextSyncTime(new Date());

  return {
    pk: connectionPk(userId),
    sk: connectionSk("slack", connectionId),
    gsi1pk: ACTIVE_SYNC_PK,
    gsi1sk: syncGsiSk(nextSyncAt, userId, "slack", connectionId),
    entityType: "connection",
    userId,
    userDir: userDirFromSubject(userId),
    connectionId,
    provider: "slack",
    providerAccountId: workspace.id,
    providerAccountLogin: workspace.name,
    displayName: workspace.name,
    scopes: parseScopes(token.scope),
    status: "active",
    secretArn,
    outputPrefix: outputPrefixForConnection(userId, "slack"),
    nextSyncAt,
    createdAt: now,
    updatedAt: now,
  };
}

export function buildSlackSecret(
  workspace: SlackWorkspace,
  token: SlackTokenResponse,
): ProviderTokenSecret {
  return {
    provider: "slack",
    providerAccountId: workspace.id,
    providerAccountLogin: workspace.name,
    accessToken: token.access_token ?? "",
    tokenType: token.token_type,
    scopes: parseScopes(token.scope),
    updatedAt: new Date().toISOString(),
  };
}

function slackApiHeaders(accessToken: string) {
  return {
    accept: "application/json",
    authorization: `Bearer ${accessToken}`,
    "user-agent": "FolioFS",
  };
}

function slackClientId() {
  const clientId = process.env.SLACK_OAUTH_CLIENT_ID;
  if (!clientId) {
    throw new Error("SLACK_OAUTH_CLIENT_ID is not configured");
  }

  return clientId;
}

function slackClientSecret() {
  const clientSecret = process.env.SLACK_OAUTH_CLIENT_SECRET;
  if (!clientSecret) {
    throw new Error("SLACK_OAUTH_CLIENT_SECRET is not configured");
  }

  return clientSecret;
}

function parseScopes(scope: string | undefined) {
  if (!scope) {
    return [];
  }

  return scope.split(/[,\s]+/).filter(Boolean);
}
