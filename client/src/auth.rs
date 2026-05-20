//! Upstream auth manager.
//!
//! Uses Clerk as an OAuth provider with Authorization Code + PKCE. The client is
//! public, carries no secret, and stores only the resulting access/refresh tokens
//! in the OS keychain. `AuthManager::noop` remains for local no-auth development.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use base64::Engine;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use url::Url;

const AUTHORIZE_URL: &str = "https://settled-hamster-79.clerk.accounts.dev/oauth/authorize";
const TOKEN_URL: &str = "https://settled-hamster-79.clerk.accounts.dev/oauth/token";
const CLIENT_ID: &str = "rjHHgXHHq5Qhkqld";
const CALLBACK_PATH: &str = "/callback";
const TOKEN_BUNDLE_KEY: &str = "tokens";

/// Minimum sleep between refresh attempts. Used both as a floor on the proactive
/// timer and as the backoff after a failed refresh.
const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct AuthManager {
    state: Arc<AuthState>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

struct AuthState {
    mode: AuthMode,
    token: RwLock<Option<TokenState>>,
    user: RwLock<Option<AuthUser>>,
}

enum AuthMode {
    None,
    Clerk(ClerkAuth),
}

struct ClerkAuth {
    keyring_service: String,
    scope: String,
    http: reqwest::Client,
    refresh_token: RwLock<Option<String>>,
}

#[derive(Clone)]
struct TokenState {
    bearer: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredTokens {
    access_token: String,
    access_token_expires_at: u64,
    refresh_token: String,
}

fn default_expires_in() -> u64 {
    3600
}

impl AuthManager {
    /// Auth manager that injects no bearer token. Useful for local dev.
    pub fn noop() -> Self {
        let state = AuthState {
            mode: AuthMode::None,
            token: RwLock::new(None),
            user: RwLock::new(None),
        };
        Self {
            state: Arc::new(state),
        }
    }

    /// Build a Clerk PKCE auth manager. Reuses valid keychain tokens when
    /// possible, refreshes expired access tokens, and falls back to browser
    /// login when no usable refresh token exists.
    pub async fn clerk_pkce(
        keyring_service: &str,
        scope: &str,
        http: reqwest::Client,
    ) -> Result<Self> {
        build_clerk_manager(keyring_service, scope, http, StartupMode::BrowserIfNeeded).await
    }

    /// Build a Clerk auth manager without opening a browser. This is used by
    /// commands like `folio whoami` that should report auth state without
    /// surprising the user with an interactive login.
    pub async fn clerk_pkce_no_browser(
        keyring_service: &str,
        scope: &str,
        http: reqwest::Client,
    ) -> Result<Self> {
        build_clerk_manager(keyring_service, scope, http, StartupMode::NoBrowser).await
    }

    /// Ensure the user is logged in, optionally forcing a fresh browser flow.
    pub async fn clerk_login(
        keyring_service: &str,
        scope: &str,
        http: reqwest::Client,
        force: bool,
    ) -> Result<Self> {
        if force {
            delete_keyring(keyring_service, TOKEN_BUNDLE_KEY)?;
        }
        let mode = if force {
            StartupMode::BrowserRequired
        } else {
            StartupMode::BrowserIfNeeded
        };
        build_clerk_manager(keyring_service, scope, http, mode).await
    }

    pub fn logout_keyring(keyring_service: &str) -> Result<()> {
        delete_keyring(keyring_service, TOKEN_BUNDLE_KEY)?;
        Ok(())
    }

    /// Spawn a background task that refreshes the token at 80% TTL.
    /// Returns `None` for the noop manager.
    pub fn spawn_refresh_loop(&self) -> Option<JoinHandle<()>> {
        if matches!(self.state.mode, AuthMode::None) {
            return None;
        }
        let state = Arc::clone(&self.state);
        Some(tokio::spawn(refresh_loop(state)))
    }

    /// Current bearer token. `None` means no auth header should be added.
    pub async fn bearer(&self) -> Option<String> {
        self.state
            .token
            .read()
            .await
            .as_ref()
            .map(|t| t.bearer.clone())
    }

    pub async fn user_info(&self) -> Option<AuthUser> {
        self.state.user.read().await.clone()
    }

    /// Force a refresh, e.g. in response to a 401 from upstream.
    pub async fn force_refresh(&self) -> Result<()> {
        let AuthMode::Clerk(auth) = &self.state.mode else {
            return Ok(());
        };
        let token = refresh_access_token(auth)
            .await
            .context("forced token refresh")?;
        let user = user_info_from_token(&token.bearer);
        *self.state.token.write().await = Some(token);
        *self.state.user.write().await = user;
        Ok(())
    }

    pub async fn logout(&self) -> Result<()> {
        let AuthMode::Clerk(auth) = &self.state.mode else {
            return Ok(());
        };
        *self.state.token.write().await = None;
        *self.state.user.write().await = None;
        *auth.refresh_token.write().await = None;
        Self::logout_keyring(&auth.keyring_service)
    }
}

#[derive(Clone, Copy)]
enum StartupMode {
    BrowserIfNeeded,
    BrowserRequired,
    NoBrowser,
}

async fn build_clerk_manager(
    keyring_service: &str,
    scope: &str,
    http: reqwest::Client,
    mode: StartupMode,
) -> Result<AuthManager> {
    let stored_tokens = read_token_bundle(keyring_service).ok();
    let refresh_token = stored_tokens
        .as_ref()
        .map(|tokens| tokens.refresh_token.clone());
    if refresh_token.is_some() {
        tracing::debug!(service = %keyring_service, "loaded token bundle from keyring");
    }

    let auth = ClerkAuth {
        keyring_service: keyring_service.to_string(),
        scope: scope.to_string(),
        http,
        refresh_token: RwLock::new(refresh_token),
    };
    let token = initial_token(&auth, mode, stored_tokens.as_ref()).await?;
    let user = user_info_from_token(&token.bearer);
    let state = AuthState {
        mode: AuthMode::Clerk(auth),
        token: RwLock::new(Some(token)),
        user: RwLock::new(user),
    };
    Ok(AuthManager {
        state: Arc::new(state),
    })
}

async fn refresh_loop(state: Arc<AuthState>) {
    let AuthMode::Clerk(auth) = &state.mode else {
        return;
    };
    loop {
        let sleep_for = next_refresh_in(&state).await;
        tracing::debug!(?sleep_for, "scheduling next token refresh");
        tokio::time::sleep(sleep_for).await;

        match refresh_access_token(auth).await {
            Ok(token) => {
                tracing::info!("token refreshed");
                let user = user_info_from_token(&token.bearer);
                *state.token.write().await = Some(token);
                *state.user.write().await = user;
            }
            Err(err) => {
                tracing::warn!(error = %err, "token refresh failed; backing off");
                tokio::time::sleep(MIN_REFRESH_INTERVAL).await;
            }
        }
    }
}

async fn next_refresh_in(state: &AuthState) -> Duration {
    let guard = state.token.read().await;
    let Some(token) = guard.as_ref() else {
        return MIN_REFRESH_INTERVAL;
    };
    let remaining = token.expires_at.saturating_duration_since(Instant::now());
    remaining.mul_f32(0.8).max(MIN_REFRESH_INTERVAL)
}

async fn initial_token(
    auth: &ClerkAuth,
    mode: StartupMode,
    stored_tokens: Option<&StoredTokens>,
) -> Result<TokenState> {
    if !matches!(mode, StartupMode::BrowserRequired) {
        if let Some(token) = load_valid_access_token(stored_tokens) {
            tracing::debug!(service = %auth.keyring_service, "using valid access token from keyring");
            return Ok(token);
        }
    }

    if matches!(mode, StartupMode::BrowserRequired) {
        return browser_login(auth).await;
    }

    if auth.refresh_token.read().await.is_some() {
        match refresh_access_token(auth).await {
            Ok(token) => return Ok(token),
            Err(err) => tracing::warn!(error = %err, "stored refresh token failed"),
        }
    }

    if matches!(mode, StartupMode::NoBrowser) {
        bail!("not logged in; run `folio login`");
    }

    browser_login(auth).await
}

async fn refresh_access_token(auth: &ClerkAuth) -> Result<TokenState> {
    let refresh_token = auth
        .refresh_token
        .read()
        .await
        .clone()
        .context("no refresh token; browser login required")?;
    let resp = auth
        .http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", CLIENT_ID),
        ])
        .send()
        .await
        .context("refresh token POST")?
        .error_for_status()
        .context("refresh token status")?
        .json::<TokenResponse>()
        .await
        .context("decode refresh token response")?;

    store_token_response(auth, resp, false).await
}

async fn browser_login(auth: &ClerkAuth) -> Result<TokenState> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind auth callback")?;
    let port = listener.local_addr().context("auth callback addr")?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
    let pkce = PkcePair::generate();
    let state = random_string(32);
    let authorize_url = authorize_url(&redirect_uri, &pkce.challenge, &state, &auth.scope)?;

    open_browser(authorize_url.as_str()).context("open login browser")?;
    let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_callback(listener, &state))
        .await
        .context("login timed out")??;

    let resp = auth
        .http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("code_verifier", &pkce.verifier),
            ("client_id", CLIENT_ID),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await
        .context("authorization code POST")?
        .error_for_status()
        .context("authorization code status")?
        .json::<TokenResponse>()
        .await
        .context("decode authorization code response")?;

    store_token_response(auth, resp, true).await
}

async fn store_token_response(
    auth: &ClerkAuth,
    resp: TokenResponse,
    require_refresh_token: bool,
) -> Result<TokenState> {
    let access_token = resp.access_token;
    let refresh_token = match resp.refresh_token {
        Some(refresh_token) => Some(refresh_token),
        None if require_refresh_token => {
            bail!(
                "Clerk did not return a refresh token; ensure the native OAuth app allows offline_access"
            );
        }
        None => auth.refresh_token.read().await.clone(),
    };
    let Some(refresh_token) = refresh_token else {
        bail!("refresh response did not include a refresh token and no existing refresh token is cached");
    };

    let expires_at_epoch = token_expiry_epoch(&access_token)
        .unwrap_or_else(|| current_epoch_secs().saturating_add(resp.expires_in));
    let expires_at = instant_from_epoch(expires_at_epoch);
    write_token_bundle(
        &auth.keyring_service,
        &StoredTokens {
            access_token: access_token.clone(),
            access_token_expires_at: expires_at_epoch,
            refresh_token: refresh_token.clone(),
        },
    )?;
    *auth.refresh_token.write().await = Some(refresh_token);
    tracing::debug!(service = %auth.keyring_service, "stored token bundle in keyring");

    Ok(TokenState {
        bearer: access_token,
        expires_at,
    })
}

fn load_valid_access_token(stored_tokens: Option<&StoredTokens>) -> Option<TokenState> {
    let stored_tokens = stored_tokens?;
    let expires_at = instant_from_epoch(stored_tokens.access_token_expires_at);
    if expires_at <= Instant::now() + MIN_REFRESH_INTERVAL {
        return None;
    }
    Some(TokenState {
        bearer: stored_tokens.access_token.clone(),
        expires_at,
    })
}

fn read_token_bundle(service: &str) -> Result<StoredTokens> {
    let raw = read_keyring(service, TOKEN_BUNDLE_KEY)?;
    serde_json::from_str(&raw).context("decode token bundle")
}

fn write_token_bundle(service: &str, tokens: &StoredTokens) -> Result<()> {
    let raw = serde_json::to_string(tokens).context("encode token bundle")?;
    write_keyring(service, TOKEN_BUNDLE_KEY, &raw)
}

async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    let (mut stream, _) = listener.accept().await.context("auth callback accept")?;
    let mut buf = vec![0; 8192];
    let n = stream.read(&mut buf).await.context("auth callback read")?;
    let request = std::str::from_utf8(&buf[..n]).context("auth callback utf8")?;
    let result = parse_callback_code(request, expected_state);
    let response = callback_response(result.is_ok());
    stream
        .write_all(response.as_bytes())
        .await
        .context("auth callback response")?;
    result
}

fn parse_callback_code(request: &str, expected_state: &str) -> Result<String> {
    let request_line = request
        .lines()
        .next()
        .context("missing callback request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().context("missing callback method")?;
    let target = parts.next().context("missing callback target")?;
    if method != "GET" {
        bail!("unexpected callback method: {method}");
    }

    let url = Url::parse(&format!("http://127.0.0.1{target}")).context("parse callback URL")?;
    if url.path() != CALLBACK_PATH {
        bail!("unexpected callback path: {}", url.path());
    }

    let mut code = None;
    let mut state = None;
    let mut error = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            _ => {}
        }
    }

    if let Some(error) = error {
        bail!("oauth error: {error}");
    }
    if state.as_deref() != Some(expected_state) {
        bail!("oauth state mismatch");
    }
    code.context("missing authorization code")
}

fn callback_response(ok: bool) -> &'static str {
    if ok {
        return "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\nconnection: close\r\n\r\n<html><body><h1>FolioFS login complete</h1><p>You can close this tab.</p></body></html>";
    }
    "HTTP/1.1 400 Bad Request\r\ncontent-type: text/html\r\nconnection: close\r\n\r\n<html><body><h1>FolioFS login failed</h1><p>Return to the app and try again.</p></body></html>"
}

fn authorize_url(
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
    scope: &str,
) -> Result<Url> {
    let mut url = Url::parse(AUTHORIZE_URL).context("parse authorize URL")?;
    url.query_pairs_mut()
        .append_pair("client_id", CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("scope", scope);
    Ok(url)
}

struct PkcePair {
    verifier: String,
    challenge: String,
}

impl PkcePair {
    fn generate() -> Self {
        let verifier = random_string(64);
        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        Self {
            verifier,
            challenge,
        }
    }
}

fn token_expiry_epoch(access_token: &str) -> Option<u64> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: JwtClaims = serde_json::from_slice(&decoded).ok()?;
    Some(claims.exp)
}

fn instant_from_epoch(epoch: u64) -> Instant {
    let now = current_epoch_secs();
    let remaining = epoch.saturating_sub(now);
    Instant::now() + Duration::from_secs(remaining)
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[derive(Deserialize)]
struct JwtClaims {
    exp: u64,
    #[serde(default)]
    sub: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

fn user_info_from_token(access_token: &str) -> Option<AuthUser> {
    let claims = decode_claims(access_token)?;
    let subject = claims.sub.clone()?;
    Some(AuthUser {
        subject,
        email: claims.email,
        name: claims.name,
    })
}

fn decode_claims(access_token: &str) -> Option<JwtClaims> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&decoded).ok()
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn read_keyring(service: &str, key: &str) -> Result<String> {
    let entry = keyring::Entry::new(service, key)
        .with_context(|| format!("keyring entry {service}/{key}"))?;
    entry
        .get_password()
        .with_context(|| format!("read keyring {service}/{key}"))
}

fn write_keyring(service: &str, key: &str, value: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, key)
        .with_context(|| format!("keyring entry {service}/{key}"))?;
    entry
        .set_password(value)
        .with_context(|| format!("write keyring {service}/{key}"))
}

fn delete_keyring(service: &str, key: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, key)
        .with_context(|| format!("keyring entry {service}/{key}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::debug!(%service, %key, error = %err, "keyring delete skipped");
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .context("open command")?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .context("xdg-open command")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .context("rundll32 browser open")?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn open_browser(_url: &str) -> Result<()> {
    Err(anyhow!("opening a browser is not supported on this OS"))
}
