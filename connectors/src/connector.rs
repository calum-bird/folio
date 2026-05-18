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

pub enum ConnectorKind {
    GitHub(crate::github::GitHubConnector),
    Slack(crate::slack::SlackConnector),
    Linear(crate::linear::LinearConnector),
}

impl Connector for ConnectorKind {
    fn plan<'a>(
        &'a self,
        connection: &'a ConnectionRecord,
    ) -> impl Future<Output = Result<SyncPlan>> + Send + 'a {
        async move {
            match self {
                Self::GitHub(connector) => connector.plan(connection).await,
                Self::Slack(connector) => connector.plan(connection).await,
                Self::Linear(connector) => connector.plan(connection).await,
            }
        }
    }

    fn sync<'a>(
        &'a self,
        connection: &'a ConnectionRecord,
    ) -> impl Future<Output = Result<SyncOutput>> + Send + 'a {
        async move {
            match self {
                Self::GitHub(connector) => connector.sync(connection).await,
                Self::Slack(connector) => connector.sync(connection).await,
                Self::Linear(connector) => connector.sync(connection).await,
            }
        }
    }
}

pub fn connector_for_secret(secret: &ProviderTokenSecret) -> Result<ConnectorKind> {
    match secret.provider.as_str() {
        "github" => Ok(ConnectorKind::GitHub(crate::github::GitHubConnector::new(
            secret.access_token.clone(),
        ))),
        "slack" => Ok(ConnectorKind::Slack(crate::slack::SlackConnector::new(
            secret.access_token.clone(),
        ))),
        "linear" => Ok(ConnectorKind::Linear(crate::linear::LinearConnector::new(
            secret.access_token.clone(),
        ))),
        provider => anyhow::bail!("unsupported connector provider: {provider}"),
    }
}
