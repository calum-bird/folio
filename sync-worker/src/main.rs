use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_lambda_events::event::sqs::SqsEvent;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoDb;
use foliofs_connectors::connector::{connector_for_secret, Connector};
use foliofs_connectors::fs::replace_rendered_tree;
use foliofs_connectors::model::{
    connection_pk, connection_sk, sync_gsi_sk, ConnectionRecord, SyncJob, ACTIVE_SYNC_PK,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

mod token_cache;
mod tokens;

use tokens::TokenLoader;

const MAX_SYNC_FAILURES: u32 = 3;
const DEFAULT_TOKEN_CACHE_TTL_SECS: u64 = 300;

#[derive(Clone)]
struct State {
    dynamodb: DynamoDb,
    table_name: String,
    data_root: PathBuf,
    interval_seconds: i64,
    tokens: Arc<Mutex<TokenLoader>>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let state = State {
        dynamodb: DynamoDb::new(&config),
        table_name: env("FOLIO_CONNECTIONS_TABLE")?,
        data_root: PathBuf::from(env_default("FOLIO_DATA_ROOT", "/mnt/folio")),
        interval_seconds: env_i64("FOLIO_SYNC_INTERVAL_SECONDS", 3600),
        tokens: Arc::new(Mutex::new(TokenLoader::new(
            aws_sdk_kms::Client::new(&config),
            env_u64("FOLIO_TOKEN_CACHE_TTL_SECONDS", DEFAULT_TOKEN_CACHE_TTL_SECS),
        ))),
    };

    run(service_fn(move |event| handler(event, state.clone()))).await
}

async fn handler(event: LambdaEvent<SqsEvent>, state: State) -> Result<(), Error> {
    for record in event.payload.records {
        let Some(body) = record.body else {
            continue;
        };
        let job = serde_json::from_str::<SyncJob>(&body).context("decode SQS sync job")?;
        sync_job(&state, job).await?;
    }

    Ok(())
}

async fn sync_job(state: &State, job: SyncJob) -> Result<()> {
    let now = chrono::Utc::now();
    let connection = get_connection(state, &job).await?;
    let leased = match acquire_lease(state, &connection, now).await {
        Ok(connection) => connection,
        Err(error) if is_lease_conflict(&error) => {
            tracing::info!(
                provider = job.provider,
                connection_id = job.connection_id,
                "sync lease already held; acknowledging duplicate job"
            );
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let result = sync_leased_connection(state, &leased).await;

    match result {
        Ok(cursor) => mark_success(state, &leased, now, cursor).await,
        Err(error) => {
            let message = format!("{error:#}");
            mark_failure(state, &leased, now, &message).await?;
            Err(error)
        }
    }
}

async fn sync_leased_connection(state: &State, connection: &ConnectionRecord) -> Result<Option<String>> {
    let secret = state.tokens.lock().await.load(connection).await?;
    let connector = connector_for_secret(&secret)?;
    let plan = connector.plan(connection).await?;
    tracing::info!(
        provider = connection.provider,
        connection_id = connection.connection_id,
        changed_count = plan.changed_count,
        full_sync = plan.full_sync,
        "connector sync plan"
    );

    if plan.changed_count == 0 && !plan.full_sync {
        return Ok(plan.cursor);
    }

    let output = connector.sync(connection).await?;
    replace_rendered_tree(&state.data_root, connection, &output.files).await?;
    Ok(output.cursor.or(plan.cursor))
}

async fn get_connection(state: &State, job: &SyncJob) -> Result<ConnectionRecord> {
    let response = state
        .dynamodb
        .get_item()
        .table_name(&state.table_name)
        .key("pk", AttributeValue::S(connection_pk(&job.user_id)))
        .key("sk", AttributeValue::S(connection_sk(&job.provider, &job.connection_id)))
        .send()
        .await
        .context("get connection")?;

    let item = response.item().context("connection not found")?;
    serde_dynamo::from_item(item.clone()).context("decode connection record")
}

async fn acquire_lease(
    state: &State,
    connection: &ConnectionRecord,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<ConnectionRecord> {
    let owner = lambda_owner();
    let lease_expires_at = iso(now + chrono::Duration::minutes(15));
    let response = state
        .dynamodb
        .update_item()
        .table_name(&state.table_name)
        .key("pk", AttributeValue::S(connection.pk.clone()))
        .key("sk", AttributeValue::S(connection.sk.clone()))
        .condition_expression("attribute_not_exists(leaseExpiresAt) OR leaseExpiresAt < :now OR leaseOwner = :owner")
        .update_expression("SET #status = :status, leaseOwner = :owner, leaseExpiresAt = :leaseExpiresAt, lastSyncStartedAt = :now, updatedAt = :now")
        .expression_attribute_names("#status", "status")
        .expression_attribute_values(":status", AttributeValue::S("syncing".to_string()))
        .expression_attribute_values(":owner", AttributeValue::S(owner))
        .expression_attribute_values(":leaseExpiresAt", AttributeValue::S(lease_expires_at))
        .expression_attribute_values(":now", AttributeValue::S(iso(now)))
        .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew)
        .send()
        .await
        .context("acquire sync lease")?;

    let attributes = response.attributes().context("lease update returned no item")?;
    serde_dynamo::from_item(attributes.clone()).context("decode leased connection")
}

async fn mark_success(
    state: &State,
    connection: &ConnectionRecord,
    now: chrono::DateTime<chrono::Utc>,
    cursor: Option<String>,
) -> Result<()> {
    let next_sync_at = iso(now + chrono::Duration::seconds(state.interval_seconds));
    let gsi1sk = sync_gsi_sk(
        &next_sync_at,
        &connection.user_id,
        &connection.provider,
        &connection.connection_id,
    );
    let mut update = state
        .dynamodb
        .update_item()
        .table_name(&state.table_name)
        .key("pk", AttributeValue::S(connection.pk.clone()))
        .key("sk", AttributeValue::S(connection.sk.clone()))
        .update_expression("SET #status = :status, nextSyncAt = :nextSyncAt, gsi1pk = :gsi1pk, gsi1sk = :gsi1sk, lastSyncFinishedAt = :now, updatedAt = :now, syncCursor = :cursor, syncFailureCount = :zero REMOVE leaseOwner, leaseExpiresAt, lastSyncError")
        .expression_attribute_names("#status", "status")
        .expression_attribute_values(":status", AttributeValue::S("active".to_string()))
        .expression_attribute_values(":nextSyncAt", AttributeValue::S(next_sync_at))
        .expression_attribute_values(":gsi1pk", AttributeValue::S(ACTIVE_SYNC_PK.to_string()))
        .expression_attribute_values(":gsi1sk", AttributeValue::S(gsi1sk))
        .expression_attribute_values(":now", AttributeValue::S(iso(now)))
        .expression_attribute_values(":zero", AttributeValue::N("0".to_string()));

    update = update.expression_attribute_values(":cursor", AttributeValue::S(cursor.unwrap_or_else(|| "full".to_string())));
    update.send().await.context("mark sync success")?;
    Ok(())
}

async fn mark_failure(
    state: &State,
    connection: &ConnectionRecord,
    now: chrono::DateTime<chrono::Utc>,
    message: &str,
) -> Result<()> {
    let failure_count = connection.sync_failure_count.saturating_add(1);
    let status = failure_status(failure_count);
    let next_sync_at = iso(now + chrono::Duration::seconds(state.interval_seconds));
    let gsi1sk = sync_gsi_sk(
        &next_sync_at,
        &connection.user_id,
        &connection.provider,
        &connection.connection_id,
    );
    state
        .dynamodb
        .update_item()
        .table_name(&state.table_name)
        .key("pk", AttributeValue::S(connection.pk.clone()))
        .key("sk", AttributeValue::S(connection.sk.clone()))
        .update_expression("SET #status = :status, nextSyncAt = :nextSyncAt, gsi1pk = :gsi1pk, gsi1sk = :gsi1sk, lastSyncError = :message, lastSyncFinishedAt = :now, updatedAt = :now, syncFailureCount = :failureCount REMOVE leaseOwner, leaseExpiresAt")
        .expression_attribute_names("#status", "status")
        .expression_attribute_values(":status", AttributeValue::S(status.to_string()))
        .expression_attribute_values(":nextSyncAt", AttributeValue::S(next_sync_at))
        .expression_attribute_values(":gsi1pk", AttributeValue::S(ACTIVE_SYNC_PK.to_string()))
        .expression_attribute_values(":gsi1sk", AttributeValue::S(gsi1sk))
        .expression_attribute_values(":message", AttributeValue::S(message.chars().take(1000).collect()))
        .expression_attribute_values(":failureCount", AttributeValue::N(failure_count.to_string()))
        .expression_attribute_values(":now", AttributeValue::S(iso(now)))
        .send()
        .await
        .context("mark sync failure")?;
    Ok(())
}

fn failure_status(failure_count: u32) -> &'static str {
    if failure_count >= MAX_SYNC_FAILURES {
        return "reconnect_required";
    }

    "failed"
}

fn is_lease_conflict(error: &anyhow::Error) -> bool {
    format!("{error:#}").contains("ConditionalCheckFailedException")
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,foliofs_sync_worker=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).json().init();
}

fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is not configured"))
}

fn env_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn lambda_owner() -> String {
    std::env::var("AWS_LAMBDA_LOG_STREAM_NAME").unwrap_or_else(|_| "local".to_string())
}

fn iso(value: chrono::DateTime<chrono::Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
