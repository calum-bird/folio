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
use clap::{Args as ClapArgs, Parser, Subcommand};
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

mod auth;
#[cfg(target_os = "macos")]
mod launch_agent;
mod mount;
mod proxy;
mod supervisor;
#[cfg(feature = "tray")]
mod tray;

#[derive(Debug, Parser)]
#[command(
    name = "folio",
    version,
    about = "FolioFS local auth-terminating WebDAV proxy and macOS mount tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Mount FolioFS in the foreground until Ctrl-C.
    Mount(RunOptions),
    /// Start the menu-bar app detached via a per-user LaunchAgent.
    Start(RunOptions),
    /// Stop the detached menu-bar app and unmount FolioFS.
    Stop,
    /// Print launch and mount status.
    Status,
    /// Open the Clerk login flow and persist tokens in Keychain.
    Login(LoginOptions),
    /// Clear tokens, stop the app, and unmount FolioFS.
    Logout(AuthOptions),
    /// Print the currently signed-in account.
    Whoami(AuthOptions),
    /// Remove the LaunchAgent plist after stopping FolioFS.
    Uninstall,
    /// Run the menu-bar app in the foreground for debugging.
    Tray(RunOptions),
}

#[derive(Debug, Clone, ClapArgs)]
pub(crate) struct ClientConfig {
    /// Upstream WebDAV URL.
    #[arg(long, default_value = "https://api.foliofs.dev")]
    pub upstream: String,

    /// Disable upstream auth. Useful only for local no-auth dav-server testing.
    #[arg(long)]
    pub no_auth: bool,

    /// Keychain service name used to store Clerk access and refresh tokens.
    #[arg(long, default_value = "foliofs")]
    pub keyring_service: String,

    /// OAuth scopes to request during browser login.
    #[arg(long, default_value = "email offline_access profile")]
    pub scope: String,

    /// Local proxy bind address. `127.0.0.1:0` picks a random free port.
    #[arg(long, default_value = "127.0.0.1:0")]
    pub listen: SocketAddr,

    /// macOS volume name to use when mounting. Becomes the trailing path
    /// component of the mount URL (e.g. `http://127.0.0.1:PORT/foliofs.dev/`)
    /// so macOS Finder names the volume `/Volumes/foliofs.dev` instead of
    /// `/Volumes/127.0.0.1 (N)`. Must also match the upstream WebDAV
    /// server's `--url-prefix`.
    #[arg(long, default_value = "foliofs.dev")]
    pub mount_name: String,
}

#[derive(Debug, Clone, ClapArgs)]
struct RunOptions {
    #[command(flatten)]
    config: ClientConfig,
}

#[derive(Debug, Clone, ClapArgs)]
struct AuthOptions {
    /// Disable upstream auth. Useful only for local no-auth dav-server testing.
    #[arg(long)]
    no_auth: bool,

    /// Keychain service name used to store Clerk access and refresh tokens.
    #[arg(long, default_value = "foliofs")]
    keyring_service: String,

    /// OAuth scopes to request during browser login.
    #[arg(long, default_value = "email offline_access profile")]
    scope: String,
}

#[derive(Debug, Clone, ClapArgs)]
struct LoginOptions {
    #[command(flatten)]
    auth: AuthOptions,

    /// Force a browser login even if Keychain contains usable tokens.
    #[arg(long)]
    force: bool,
}

#[cfg(not(feature = "tray"))]
#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    run_cli(Cli::parse()).await
}

#[cfg(feature = "tray")]
fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    if let Command::Tray(options) = cli.command {
        return tray::run(options.config);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("tokio runtime")?;
    runtime.block_on(run_cli(cli))
}

async fn run_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Mount(options) => run_headless(options.config).await,
        Command::Start(options) => start_tray(options.config),
        Command::Stop => stop_tray(),
        Command::Status => print_status(),
        Command::Login(options) => login(options).await,
        Command::Logout(options) => logout(options).await,
        Command::Whoami(options) => whoami(options).await,
        Command::Uninstall => uninstall(),
        Command::Tray(options) => run_tray(options.config),
    }
}

async fn run_headless(config: ClientConfig) -> Result<()> {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let ctrl_c_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = command_tx.send(supervisor::ClientCommand::Shutdown);
        }
    });
    let result = supervisor::run_client(config, command_rx, None, true).await;
    ctrl_c_task.abort();
    result
}

fn run_tray(_config: ClientConfig) -> Result<()> {
    #[cfg(feature = "tray")]
    {
        tray::run(_config)
    }
    #[cfg(not(feature = "tray"))]
    {
        anyhow::bail!("this folio binary was built without tray support");
    }
}

fn start_tray(_config: ClientConfig) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        launch_agent::start(&_config)
    }
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("folio start is currently macOS-only");
    }
}

fn stop_tray() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        launch_agent::stop()
    }
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("folio stop is currently macOS-only");
    }
}

fn print_status() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        launch_agent::status()
    }
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("folio status is currently macOS-only");
    }
}

fn uninstall() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        launch_agent::uninstall()
    }
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("folio uninstall is currently macOS-only");
    }
}

async fn login(options: LoginOptions) -> Result<()> {
    if options.auth.no_auth {
        println!("Auth disabled via --no-auth.");
        return Ok(());
    }

    let http = reqwest::Client::builder().build()?;
    let auth = auth::AuthManager::clerk_login(
        &options.auth.keyring_service,
        &options.auth.scope,
        http,
        options.force,
    )
    .await?;
    print_user(auth.user_info().await.as_ref());
    Ok(())
}

async fn logout(options: AuthOptions) -> Result<()> {
    stop_tray_if_running();
    auth::AuthManager::logout_keyring(&options.keyring_service)?;
    println!("Logged out.");
    Ok(())
}

async fn whoami(options: AuthOptions) -> Result<()> {
    if options.no_auth {
        println!("Auth disabled via --no-auth.");
        return Ok(());
    }

    let http = reqwest::Client::builder().build()?;
    let auth =
        auth::AuthManager::clerk_pkce_no_browser(&options.keyring_service, &options.scope, http)
            .await?;
    print_user(auth.user_info().await.as_ref());
    Ok(())
}

fn stop_tray_if_running() {
    if let Err(err) = stop_tray() {
        tracing::debug!(error = %err, "stop skipped during logout");
    }
}

fn print_user(user: Option<&auth::AuthUser>) {
    let Some(user) = user else {
        println!("Signed in, but the access token did not include user profile details.");
        return;
    };
    if let Some(email) = user.email.as_deref() {
        println!("{email}");
        return;
    }
    if let Some(name) = user.name.as_deref() {
        println!("{name}");
        return;
    }
    println!("{}", user.subject);
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,foliofs_client=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
