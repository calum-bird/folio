use std::collections::HashMap;

use anyhow::{Context, Result};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::connector::{Connector, SyncOutput, SyncPlan};
use crate::model::{ConnectionRecord, RenderedFile};
use crate::render::{slug, Renderer};

const INDEX_TEMPLATE: &str = include_str!("templates/index.md.j2");
const ISSUE_TEMPLATE: &str = include_str!("templates/issue.md.j2");

#[derive(Clone)]
pub struct LinearConnector {
    access_token: String,
    http: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct GraphQlRequest<'a> {
    query: &'a str,
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct LinearSyncData {
    organization: LinearOrganization,
    teams: LinearConnection<LinearTeam>,
    issues: LinearConnection<LinearIssue>,
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct LinearConnection<T> {
    #[serde(default)]
    nodes: Vec<T>,
}

impl<T> Default for LinearConnection<T> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearOrganization {
    id: String,
    name: String,
    #[serde(default)]
    #[serde(rename = "urlKey")]
    url_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LinearTeam {
    id: String,
    key: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearIssue {
    id: String,
    identifier: String,
    title: String,
    url: String,
    priority: u8,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    state: LinearIssueState,
    team: LinearIssueTeam,
    #[serde(default)]
    assignee: Option<LinearUser>,
    #[serde(skip_deserializing)]
    assignee_name: Option<String>,
    #[serde(skip_deserializing)]
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearIssueState {
    name: String,
    #[serde(rename = "type")]
    state_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearIssueTeam {
    key: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearUser {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct LinearIndex {
    account: String,
    synced_at: String,
    total_issues: usize,
    teams: Vec<LinearTeamSummary>,
}

#[derive(Debug, Serialize)]
struct LinearTeamSummary {
    key: String,
    name: String,
    slug: String,
    issues: Vec<LinearIssueSummary>,
}

#[derive(Debug, Serialize)]
struct LinearIssueSummary {
    identifier: String,
    slug: String,
    title: String,
    state: String,
}

impl LinearConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http: reqwest::Client::new(),
        }
    }

    async fn fetch_data(&self) -> Result<LinearSyncData> {
        let mut data = self.graphql::<LinearSyncData>(SYNC_QUERY).await?;
        for issue in &mut data.issues.nodes {
            issue.slug = slug(&issue.identifier);
            issue.assignee_name = issue.assignee.as_ref().and_then(linear_user_name);
        }
        Ok(data)
    }

    async fn graphql<T>(&self, query: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .post("https://api.linear.app/graphql")
            .headers(self.headers()?)
            .json(&GraphQlRequest { query })
            .send()
            .await
            .context("call Linear GraphQL")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Linear request failed: {status} {body}");
        }

        let payload = response
            .json::<GraphQlResponse<T>>()
            .await
            .context("decode Linear GraphQL response")?;
        if let Some(error) = payload.errors.first() {
            anyhow::bail!("Linear GraphQL failed: {}", error.message);
        }

        payload.data.context("Linear GraphQL response did not include data")
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("FolioFS"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))
                .context("build Linear authorization header")?,
        );
        Ok(headers)
    }
}

impl Connector for LinearConnector {
    fn plan<'a>(
        &'a self,
        _connection: &'a ConnectionRecord,
    ) -> impl std::future::Future<Output = Result<SyncPlan>> + Send + 'a {
        async move {
            let data = self.fetch_data().await?;
            Ok(SyncPlan {
                changed_count: data.issues.nodes.len() + data.teams.nodes.len(),
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
            let renderer = linear_renderer()?;
            let data = self.fetch_data().await?;
            let mut files = Vec::new();

            render_index(connection, &data, &renderer, &mut files)?;
            render_issues(data.issues.nodes, &renderer, &mut files)?;

            Ok(SyncOutput {
                files,
                cursor: Some(now_isoish()),
            })
        }
    }
}

fn render_index(
    connection: &ConnectionRecord,
    data: &LinearSyncData,
    renderer: &Renderer,
    files: &mut Vec<RenderedFile>,
) -> Result<()> {
    let mut team_map: HashMap<String, LinearTeamSummary> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for team in &data.teams.nodes {
        team_map.insert(
            team.key.clone(),
            LinearTeamSummary {
                key: team.key.clone(),
                name: team.name.clone(),
                slug: slug(&team.key),
                issues: Vec::new(),
            },
        );
        order.push(team.key.clone());
    }

    for issue in &data.issues.nodes {
        let key = issue.team.key.clone();
        let entry = team_map.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            LinearTeamSummary {
                key: key.clone(),
                name: issue.team.name.clone(),
                slug: slug(&key),
                issues: Vec::new(),
            }
        });
        entry.issues.push(LinearIssueSummary {
            identifier: issue.identifier.clone(),
            slug: issue.slug.clone(),
            title: issue.title.clone(),
            state: issue.state.name.clone(),
        });
    }

    let teams = order
        .into_iter()
        .filter_map(|key| team_map.remove(&key))
        .collect::<Vec<_>>();

    let index = LinearIndex {
        account: connection.provider_account_login.clone(),
        synced_at: now_isoish(),
        total_issues: data.issues.nodes.len(),
        teams,
    };
    files.push(renderer.render("index.md", "index.md".to_string(), &index)?);
    Ok(())
}

fn render_issues(
    issues: Vec<LinearIssue>,
    renderer: &Renderer,
    files: &mut Vec<RenderedFile>,
) -> Result<()> {
    for issue in issues {
        let team_slug = slug(&issue.team.key);
        files.push(renderer.render(
            "issue.md",
            format!("teams/{}/issues/{}.md", team_slug, issue.slug),
            &issue,
        )?);
    }

    Ok(())
}

fn linear_renderer() -> Result<Renderer> {
    Renderer::new(&[
        ("index.md", INDEX_TEMPLATE),
        ("issue.md", ISSUE_TEMPLATE),
    ])
}

fn linear_user_name(user: &LinearUser) -> Option<String> {
    user.display_name.clone().or_else(|| user.name.clone())
}

fn now_isoish() -> String {
    chrono::Utc::now().to_rfc3339()
}

const SYNC_QUERY: &str = r#"
query FolioLinearSync {
  organization {
    id
    name
    urlKey
  }
  teams(first: 50) {
    nodes {
      id
      key
      name
    }
  }
  issues(first: 100) {
    nodes {
      id
      identifier
      title
      url
      priority
      description
      createdAt
      updatedAt
      state {
        name
        type
      }
      team {
        key
        name
      }
      assignee {
        name
        displayName
      }
    }
  }
}
"#;
