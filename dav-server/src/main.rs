//! FolioFS WebDAV server.
//!
//! Serves a local directory over WebDAV. The directory is whatever the render
//! pipeline writes to. Backend is `dav_server::localfs::LocalFs` for now; the
//! plan is to swap in an on-demand `DavFileSystem` once the protocol path is
//! proven via OS WebDAV clients.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use dav_server::body::Body;
use dav_server::fakels::FakeLs;
use dav_server::localfs::LocalFs;
use dav_server::DavHandler;
use hyper::body::Incoming;
use hyper::http::{Method, Request, Response, StatusCode};
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

    /// Disable Clerk JWT validation. Useful for raw WebDAV protocol testing.
    #[arg(long)]
    no_auth: bool,

    /// Log the full raw JWT. This is only for local debugging.
    #[arg(long)]
    log_raw_jwt: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    let verifier = build_auth_verifier(&args).await?;
    serve(args.bind, args.root, verifier).await
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

fn build_handler(root: &Path) -> Result<DavHandler> {
    if !root.is_dir() {
        return Err(anyhow!(
            "render root is missing or not a directory: {}",
            root.display()
        ));
    }
    Ok(DavHandler::builder()
        .filesystem(LocalFs::new(root, false, false, false))
        .locksystem(FakeLs::new())
        .build_handler())
}

async fn serve(addr: SocketAddr, root: PathBuf, verifier: Option<AuthVerifier>) -> Result<()> {
    let listener = TcpListener::bind(addr).await.context("bind")?;
    tracing::info!(%addr, auth_enabled = verifier.is_some(), "foliofs-dav-server listening");

    loop {
        let (stream, peer) = listener.accept().await.context("accept")?;
        let verifier = verifier.clone();
        let root = root.clone();
        tokio::spawn(async move {
            if let Err(err) = serve_conn(stream, root, verifier).await {
                tracing::warn!(%peer, error = %err, "connection ended");
            }
        });
    }
}

async fn serve_conn(
    stream: TcpStream,
    root: PathBuf,
    verifier: Option<AuthVerifier>,
) -> Result<()> {
    let io = TokioIo::new(stream);
    let service = service_fn(move |req| {
        let verifier = verifier.clone();
        let root = root.clone();
        async move { Ok::<_, Infallible>(handle_request(req, root, verifier).await) }
    });
    http1::Builder::new()
        .serve_connection(io, service)
        .await
        .context("serve_connection")
}

async fn handle_request(
    req: Request<Incoming>,
    root: PathBuf,
    verifier: Option<AuthVerifier>,
) -> Response<Body> {
    if req.method() == Method::GET && req.uri().path() == "/healthz" {
        return health_ok();
    }

    let Some(verifier) = verifier.as_ref() else {
        return match build_handler(&root) {
            Ok(handler) => handler.handle(req).await,
            Err(err) => {
                tracing::warn!(error = %err, "failed to build unauthenticated WebDAV handler");
                internal_error()
            }
        };
    };

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    match verifier.verify_header(req.headers(), &method, &path) {
        Ok(user) => {
            let user_root = root.join(&user.user_dir);
            if let Err(err) = tokio::fs::create_dir_all(&user_root).await {
                tracing::warn!(
                    subject = %user.subject,
                    user_dir = %user.user_dir,
                    error = %err,
                    "failed to create user directory"
                );
                return internal_error();
            }
            match build_handler(&user_root) {
                Ok(handler) => handler.handle(req).await,
                Err(err) => {
                    tracing::warn!(
                        subject = %user.subject,
                        user_dir = %user.user_dir,
                        error = %err,
                        "failed to build user-scoped WebDAV handler"
                    );
                    internal_error()
                }
            }
        }
        Err(err) => {
            tracing::warn!(%method, path, error = %err, "rejecting unauthenticated WebDAV request");
            unauthorized()
        }
    }
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
