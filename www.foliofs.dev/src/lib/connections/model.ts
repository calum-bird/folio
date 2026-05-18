export const CONNECTION_GSI = "gsi1";
export const ACTIVE_SYNC_PK = "SYNC#ACTIVE";

export type ConnectionProvider = "github" | "slack" | "linear";

export type ConnectionStatus =
  | "active"
  | "disabled"
  | "syncing"
  | "failed"
  | "reconnect_required";

export type ConnectionRecord = {
  pk: string;
  sk: string;
  gsi1pk?: string;
  gsi1sk?: string;
  entityType: "connection";
  userId: string;
  userDir: string;
  connectionId: string;
  provider: ConnectionProvider;
  providerAccountId: string;
  providerAccountLogin: string;
  displayName: string;
  scopes: string[];
  status: ConnectionStatus;
  secretArn: string;
  outputPrefix: string;
  syncCursor?: string;
  nextSyncAt: string;
  lastSyncStartedAt?: string;
  lastSyncFinishedAt?: string;
  lastSyncError?: string;
  syncFailureCount?: number;
  leaseOwner?: string;
  leaseExpiresAt?: string;
  createdAt: string;
  updatedAt: string;
};

export type ProviderTokenSecret = {
  provider: ConnectionProvider;
  providerAccountId: string;
  providerAccountLogin: string;
  accessToken: string;
  refreshToken?: string;
  tokenType?: string;
  scopes: string[];
  expiresAt?: string;
  updatedAt: string;
};

export function connectionPk(userId: string) {
  return `USER#${userId}`;
}

export function connectionSk(provider: ConnectionProvider, connectionId: string) {
  return `CONNECTION#${provider}#${connectionId}`;
}

export function syncGsiSk(nextSyncAt: string, userId: string, provider: string, connectionId: string) {
  return `${nextSyncAt}#USER#${userId}#CONNECTION#${provider}#${connectionId}`;
}

export function userDirFromSubject(subject: string) {
  return subject
    .split("")
    .map((char) => {
      if (/^[A-Za-z0-9_-]$/.test(char)) {
        return char;
      }

      return "_";
    })
    .join("");
}

export function outputPrefixForConnection(
  userId: string,
  provider: ConnectionProvider,
) {
  return `${userDirFromSubject(userId)}/${provider}`;
}

export function secretNameForConnection(
  userId: string,
  provider: ConnectionProvider,
  connectionId: string,
) {
  return `${connectionSecretPrefix()}/${userDirFromSubject(userId)}/${provider}/${connectionId}`;
}

export function connectionTableName() {
  const tableName = process.env.FOLIO_CONNECTIONS_TABLE;
  if (!tableName) {
    throw new Error("FOLIO_CONNECTIONS_TABLE is not configured");
  }

  return tableName;
}

export function connectionSecretPrefix() {
  return process.env.FOLIO_CONNECTION_SECRET_PREFIX ?? "foliofs/connections";
}

export function syncIntervalSeconds() {
  const raw = Number(process.env.FOLIO_SYNC_INTERVAL_SECONDS ?? 3600);
  if (Number.isFinite(raw) && raw > 0) {
    return raw;
  }

  return 3600;
}

export function nextSyncTime(from = new Date()) {
  return new Date(from.getTime() + syncIntervalSeconds() * 1000).toISOString();
}
