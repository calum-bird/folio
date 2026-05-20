//! FolioFS WebDAV server.
//!
//! Serves a local directory over WebDAV. The directory is whatever the render
//! pipeline writes to. Backend is `dav_server::localfs::LocalFs` for now; the
//! plan is to swap in an on-demand `DavFileSystem` once the protocol path is
//! proven via OS WebDAV clients.
//!
//! The server is intentionally read-only and only advertises WebDAV class 1
//! in `OPTIONS` responses, which causes macOS `mount_webdav` to mount the
//! share read-only without any client-side flags.
//!
//! Requests are expected under the configured `--url-prefix` (default
//! `/foliofs.dev`). That prefix becomes the last URL path component the OS
//! sees, which is what macOS Finder uses as the volume name. Pointing the OS
//! at `http://host/foliofs.dev/` therefore yields `/Volumes/foliofs.dev`
//! instead of `/Volumes/127.0.0.1 (N)`.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use dav_server::body::Body;
use dav_server::localfs::LocalFs;
use dav_server::DavHandler;
use dav_server::DavMethodSet;
use hyper::body::Incoming;
use hyper::http::{HeaderValue, Method, Request, Response, StatusCode};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tracing_subscriber::EnvFilter;

mod auth;

use auth::AuthVerifier;

#[derive(Debug, Parser)]
#[command(name = "foliofs-dav-server", version, about = "FolioFS WebDAV server")]
struct Args {
    /// Address to bind the WebDAV listener on.
    #[arg(long, default_value = "127.0.0.1:4918")]
    bind: SocketAddr,

    /// Directory to serve. Created by the render pipeline.
    #[arg(long, default_value = "render")]
    root: PathBuf,

    /// URL prefix under which the WebDAV tree is served. The last segment of
    /// this prefix becomes the macOS volume name when a client mounts
    /// `http(s)://host<prefix>/`. Set to an empty string to serve from the
    /// root path with no prefix (you will get `/Volumes/<host>` then).
    #[arg(long, default_value = "/foliofs.dev")]
    url_prefix: String,

    /// Disable Clerk JWT validation. Useful for raw WebDAV protocol testing.
    #[arg(long)]
    no_auth: bool,

    /// Log the full raw JWT. This is only for local debugging.
    #[arg(long)]
    log_raw_jwt: bool,
}

#[derive(Clone)]
struct ServerConfig {
    root: PathBuf,
    url_prefix: String,
    verifier: Option<AuthVerifier>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    let verifier = build_auth_verifier(&args).await?;
    let config = ServerConfig {
        root: args.root,
        url_prefix: normalize_prefix(&args.url_prefix)?,
        verifier,
    };
    serve(args.bind, config).await
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,foliofs_dav_server=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn build_auth_verifier(args: &Args) -> Result<Option<AuthVerifier>> {
    if args.no_auth {
        tracing::warn!("server auth disabled via --no-auth");
        return Ok(None);
    }
    AuthVerifier::fetch(args.log_raw_jwt).await.map(Some)
}

fn normalize_prefix(raw: &str) -> Result<String> {
    let trimmed = raw.trim_end_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if !trimmed.starts_with('/') {
        return Err(anyhow!(
            "url-prefix must start with '/' or be empty: got {raw:?}"
        ));
    }
    Ok(trimmed.to_string())
}

fn build_handler(root: &Path, url_prefix: &str) -> Result<DavHandler> {
    if !root.is_dir() {
        return Err(anyhow!(
            "render root is missing or not a directory: {}",
            root.display()
        ));
    }

    let mut builder = DavHandler::builder()
        .filesystem(LocalFs::new(root, false, false, false))
        .methods(DavMethodSet::WEBDAV_RO);

    if !url_prefix.is_empty() {
        builder = builder.strip_prefix(url_prefix.to_string());
    }

    Ok(builder.build_handler())
}

async fn serve(addr: SocketAddr, config: ServerConfig) -> Result<()> {
    let listener = TcpListener::bind(addr).await.context("bind")?;
    tracing::info!(
        %addr,
        auth_enabled = config.verifier.is_some(),
        url_prefix = %config.url_prefix,
        "foliofs-dav-server listening"
    );

    loop {
        let (stream, peer) = listener.accept().await.context("accept")?;
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(err) = serve_conn(stream, config).await {
                tracing::warn!(%peer, error = %err, "connection ended");
            }
        });
    }
}

async fn serve_conn(stream: TcpStream, config: ServerConfig) -> Result<()> {
    let io = TokioIo::new(stream);
    let service = service_fn(move |req| {
        let config = config.clone();
        async move { Ok::<_, Infallible>(handle_request(req, config).await) }
    });
    http1::Builder::new()
        .serve_connection(io, service)
        .await
        .context("serve_connection")
}

async fn handle_request(req: Request<Incoming>, config: ServerConfig) -> Response<Body> {
    if req.method() == Method::GET && req.uri().path() == "/healthz" {
        return health_ok();
    }

    let method = req.method().clone();
    let response = dispatch_dav(req, &config).await;
    apply_read_only_headers(response, &method)
}

async fn dispatch_dav(req: Request<Incoming>, config: &ServerConfig) -> Response<Body> {
    let Some(verifier) = config.verifier.as_ref() else {
        return run_handler(req, &config.root, &config.url_prefix).await;
    };

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let user = match verifier.verify_header(req.headers(), &method, &path) {
        Ok(user) => user,
        Err(err) => {
            tracing::warn!(%method, path, error = %err, "rejecting unauthenticated WebDAV request");
            return unauthorized();
        }
    };

    let user_root = config.root.join(&user.user_dir);
    if let Err(err) = tokio::fs::create_dir_all(&user_root).await {
        tracing::warn!(
            subject = %user.subject,
            user_dir = %user.user_dir,
            error = %err,
            "failed to create user directory"
        );
        return internal_error();
    }

    run_handler(req, &user_root, &config.url_prefix).await
}

async fn run_handler(req: Request<Incoming>, root: &Path, url_prefix: &str) -> Response<Body> {
    match build_handler(root, url_prefix) {
        Ok(handler) => handler.handle(req).await,
        Err(err) => {
            tracing::warn!(error = %err, root = %root.display(), "failed to build WebDAV handler");
            internal_error()
        }
    }
}

// dav-server hardcodes `DAV: 1,2,3,...` on every OPTIONS response. macOS only
// auto-mounts read-only when class 2 (LOCK) is absent, so we strip the extra
// classes here. We also do it on PROPFIND because some clients re-read the
// DAV header from those responses to update their read-only state.
fn apply_read_only_headers(mut response: Response<Body>, method: &Method) -> Response<Body> {
    if !response.status().is_success() {
        return response;
    }
    if method == Method::OPTIONS || method.as_str().eq_ignore_ascii_case("PROPFIND") {
        response
            .headers_mut()
            .insert("DAV", HeaderValue::from_static("1"));
    }
    response
}

fn health_ok() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("ok\n"))
        .expect("valid health response")
}

fn unauthorized() -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("www-authenticate", "Bearer")
        .body(Body::from("unauthorized\n"))
        .expect("valid unauthorized response")
}

fn internal_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("internal server error\n"))
        .expect("valid internal error response")
}
