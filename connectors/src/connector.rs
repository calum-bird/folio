use std::future::Future;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{ConnectionRecord, ProviderTokenSecret, RenderedFile};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncPlan {
    pub changed_count: usize,
    pub cursor: Option<String>,
    pub full_sync: bool,
}

pub trait Connector {
    fn plan<'a>(
        &'a self,
        connection: &'a ConnectionRecord,
    ) -> impl Future<Output = Result<SyncPlan>> + Send + 'a;

    fn sync<'a>(
        &'a self,
        connection: &'a ConnectionRecord,
    ) -> impl Future<Output = Result<SyncOutput>> + Send + 'a;
}

#[derive(Debug, Clone)]
pub struct SyncOutput {
    pub files: Vec<RenderedFile>,
    pub cursor: Option<String>,
}

pub fn connector_for_secret(secret: &ProviderTokenSecret) -> Result<impl Connector> {
    match secret.provider.as_str() {
        "github" => Ok(crate::github::GitHubConnector::new(secret.access_token.clone())),
        provider => anyhow::bail!("unsupported connector provider: {provider}"),
    }
}
