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

type LinearTokenResponse = {
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  token_type?: string;
  scope?: string;
  error?: string;
  error_description?: string;
};

type LinearViewerResponse = {
  data?: {
    viewer?: {
      id: string;
      name?: string;
      displayName?: string;
      email?: string;
    };
    organization?: {
      id: string;
      name: string;
      urlKey?: string;
    };
  };
  errors?: Array<{ message?: string }>;
};

type LinearWorkspace = {
  id: string;
  name: string;
  login: string;
  userName: string;
};

export function linearAuthorizeUrl(state: string, redirectUri: string) {
  const scope = process.env.LINEAR_OAUTH_SCOPES ?? "read";
  const url = new URL("https://linear.app/oauth/authorize");
  url.searchParams.set("client_id", linearClientId());
  url.searchParams.set("redirect_uri", redirectUri);
  url.searchParams.set("response_type", "code");
  url.searchParams.set("scope", scope);
  url.searchParams.set("state", state);
  return url;
}

export async function exchangeLinearCode(code: string, redirectUri: string) {
  const body = new URLSearchParams({
    client_id: linearClientId(),
    client_secret: linearClientSecret(),
    code,
    grant_type: "authorization_code",
    redirect_uri: redirectUri,
  });
  const response = await fetch("https://api.linear.app/oauth/token", {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/x-www-form-urlencoded",
    },
    body,
  });

  if (!response.ok) {
    throw new Error(`Linear token exchange failed: ${response.status}`);
  }

  const token = (await response.json()) as LinearTokenResponse;
  if (token.error) {
    throw new Error(token.error_description ?? token.error);
  }

  if (!token.access_token) {
    throw new Error("Linear token exchange did not return an access token");
  }

  return token;
}

export async function fetchLinearWorkspace(accessToken: string): Promise<LinearWorkspace> {
  const response = await fetch("https://api.linear.app/graphql", {
    method: "POST",
    headers: linearApiHeaders(accessToken),
    body: JSON.stringify({
      query: `
        query FolioLinearWorkspace {
          viewer {
            id
            name
            displayName
            email
          }
          organization {
            id
            name
            urlKey
          }
        }
      `,
    }),
  });

  if (!response.ok) {
    throw new Error(`Linear workspace lookup failed: ${response.status}`);
  }

  const payload = (await response.json()) as LinearViewerResponse;
  const error = payload.errors?.find((entry) => entry.message)?.message;
  if (error) {
    throw new Error(error);
  }

  const organization = payload.data?.organization;
  const viewer = payload.data?.viewer;
  if (!organization || !viewer) {
    throw new Error("Linear workspace lookup did not return an organization");
  }

  return {
    id: organization.id,
    name: organization.name,
    login: organization.urlKey ?? organization.name,
    userName: viewer.displayName ?? viewer.name ?? viewer.email ?? viewer.id,
  };
}

export function buildLinearConnection(
  userId: string,
  workspace: LinearWorkspace,
  token: LinearTokenResponse,
  secretArn: string,
): ConnectionRecord {
  const now = new Date().toISOString();
  const connectionId = `linear-${workspace.id}`;
  const nextSyncAt = nextSyncTime(new Date());

  return {
    pk: connectionPk(userId),
    sk: connectionSk("linear", connectionId),
    gsi1pk: ACTIVE_SYNC_PK,
    gsi1sk: syncGsiSk(nextSyncAt, userId, "linear", connectionId),
    entityType: "connection",
    userId,
    userDir: userDirFromSubject(userId),
    connectionId,
    provider: "linear",
    providerAccountId: workspace.id,
    providerAccountLogin: workspace.login,
    displayName: workspace.name,
    scopes: parseScopes(token.scope),
    status: "active",
    secretArn,
    outputPrefix: outputPrefixForConnection(userId, "linear"),
    nextSyncAt,
    createdAt: now,
    updatedAt: now,
  };
}

export function buildLinearSecret(
  workspace: LinearWorkspace,
  token: LinearTokenResponse,
): ProviderTokenSecret {
  return {
    provider: "linear",
    providerAccountId: workspace.id,
    providerAccountLogin: workspace.login,
    accessToken: token.access_token ?? "",
    refreshToken: token.refresh_token,
    tokenType: token.token_type,
    scopes: parseScopes(token.scope),
    expiresAt: token.expires_in ? new Date(Date.now() + token.expires_in * 1000).toISOString() : undefined,
    updatedAt: new Date().toISOString(),
  };
}

function linearApiHeaders(accessToken: string) {
  return {
    accept: "application/json",
    authorization: `Bearer ${accessToken}`,
    "content-type": "application/json",
    "user-agent": "FolioFS",
  };
}

function linearClientId() {
  const clientId = process.env.LINEAR_OAUTH_CLIENT_ID;
  if (!clientId) {
    throw new Error("LINEAR_OAUTH_CLIENT_ID is not configured");
  }

  return clientId;
}

function linearClientSecret() {
  const clientSecret = process.env.LINEAR_OAUTH_CLIENT_SECRET;
  if (!clientSecret) {
    throw new Error("LINEAR_OAUTH_CLIENT_SECRET is not configured");
  }

  return clientSecret;
}

function parseScopes(scope: string | undefined) {
  if (!scope) {
    return [];
  }

  return scope.split(/[,\s]+/).filter(Boolean);
}
