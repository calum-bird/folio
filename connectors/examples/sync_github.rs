use std::path::PathBuf;

use anyhow::{Context, Result};
use foliofs_connectors::connector::{Connector, SyncOutput};
use foliofs_connectors::fs::replace_rendered_tree;
use foliofs_connectors::github::GitHubConnector;
use foliofs_connectors::model::{connection_pk, connection_sk, ConnectionRecord};

#[tokio::main]
async fn main() -> Result<()> {
    let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN is required")?;
    let login = std::env::var("GITHUB_LOGIN").unwrap_or_else(|_| "github".to_string());
    let output = std::env::var("FOLIO_EXAMPLE_OUTPUT").unwrap_or_else(|_| "tmp/folio".to_string());
    let owner = std::env::var("GITHUB_OWNER").ok();
    let connection = example_connection(&login);
    let mut connector = GitHubConnector::new(token);
    if let Some(owner) = owner {
        println!("filtering repositories to owner: {owner}");
        connector = connector.with_owner_filter(owner);
    }

    let plan = connector.plan(&connection).await?;
    println!(
        "plan: changed_count={} full_sync={} cursor={:?}",
        plan.changed_count, plan.full_sync, plan.cursor
    );

    let SyncOutput { files, cursor } = connector.sync(&connection).await?;
    replace_rendered_tree(&PathBuf::from(&output), &connection, &files).await?;

    println!(
        "wrote {} files to {}/{}, cursor={:?}",
        files.len(),
        output,
        connection.output_prefix,
        cursor
    );
    Ok(())
}

fn example_connection(login: &str) -> ConnectionRecord {
    let user_id = "local-user";
    let connection_id = "github-local";
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    ConnectionRecord {
        pk: connection_pk(user_id),
        sk: connection_sk("github", connection_id),
        gsi1pk: None,
        gsi1sk: None,
        entity_type: "connection".to_string(),
        user_id: user_id.to_string(),
        user_dir: user_id.to_string(),
        connection_id: connection_id.to_string(),
        provider: "github".to_string(),
        provider_account_id: login.to_string(),
        provider_account_login: login.to_string(),
        display_name: login.to_string(),
        scopes: vec!["repo".to_string(), "read:user".to_string()],
        status: "active".to_string(),
        secret_arn: "local".to_string(),
        output_prefix: format!("{user_id}/github"),
        sync_cursor: None,
        next_sync_at: now.clone(),
        last_sync_started_at: None,
        last_sync_finished_at: None,
        last_sync_error: None,
        sync_failure_count: 0,
        lease_owner: None,
        lease_expires_at: None,
        created_at: now.clone(),
        updated_at: now,
    }
}
