//! Local auth-terminating WebDAV reverse proxy.
//!
//! Incoming: dumb Basic auth on `127.0.0.1`. Outgoing: same request body and
//! headers (minus auth + hop-by-hop), with a fresh `Authorization: Bearer ...`
//! injected when the auth manager has one. On a 401 we force a token refresh
//! and retry once.
//!
//! Request bodies are buffered into memory (64 MiB cap) so retries are
//! possible. WebDAV bodies for the markdown use case are tiny; large file
//! transfers would need a streaming retry strategy instead.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Router;
use base64::Engine;
use bytes::Bytes;
use tokio::net::TcpListener;

use crate::auth::AuthManager;

const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

const NEVER_FORWARD: &[&str] = &["authorization", "host", "content-length"];

#[derive(Clone)]
pub struct LocalBasicCreds {
    pub user: String,
    pub password: String,
}

#[derive(Clone)]
pub struct ProxyState {
    pub upstream: Uri,
    pub auth: AuthManager,
    pub local_creds: LocalBasicCreds,
    pub http: reqwest::Client,
}

pub async fn bind(addr: SocketAddr) -> Result<(TcpListener, SocketAddr)> {
    let listener = TcpListener::bind(addr).await.context("bind proxy")?;
    let addr = listener.local_addr().context("local_addr")?;
    Ok((listener, addr))
}

pub async fn serve(listener: TcpListener, state: ProxyState) -> Result<()> {
    let app = Router::new().fallback(handle).with_state(state);
    axum::serve(listener, app).await.context("axum serve")
}

async fn handle(State(state): State<ProxyState>, req: Request) -> Response {
    match try_forward(&state, req).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn try_forward(state: &ProxyState, req: Request) -> Result<Response, ProxyError> {
    check_local_basic(req.headers(), &state.local_creds)?;
    let (parts, body) = req.into_parts();
    let url = build_upstream_url(&state.upstream, &parts.uri);
    let body = axum::body::to_bytes(body, MAX_BODY_BYTES)
        .await
        .map_err(|_| ProxyError::BodyTooLarge)?;

    let response = forward(state, &parts.method, &url, &parts.headers, body.clone()).await?;
    if response.status() != StatusCode::UNAUTHORIZED || state.auth.bearer().await.is_none() {
        return Ok(response);
    }

    tracing::info!(%url, "upstream 401, refreshing token and retrying once");
    state
        .auth
        .force_refresh()
        .await
        .map_err(ProxyError::AuthRefresh)?;
    forward(state, &parts.method, &url, &parts.headers, body).await
}

async fn forward(
    state: &ProxyState,
    method: &Method,
    url: &str,
    incoming_headers: &HeaderMap,
    body: Bytes,
) -> Result<Response, ProxyError> {
    let mut req = state.http.request(method.clone(), url);
    for (name, value) in incoming_headers {
        if should_forward(name) {
            req = req.header(name, value);
        }
    }
    if let Some(token) = state.auth.bearer().await {
        let user = state.auth.user_info().await;
        tracing::info!(
            %method,
            %url,
            jwt = %token_preview(&token),
            subject = ?user.as_ref().map(|u| u.subject.as_str()),
            email = ?user.as_ref().and_then(|u| u.email.as_deref()),
            name = ?user.as_ref().and_then(|u| u.name.as_deref()),
            "proxy injecting bearer token"
        );
        req = req.bearer_auth(token);
    } else {
        tracing::info!(%method, %url, "proxy forwarding without bearer token");
    }
    let upstream = req.body(body).send().await?;
    Ok(build_response(upstream))
}

fn token_preview(token: &str) -> String {
    let prefix = token.chars().take(16).collect::<String>();
    format!("{prefix}...")
}

fn build_response(upstream: reqwest::Response) -> Response {
    let status = upstream.status();
    let mut builder = Response::builder().status(status.as_u16());
    for (name, value) in upstream.headers() {
        if should_forward(name) {
            builder = builder.header(name, value);
        }
    }
    let stream = upstream.bytes_stream();
    builder
        .body(Body::from_stream(stream))
        .expect("valid response")
}

fn should_forward(name: &HeaderName) -> bool {
    let lower = name.as_str();
    !HOP_BY_HOP.iter().any(|h| lower.eq_ignore_ascii_case(h))
        && !NEVER_FORWARD.iter().any(|h| lower.eq_ignore_ascii_case(h))
}

fn build_upstream_url(upstream: &Uri, incoming: &Uri) -> String {
    let scheme = upstream.scheme_str().unwrap_or("http");
    let authority = upstream.authority().map(|a| a.as_str()).unwrap_or("");
    let base_path = upstream.path().trim_end_matches('/');
    let incoming_pq = incoming.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    format!("{scheme}://{authority}{base_path}{incoming_pq}")
}

fn check_local_basic(headers: &HeaderMap, expected: &LocalBasicCreds) -> Result<(), ProxyError> {
    let header = headers
        .get(header::AUTHORIZATION)
        .ok_or(ProxyError::MissingBasic)?;
    let value = header.to_str().map_err(|_| ProxyError::BadBasic)?;
    let encoded = value.strip_prefix("Basic ").ok_or(ProxyError::BadBasic)?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|_| ProxyError::BadBasic)?;
    let creds = std::str::from_utf8(&decoded).map_err(|_| ProxyError::BadBasic)?;
    let (user, pass) = creds.split_once(':').ok_or(ProxyError::BadBasic)?;
    if user == expected.user && pass == expected.password {
        return Ok(());
    }
    Err(ProxyError::BadBasic)
}

#[derive(Debug, thiserror::Error)]
enum ProxyError {
    #[error("missing basic auth")]
    MissingBasic,
    #[error("invalid basic auth")]
    BadBasic,
    #[error("request body too large")]
    BodyTooLarge,
    #[error("auth refresh failed: {0}")]
    AuthRefresh(anyhow::Error),
    #[error("upstream transport: {0}")]
    Upstream(#[from] reqwest::Error),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::MissingBasic | Self::BadBasic => StatusCode::UNAUTHORIZED,
            Self::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Upstream(_) | Self::AuthRefresh(_) => StatusCode::BAD_GATEWAY,
        };
        if !matches!(self, Self::MissingBasic | Self::BadBasic) {
            tracing::warn!(error = %self, "proxy error");
        }

        let mut response = (status, self.to_string()).into_response();
        if matches!(self, Self::MissingBasic | Self::BadBasic) {
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Basic realm=\"FolioFS\""),
            );
        }
        response
    }
}
