use anyhow::{Context, Result};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::connector::{Connector, SyncOutput, SyncPlan};
use crate::model::{ConnectionRecord, RenderedFile};
use crate::render::{slug, Renderer};

const INDEX_TEMPLATE: &str = include_str!("templates/index.md.j2");
const CHANNEL_TEMPLATE: &str = include_str!("templates/channel.md.j2");

#[derive(Clone)]
pub struct SlackConnector {
    access_token: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct SlackChannelsResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    channels: Vec<SlackChannel>,
    #[serde(default)]
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    messages: Vec<SlackMessage>,
}

#[derive(Debug, Deserialize)]
struct ResponseMetadata {
    #[serde(default)]
    next_cursor: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SlackChannel {
    id: String,
    name: String,
    #[serde(default)]
    is_channel: bool,
    #[serde(default)]
    is_private: bool,
    #[serde(default)]
    is_archived: bool,
    #[serde(default)]
    is_member: bool,
    #[serde(default)]
    topic: SlackTopic,
    #[serde(default)]
    purpose: SlackTopic,
    #[serde(default)]
    num_members: Option<u64>,
    #[serde(default)]
    messages: Vec<SlackMessage>,
    #[serde(skip_deserializing)]
    slug: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct SlackTopic {
    #[serde(default)]
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SlackMessage {
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    text: String,
    ts: String,
    #[serde(default)]
    thread_ts: Option<String>,
    #[serde(skip_deserializing)]
    author: String,
}

#[derive(Debug, Serialize)]
struct SlackIndex {
    account: String,
    synced_at: String,
    channels: Vec<SlackChannelSummary>,
}

#[derive(Debug, Serialize)]
struct SlackChannelSummary {
    id: String,
    name: String,
    slug: String,
}

impl SlackConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http: reqwest::Client::new(),
        }
    }

    async fn fetch_channels(&self) -> Result<Vec<SlackChannel>> {
        let mut channels = Vec::new();
        let mut cursor = String::new();

        for _ in 0..3 {
            let page = self.fetch_channel_page(&cursor).await?;
            let next_cursor = page
                .response_metadata
                .map(|metadata| metadata.next_cursor)
                .unwrap_or_default();
            channels.extend(page.channels.into_iter().filter(|channel| !channel.is_archived));

            if next_cursor.is_empty() {
                return Ok(channels);
            }

            cursor = next_cursor;
        }

        Ok(channels)
    }

    async fn fetch_channel_page(&self, cursor: &str) -> Result<SlackChannelsResponse> {
        let mut request = self.http.get("https://slack.com/api/conversations.list").headers(self.headers()?);
        request = request.query(&[
            ("types", "public_channel,private_channel"),
            ("exclude_archived", "true"),
            ("limit", "100"),
        ]);
        if !cursor.is_empty() {
            request = request.query(&[("cursor", cursor)]);
        }

        let page = request
            .send()
            .await
            .context("fetch Slack channels")?
            .json::<SlackChannelsResponse>()
            .await
            .context("decode Slack channels")?;
        ensure_slack_ok(page.ok, page.error.as_deref())?;
        Ok(page)
    }

    async fn fetch_messages(&self, channel_id: &str) -> Result<Vec<SlackMessage>> {
        let response = self
            .http
            .get("https://slack.com/api/conversations.history")
            .headers(self.headers()?)
            .query(&[("channel", channel_id), ("limit", "50")])
            .send()
            .await
            .with_context(|| format!("fetch Slack messages for {channel_id}"))?;
        let history = response
            .json::<SlackHistoryResponse>()
            .await
            .with_context(|| format!("decode Slack messages for {channel_id}"))?;
        ensure_slack_ok(history.ok, history.error.as_deref())?;
        Ok(history
            .messages
            .into_iter()
            .map(with_author)
            .collect())
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("FolioFS"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))
                .context("build Slack authorization header")?,
        );
        Ok(headers)
    }
}

impl Connector for SlackConnector {
    fn plan<'a>(
        &'a self,
        _connection: &'a ConnectionRecord,
    ) -> impl std::future::Future<Output = Result<SyncPlan>> + Send + 'a {
        async move {
            let channels = self.fetch_channels().await?;
            Ok(SyncPlan {
                changed_count: channels.len(),
                cursor: Some(now_isoish()),
                full_sync: true,
            })
        }
    }

    fn sync<'a>(
        &'a self,
        connection: &'a ConnectionRecord,
    ) -> impl std::future::Future<Output = Result<SyncOutput>> + Send + 'a {
        async move {
            let renderer = slack_renderer()?;
            let mut channels = self.fetch_channels().await?;
            let mut files = Vec::new();

            for channel in &mut channels {
                channel.slug = slug(&format!("{}-{}", channel.name, channel.id));
                channel.messages = self.fetch_messages(&channel.id).await?;
            }

            render_index(connection, &channels, &renderer, &mut files)?;
            render_channels(channels, &renderer, &mut files)?;

            Ok(SyncOutput {
                files,
                cursor: Some(now_isoish()),
            })
        }
    }
}

fn render_index(
    connection: &ConnectionRecord,
    channels: &[SlackChannel],
    renderer: &Renderer,
    files: &mut Vec<RenderedFile>,
) -> Result<()> {
    let index = SlackIndex {
        account: connection.provider_account_login.clone(),
        synced_at: now_isoish(),
        channels: channels
            .iter()
            .map(|channel| SlackChannelSummary {
                id: channel.id.clone(),
                name: channel.name.clone(),
                slug: channel.slug.clone(),
            })
            .collect(),
    };
    files.push(renderer.render("index.md", "index.md".to_string(), &index)?);
    Ok(())
}

fn render_channels(
    channels: Vec<SlackChannel>,
    renderer: &Renderer,
    files: &mut Vec<RenderedFile>,
) -> Result<()> {
    for channel in channels {
        files.push(renderer.render(
            "channel.md",
            format!("channels/{}.md", channel.slug),
            &channel,
        )?);
    }

    Ok(())
}

fn slack_renderer() -> Result<Renderer> {
    Renderer::new(&[
        ("index.md", INDEX_TEMPLATE),
        ("channel.md", CHANNEL_TEMPLATE),
    ])
}

fn ensure_slack_ok(ok: bool, error: Option<&str>) -> Result<()> {
    if ok {
        return Ok(());
    }

    anyhow::bail!("Slack request failed: {}", error.unwrap_or("unknown_error"));
}

fn with_author(mut message: SlackMessage) -> SlackMessage {
    message.author = message
        .user
        .clone()
        .or_else(|| message.username.clone())
        .unwrap_or_else(|| "unknown".to_string());
    message
}

fn now_isoish() -> String {
    chrono::Utc::now().to_rfc3339()
}
