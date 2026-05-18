pub mod connector;
pub mod fs;
pub mod github;
pub mod linear;
pub mod model;
pub mod render;
pub mod slack;

use anyhow::Result;

use crate::connector::{connector_for_secret, Connector, SyncOutput};
use crate::model::{ConnectionRecord, ProviderTokenSecret, RenderedFile};

pub async fn sync_connection(
    connection: &ConnectionRecord,
    secret: &ProviderTokenSecret,
) -> Result<Vec<RenderedFile>> {
    let connector = connector_for_secret(secret)?;
    let SyncOutput { files, .. } = connector.sync(connection).await?;
    Ok(files)
}
