use anyhow::{Context, Result};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::connector::{Connector, SyncOutput, SyncPlan};
use crate::model::{ConnectionRecord, RenderedFile};
use crate::render::{slug, Renderer};

const INDEX_TEMPLATE: &str = include_str!("templates/index.md.j2");
const REPOSITORY_TEMPLATE: &str = include_str!("templates/repository.md.j2");
const ISSUE_TEMPLATE: &str = include_str!("templates/issue.md.j2");

#[derive(Clone)]
pub struct GitHubConnector {
    access_token: String,
    owner_filter: Option<String>,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    id: u64,
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    private: bool,
    archived: bool,
    fork: bool,
    language: Option<String>,
    pushed_at: Option<String>,
    updated_at: Option<String>,
    #[serde(default)]
    issues: Vec<Issue>,
    #[serde(skip_deserializing)]
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Issue {
    id: u64,
    number: u64,
    title: String,
    state: String,
    html_url: String,
    user: Option<IssueUser>,
    labels: Vec<Label>,
    created_at: String,
    updated_at: String,
    body: Option<String>,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
    #[serde(skip_deserializing)]
    repo: String,
    #[serde(skip_deserializing)]
    author: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IssueUser {
    login: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Label {
    name: String,
}

#[derive(Debug, Serialize)]
struct GitHubIndex {
    account: String,
    synced_at: String,
    repositories: Vec<RepositorySummary>,
}

#[derive(Debug, Serialize)]
struct RepositorySummary {
    full_name: String,
    slug: String,
}

impl GitHubConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            owner_filter: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn with_owner_filter(mut self, owner: impl Into<String>) -> Self {
        self.owner_filter = Some(owner.into());
        self
    }

    async fn fetch_repositories(&self) -> Result<Vec<Repository>> {
        let mut repositories = Vec::new();
        for page in 1..=3 {
            let url = format!(
                "https://api.github.com/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&sort=updated&page={page}"
            );
            let page_repositories = self
                .get_json::<Vec<Repository>>(&url)
                .await
                .with_context(|| format!("fetch GitHub repositories page {page}"))?;
            let is_last_page = page_repositories.len() < 100;
            repositories.extend(self.filter_repositories(page_repositories));

            if is_last_page {
                return Ok(repositories);
            }
        }

        Ok(repositories)
    }

    fn filter_repositories(&self, repositories: Vec<Repository>) -> Vec<Repository> {
        let Some(owner_filter) = self.owner_filter.as_deref() else {
            return repositories;
        };

        repositories
            .into_iter()
            .filter(|repo| repo.full_name.split('/').next() == Some(owner_filter))
            .collect()
    }

    async fn fetch_issues(&self, full_name: &str) -> Result<Vec<Issue>> {
        let url = format!("https://api.github.com/repos/{full_name}/issues?state=open&per_page=50");
        let issues = self
            .get_json::<Vec<Issue>>(&url)
            .await
            .with_context(|| format!("fetch GitHub issues for {full_name}"))?;
        Ok(issues
            .into_iter()
            .filter(|issue| issue.pull_request.is_none())
            .collect())
    }

    async fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .get(url)
            .headers(self.headers()?)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub request failed: {status} {body}");
        }

        response
            .json::<T>()
            .await
            .with_context(|| format!("decode {url}"))
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("FolioFS"));
        headers.insert(
            "x-github-api-version",
            HeaderValue::from_static("2022-11-28"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))
                .context("build GitHub authorization header")?,
        );
        Ok(headers)
    }
}

impl Connector for GitHubConnector {
    fn plan<'a>(
        &'a self,
        _connection: &'a ConnectionRecord,
    ) -> impl std::future::Future<Output = Result<SyncPlan>> + Send + 'a {
        async move {
            let repositories = self.fetch_repositories().await?;
            Ok(SyncPlan {
                changed_count: repositories.len(),
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
            let renderer = github_renderer()?;
            let mut repos = self.fetch_repositories().await?;
            let mut files = Vec::new();

            for repo in &mut repos {
                repo.slug = slug(&repo.full_name.replace('/', "__"));
                repo.issues = self.fetch_issues(&repo.full_name).await?;
                for issue in &mut repo.issues {
                    issue.repo = repo.full_name.clone();
                    issue.author = issue.user.as_ref().map(|user| user.login.clone());
                }
            }

            render_index(connection, &repos, &renderer, &mut files)?;
            render_repositories(repos, &renderer, &mut files)?;

            Ok(SyncOutput {
                files,
                cursor: Some(now_isoish()),
            })
        }
    }
}

fn render_index(
    connection: &ConnectionRecord,
    repos: &[Repository],
    renderer: &Renderer,
    files: &mut Vec<RenderedFile>,
) -> Result<()> {
    let index = GitHubIndex {
        account: connection.provider_account_login.clone(),
        synced_at: now_isoish(),
        repositories: repos
            .iter()
            .map(|repo| RepositorySummary {
                full_name: repo.full_name.clone(),
                slug: repo.slug.clone(),
            })
            .collect(),
    };
    files.push(renderer.render("index.md", "index.md".to_string(), &index)?);
    Ok(())
}

fn render_repositories(
    repos: Vec<Repository>,
    renderer: &Renderer,
    files: &mut Vec<RenderedFile>,
) -> Result<()> {
    for repo in repos {
        files.push(renderer.render(
            "repository.md",
            format!("repos/{}.md", repo.slug),
            &repo,
        )?);

        for issue in repo.issues {
            files.push(renderer.render(
                "issue.md",
                format!("issues/{}-{}.md", repo.slug, issue.number),
                &issue,
            )?);
        }
    }

    Ok(())
}

fn github_renderer() -> Result<Renderer> {
    Renderer::new(&[
        ("index.md", INDEX_TEMPLATE),
        ("repository.md", REPOSITORY_TEMPLATE),
        ("issue.md", ISSUE_TEMPLATE),
    ])
}

fn now_isoish() -> String {
    chrono::Utc::now().to_rfc3339()
}
