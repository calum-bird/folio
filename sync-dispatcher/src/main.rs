use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoDb;
use aws_sdk_sqs::Client as Sqs;
use foliofs_connectors::model::{
    sync_gsi_sk, ConnectionRecord, SyncJob, ACTIVE_SYNC_PK, CONNECTION_GSI,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct State {
    dynamodb: DynamoDb,
    sqs: Sqs,
    table_name: String,
    queue_url: String,
    interval_seconds: i64,
    batch_size: i32,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let state = State {
        dynamodb: DynamoDb::new(&config),
        sqs: Sqs::new(&config),
        table_name: env("FOLIO_CONNECTIONS_TABLE")?,
        queue_url: env("FOLIO_SYNC_QUEUE_URL")?,
        interval_seconds: env_i64("FOLIO_SYNC_INTERVAL_SECONDS", 3600),
        batch_size: env_i32("FOLIO_SYNC_DISPATCH_BATCH_SIZE", 100),
    };

    run(service_fn(move |event| handler(event, state.clone()))).await
}

async fn handler(_event: LambdaEvent<Value>, state: State) -> Result<Value, Error> {
    let now = chrono::Utc::now();
    let due = load_due_connections(&state, now).await?;
    let mut enqueued = 0usize;

    for connection in due {
        enqueue_sync(&state, &connection).await?;
        advance_next_sync(&state, &connection, now).await?;
        enqueued += 1;
    }

    tracing::info!(enqueued, "sync dispatcher complete");
    Ok(json!({ "enqueued": enqueued }))
}

async fn load_due_connections(state: &State, now: chrono::DateTime<chrono::Utc>) -> Result<Vec<ConnectionRecord>> {
    let response = state
        .dynamodb
        .query()
        .table_name(&state.table_name)
        .index_name(CONNECTION_GSI)
        .key_condition_expression("gsi1pk = :pk AND gsi1sk <= :due")
        .expression_attribute_values(":pk", AttributeValue::S(ACTIVE_SYNC_PK.to_string()))
        .expression_attribute_values(":due", AttributeValue::S(format!("{}#", iso(now))))
        .limit(state.batch_size)
        .send()
        .await
        .context("query due connections")?;

    let mut connections = Vec::new();
    for item in response.items() {
        connections.push(serde_dynamo::from_item(item.clone()).context("decode connection record")?);
    }

    Ok(connections)
}

async fn enqueue_sync(state: &State, connection: &ConnectionRecord) -> Result<()> {
    let job = SyncJob {
        user_id: connection.user_id.clone(),
        connection_id: connection.connection_id.clone(),
        provider: connection.provider.clone(),
    };
    state
        .sqs
        .send_message()
        .queue_url(&state.queue_url)
        .message_body(serde_json::to_string(&job).context("encode sync job")?)
        .send()
        .await
        .context("enqueue sync job")?;
    Ok(())
}

async fn advance_next_sync(
    state: &State,
    connection: &ConnectionRecord,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let next = now + chrono::Duration::seconds(state.interval_seconds);
    let next_sync_at = iso(next);
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
        .update_expression("SET nextSyncAt = :nextSyncAt, gsi1sk = :gsi1sk, updatedAt = :now")
        .expression_attribute_values(":nextSyncAt", AttributeValue::S(next_sync_at))
        .expression_attribute_values(":gsi1sk", AttributeValue::S(gsi1sk))
        .expression_attribute_values(":now", AttributeValue::S(iso(now)))
        .send()
        .await
        .context("advance next sync")?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,foliofs_sync_dispatcher=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).json().init();
}

fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is not configured"))
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_i32(name: &str, default: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn iso(value: chrono::DateTime<chrono::Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
