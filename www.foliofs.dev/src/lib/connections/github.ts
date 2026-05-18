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

export function githubAuthorizeUrl(state: string, redirectUri: string) {
  const clientId = githubClientId();
  const scope = process.env.GITHUB_OAUTH_SCOPES ?? "read:user user:email repo";
  const url = new URL("https://github.com/login/oauth/authorize");
  url.searchParams.set("client_id", clientId);
  url.searchParams.set("redirect_uri", redirectUri);
  url.searchParams.set("scope", scope);
  url.searchParams.set("state", state);
  return url;
}

export async function exchangeGitHubCode(code: string, redirectUri: string) {
  const response = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json",
    },
    body: JSON.stringify({
      client_id: githubClientId(),
      client_secret: githubClientSecret(),
      code,
      redirect_uri: redirectUri,
    }),
  });

  if (!response.ok) {
    throw new Error(`GitHub token exchange failed: ${response.status}`);
  }

  const token = (await response.json()) as GitHubTokenResponse;
  if (token.error) {
    throw new Error(token.error_description ?? token.error);
  }

  if (!token.access_token) {
    throw new Error("GitHub token exchange did not return an access token");
  }

  return token;
}

export async function fetchGitHubUser(accessToken: string) {
  const response = await fetch("https://api.github.com/user", {
    headers: githubApiHeaders(accessToken),
  });

  if (!response.ok) {
    throw new Error(`GitHub user lookup failed: ${response.status}`);
  }

  return (await response.json()) as GitHubUser;
}

export function buildGitHubConnection(
  userId: string,
  githubUser: GitHubUser,
  token: GitHubTokenResponse,
  secretArn: string,
): ConnectionRecord {
  const now = new Date().toISOString();
  const connectionId = `github-${githubUser.id}`;
  const nextSyncAt = nextSyncTime(new Date());

  return {
    pk: connectionPk(userId),
    sk: connectionSk("github", connectionId),
    gsi1pk: ACTIVE_SYNC_PK,
    gsi1sk: syncGsiSk(nextSyncAt, userId, "github", connectionId),
    entityType: "connection",
    userId,
    userDir: userDirFromSubject(userId),
    connectionId,
    provider: "github",
    providerAccountId: String(githubUser.id),
    providerAccountLogin: githubUser.login,
    displayName: githubUser.name ?? githubUser.login,
    scopes: parseScopes(token.scope),
    status: "active",
    secretArn,
    outputPrefix: outputPrefixForConnection(userId, "github"),
    nextSyncAt,
    createdAt: now,
    updatedAt: now,
  };
}

export function buildGitHubSecret(
  githubUser: GitHubUser,
  token: GitHubTokenResponse,
): ProviderTokenSecret {
  return {
    provider: "github",
    providerAccountId: String(githubUser.id),
    providerAccountLogin: githubUser.login,
    accessToken: token.access_token ?? "",
    refreshToken: token.refresh_token,
    tokenType: token.token_type,
    scopes: parseScopes(token.scope),
    expiresAt: token.expires_in ? new Date(Date.now() + token.expires_in * 1000).toISOString() : undefined,
    updatedAt: new Date().toISOString(),
  };
}

export function githubApiHeaders(accessToken: string) {
  return {
    accept: "application/vnd.github+json",
    authorization: `Bearer ${accessToken}`,
    "x-github-api-version": "2022-11-28",
    "user-agent": "FolioFS",
  };
}

function githubClientId() {
  const clientId = process.env.GITHUB_OAUTH_CLIENT_ID;
  if (!clientId) {
    throw new Error("GITHUB_OAUTH_CLIENT_ID is not configured");
  }

  return clientId;
}

function githubClientSecret() {
  const clientSecret = process.env.GITHUB_OAUTH_CLIENT_SECRET;
  if (!clientSecret) {
    throw new Error("GITHUB_OAUTH_CLIENT_SECRET is not configured");
  }

  return clientSecret;
}

function parseScopes(scope: string | undefined) {
  if (!scope) {
    return [];
  }

  return scope.split(/[,\s]+/).filter(Boolean);
}
