import "server-only";

import {
  ACTIVE_SYNC_PK,
  type ConnectionProvider,
  type ConnectionRecord,
  type ProviderTokenSecret,
  connectionPk,
  connectionSk,
  nextSyncTime,
  outputPrefixForConnection,
  syncGsiSk,
  userDirFromSubject,
} from "@/lib/connections/model";
import { githubConnector } from "@/lib/connections/providers/github";
import { linearConnector } from "@/lib/connections/providers/linear";
import { slackConnector } from "@/lib/connections/providers/slack";

export type OAuthAccount = {
  id: string;
  login: string;
  displayName: string;
};

export type OAuthTokenLike = {
  accessToken: string;
  refreshToken?: string;
  tokenType?: string;
  scope?: string;
  expiresInSeconds?: number;
};

export type ConnectorDefinition = {
  provider: ConnectionProvider;
  displayName: string;
  description: string;
  defaultScopes: string;
  scopeEnvVar: string;
  buildAuthorizeUrl(args: { state: string; redirectUri: string; scopes: string }): URL;
  exchangeCode(args: { code: string; redirectUri: string }): Promise<OAuthTokenLike>;
  fetchAccount(accessToken: string): Promise<OAuthAccount>;
};

export const CONNECTORS: ConnectorDefinition[] = [
  githubConnector,
  slackConnector,
  linearConnector,
];

export function getConnector(provider: string) {
  return CONNECTORS.find((connector) => connector.provider === provider);
}

export function scopesForConnector(connector: ConnectorDefinition) {
  return process.env[connector.scopeEnvVar] ?? connector.defaultScopes;
}

export function buildSecret(
  connector: ConnectorDefinition,
  account: OAuthAccount,
  token: OAuthTokenLike,
): ProviderTokenSecret {
  return {
    provider: connector.provider,
    providerAccountId: account.id,
    providerAccountLogin: account.login,
    accessToken: token.accessToken,
    refreshToken: token.refreshToken,
    tokenType: token.tokenType,
    scopes: parseScopes(token.scope),
    expiresAt: token.expiresInSeconds
      ? new Date(Date.now() + token.expiresInSeconds * 1000).toISOString()
      : undefined,
    updatedAt: new Date().toISOString(),
  };
}

export function buildConnection(
  connector: ConnectorDefinition,
  args: {
    userId: string;
    account: OAuthAccount;
    token: OAuthTokenLike;
    encryptedToken: string;
  },
): ConnectionRecord {
  const now = new Date().toISOString();
  const connectionId = connectionIdFor(connector, args.account);
  const nextSyncAt = nextSyncTime(new Date());

  return {
    pk: connectionPk(args.userId),
    sk: connectionSk(connector.provider, connectionId),
    gsi1pk: ACTIVE_SYNC_PK,
    gsi1sk: syncGsiSk(nextSyncAt, args.userId, connector.provider, connectionId),
    entityType: "connection",
    userId: args.userId,
    userDir: userDirFromSubject(args.userId),
    connectionId,
    provider: connector.provider,
    providerAccountId: args.account.id,
    providerAccountLogin: args.account.login,
    displayName: args.account.displayName,
    scopes: parseScopes(args.token.scope),
    status: "active",
    encryptedToken: args.encryptedToken,
    outputPrefix: outputPrefixForConnection(args.userId, connector.provider),
    nextSyncAt,
    createdAt: now,
    updatedAt: now,
  };
}

export function connectionIdFor(connector: ConnectorDefinition, account: OAuthAccount) {
  return `${connector.provider}-${account.id}`;
}

export function parseScopes(scope: string | undefined) {
  if (!scope) {
    return [];
  }

  return scope.split(/[,\s]+/).filter(Boolean);
}

export function requireEnv(name: string) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is not configured`);
  }

  return value;
}
