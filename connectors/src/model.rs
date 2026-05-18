use serde::{Deserialize, Serialize};

pub const ACTIVE_SYNC_PK: &str = "SYNC#ACTIVE";
pub const CONNECTION_GSI: &str = "gsi1";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionRecord {
    pub pk: String,
    pub sk: String,
    #[serde(default)]
    pub gsi1pk: Option<String>,
    #[serde(default)]
    pub gsi1sk: Option<String>,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "userDir")]
    pub user_dir: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    pub provider: String,
    #[serde(rename = "providerAccountId")]
    pub provider_account_id: String,
    #[serde(rename = "providerAccountLogin")]
    pub provider_account_login: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub scopes: Vec<String>,
    pub status: String,
    #[serde(rename = "secretArn")]
    pub secret_arn: String,
    #[serde(rename = "outputPrefix")]
    pub output_prefix: String,
    #[serde(default, rename = "syncCursor")]
    pub sync_cursor: Option<String>,
    #[serde(rename = "nextSyncAt")]
    pub next_sync_at: String,
    #[serde(default, rename = "lastSyncStartedAt")]
    pub last_sync_started_at: Option<String>,
    #[serde(default, rename = "lastSyncFinishedAt")]
    pub last_sync_finished_at: Option<String>,
    #[serde(default, rename = "lastSyncError")]
    pub last_sync_error: Option<String>,
    #[serde(default, rename = "syncFailureCount")]
    pub sync_failure_count: u32,
    #[serde(default, rename = "leaseOwner")]
    pub lease_owner: Option<String>,
    #[serde(default, rename = "leaseExpiresAt")]
    pub lease_expires_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderTokenSecret {
    pub provider: String,
    #[serde(rename = "providerAccountId")]
    pub provider_account_id: String,
    #[serde(rename = "providerAccountLogin")]
    pub provider_account_login: String,
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(default, rename = "refreshToken")]
    pub refresh_token: Option<String>,
    #[serde(default, rename = "tokenType")]
    pub token_type: Option<String>,
    pub scopes: Vec<String>,
    #[serde(default, rename = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncJob {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct RenderedFile {
    pub relative_path: String,
    pub contents: String,
}

pub fn connection_pk(user_id: &str) -> String {
    format!("USER#{user_id}")
}

pub fn connection_sk(provider: &str, connection_id: &str) -> String {
    format!("CONNECTION#{provider}#{connection_id}")
}

pub fn sync_gsi_sk(next_sync_at: &str, user_id: &str, provider: &str, connection_id: &str) -> String {
    format!("{next_sync_at}#USER#{user_id}#CONNECTION#{provider}#{connection_id}")
}
