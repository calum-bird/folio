//! FolioFS client.
//!
//! Headless pipeline on startup:
//!   1. Load OAuth client_credentials from the OS keychain (optional).
//!   2. Fetch an initial bearer token; spawn a refresh task at 80% TTL.
//!   3. Bind a localhost WebDAV reverse proxy with a random Basic password.
//!   4. Drive the OS WebDAV client to mount that localhost endpoint.
//!   5. Wait for Ctrl-C, then unmount cleanly and exit.
//!
//! Tray mode keeps the same proxy/auth session alive, but drives mount,
//! unmount, and shutdown through menu commands.
//!
//! The OS only ever sees harmless localhost Basic creds. The real OAuth
//! credentials live in the keychain and this process's memory, never on a
//! mount table, never sent anywhere besides the upstream over TLS.

#[cfg(feature = "tray")]
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

mod auth;
mod mount;
mod proxy;
mod supervisor;
#[cfg(feature = "tray")]
mod tray;

#[derive(Debug, Parser)]
#[command(
    name = "foliofs-client",
    version,
    about = "FolioFS local auth-terminating WebDAV proxy and auto-mount"
)]
pub(crate) struct Args {
    /// Upstream WebDAV URL. e.g. http://127.0.0.1:4918 for local dev,
    /// https://api.folio.fs for production.
    #[arg(long)]
    upstream: String,

    /// Disable upstream auth. Useful only for local no-auth dav-server testing.
    #[arg(long)]
    no_auth: bool,

    /// Keychain service name used to store Clerk access and refresh tokens.
    #[arg(long, default_value = "foliofs")]
    keyring_service: String,

    /// OAuth scopes to request during browser login.
    #[arg(long, default_value = "email offline_access profile")]
    scope: String,

    /// Local proxy bind address. `127.0.0.1:0` picks a random free port.
    #[arg(long, default_value = "127.0.0.1:0")]
    listen: SocketAddr,

    /// Run with a menu bar tray icon instead of the headless Ctrl-C flow.
    #[cfg(feature = "tray")]
    #[arg(long)]
    tray: bool,
}

#[cfg(not(feature = "tray"))]
#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    run_headless(args).await
}

#[cfg(feature = "tray")]
fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    if args.tray {
        return tray::run(args);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("tokio runtime")?;
    runtime.block_on(run_headless(args))
}

async fn run_headless(args: Args) -> Result<()> {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let ctrl_c_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = command_tx.send(supervisor::ClientCommand::Shutdown);
        }
    });
    let result = supervisor::run_client(args, command_rx, None, true).await;
    ctrl_c_task.abort();
    result
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,foliofs_client=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
