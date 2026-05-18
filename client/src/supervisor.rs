//! Client supervisor.
//!
//! Owns the long-lived auth/proxy session and accepts mount lifecycle commands.
//! Headless mode sends only `Shutdown` from Ctrl-C. Tray mode sends `Mount`,
//! `Unmount`, and `Shutdown` while the proxy stays alive.

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::sync::mpsc;

use crate::auth::AuthUser;
use crate::{auth, mount, proxy, Args};

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "tray"), allow(dead_code))]
pub(crate) enum ClientStatus {
    Starting,
    AuthDisabled,
    Authenticated(AuthUser),
    LoggedOut,
    ProxyListening(SocketAddr),
    Mounting,
    Mounted(PathBuf),
    Unmounting(PathBuf),
    Unmounted,
    Stopped,
    Failed(String),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(feature = "tray"), allow(dead_code))]
pub(crate) enum ClientCommand {
    Mount,
    Unmount,
    Logout,
    Shutdown,
}

pub(crate) async fn run_client(
    args: Args,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    status_tx: Option<mpsc::UnboundedSender<ClientStatus>>,
    auto_mount: bool,
) -> Result<()> {
    send_status(&status_tx, ClientStatus::Starting);
    let auth_required = !args.no_auth;
    let http = reqwest::Client::builder().build().context("http client")?;
    let auth = build_auth(&args, &http).await?;
    send_auth_status(&auth, &status_tx, auth_required).await;
    let _refresh_task = auth.spawn_refresh_loop();

    let upstream: axum::http::Uri = args.upstream.parse().context("parse --upstream URL")?;
    let local_creds = generate_local_creds();
    let (listener, listen_addr) = proxy::bind(args.listen).await?;
    tracing::info!(%listen_addr, %upstream, "proxy listening");
    send_status(&status_tx, ClientStatus::ProxyListening(listen_addr));

    let state = proxy::ProxyState {
        upstream,
        auth: auth.clone(),
        local_creds: local_creds.clone(),
        http,
    };
    let mut proxy_task = tokio::spawn(proxy::serve(listener, state));
    let local_url = format!("http://{listen_addr}/");
    let mut mount_path = initial_mount(auto_mount, &local_url, &local_creds, &status_tx).await;

    loop {
        tokio::select! {
            command = command_rx.recv() => {
                let Some(command) = command else {
                    break;
                };
                handle_command(
                    command,
                    &local_url,
                    &local_creds,
                    &mut mount_path,
                    &status_tx,
                    &mut proxy_task,
                    &auth,
                ).await;

                if matches!(command, ClientCommand::Shutdown) {
                    break;
                }
            }
            res = &mut proxy_task => {
                handle_proxy_result(res, &status_tx);
                break;
            }
        }
    }

    send_status(&status_tx, ClientStatus::Stopped);
    Ok(())
}

async fn initial_mount(
    auto_mount: bool,
    local_url: &str,
    local_creds: &proxy::LocalBasicCreds,
    status_tx: &Option<mpsc::UnboundedSender<ClientStatus>>,
) -> Option<PathBuf> {
    if auto_mount {
        return mount_once(local_url, local_creds, status_tx).await;
    }
    send_status(status_tx, ClientStatus::Unmounted);
    None
}

async fn handle_command(
    command: ClientCommand,
    local_url: &str,
    local_creds: &proxy::LocalBasicCreds,
    mount_path: &mut Option<PathBuf>,
    status_tx: &Option<mpsc::UnboundedSender<ClientStatus>>,
    proxy_task: &mut tokio::task::JoinHandle<Result<()>>,
    auth: &auth::AuthManager,
) {
    match command {
        ClientCommand::Mount => {
            if mount_path.is_some() {
                return;
            }
            *mount_path = mount_once(local_url, local_creds, status_tx).await;
        }
        ClientCommand::Unmount => {
            unmount_current(mount_path, status_tx).await;
        }
        ClientCommand::Logout => {
            unmount_current(mount_path, status_tx).await;
            match auth.logout().await {
                Ok(()) => send_status(status_tx, ClientStatus::LoggedOut),
                Err(err) => send_status(status_tx, ClientStatus::Failed(format!("logout: {err}"))),
            }
        }
        ClientCommand::Shutdown => {
            unmount_current(mount_path, status_tx).await;
            proxy_task.abort();
        }
    }
}

fn handle_proxy_result(
    res: std::result::Result<Result<()>, tokio::task::JoinError>,
    status_tx: &Option<mpsc::UnboundedSender<ClientStatus>>,
) {
    match res {
        Ok(Ok(())) => tracing::warn!("proxy stopped unexpectedly"),
        Ok(Err(err)) => {
            tracing::error!(error = %err, "proxy errored");
            send_status(status_tx, ClientStatus::Failed(format!("proxy: {err}")));
        }
        Err(err) => {
            if err.is_cancelled() {
                return;
            }
            tracing::error!(error = %err, "proxy task panicked");
            send_status(
                status_tx,
                ClientStatus::Failed(format!("proxy task: {err}")),
            );
        }
    }
}

async fn mount_once(
    local_url: &str,
    local_creds: &proxy::LocalBasicCreds,
    status_tx: &Option<mpsc::UnboundedSender<ClientStatus>>,
) -> Option<PathBuf> {
    send_status(status_tx, ClientStatus::Mounting);
    let result = mount::mount(
        local_url,
        mount::LocalCreds {
            user: &local_creds.user,
            password: &local_creds.password,
        },
    )
    .await;

    match result {
        Ok(path) => {
            tracing::info!(mount_path = %path.display(), "mounted");
            send_status(status_tx, ClientStatus::Mounted(path.clone()));
            Some(path)
        }
        Err(err) => {
            tracing::warn!(error = %err, "mount failed");
            send_status(status_tx, ClientStatus::Failed(format!("mount: {err}")));
            send_status(status_tx, ClientStatus::Unmounted);
            None
        }
    }
}

async fn unmount_current(
    mount_path: &mut Option<PathBuf>,
    status_tx: &Option<mpsc::UnboundedSender<ClientStatus>>,
) {
    let Some(path) = mount_path.take() else {
        send_status(status_tx, ClientStatus::Unmounted);
        return;
    };

    tracing::info!(mount_path = %path.display(), "unmounting");
    send_status(status_tx, ClientStatus::Unmounting(path.clone()));
    if let Err(err) = mount::unmount(&path).await {
        tracing::warn!(error = %err, "unmount failed; you may need to eject manually");
        send_status(status_tx, ClientStatus::Failed(format!("unmount: {err}")));
    }
    send_status(status_tx, ClientStatus::Unmounted);
}

async fn build_auth(args: &Args, http: &reqwest::Client) -> Result<auth::AuthManager> {
    if args.no_auth {
        tracing::warn!("--no-auth set; running without upstream auth");
        return Ok(auth::AuthManager::noop());
    }
    auth::AuthManager::clerk_pkce(&args.keyring_service, &args.scope, http.clone()).await
}

async fn send_auth_status(
    auth: &auth::AuthManager,
    tx: &Option<mpsc::UnboundedSender<ClientStatus>>,
    auth_required: bool,
) {
    if !auth_required {
        send_status(tx, ClientStatus::AuthDisabled);
        return;
    }
    if let Some(user) = auth.user_info().await {
        send_status(tx, ClientStatus::Authenticated(user));
        return;
    }
    tracing::warn!("auth is enabled but access token has no readable user claims");
    send_status(tx, ClientStatus::AuthDisabled);
}

fn send_status(tx: &Option<mpsc::UnboundedSender<ClientStatus>>, status: ClientStatus) {
    if let Some(tx) = tx {
        let _ = tx.send(status);
    }
}

fn generate_local_creds() -> proxy::LocalBasicCreds {
    let password: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    proxy::LocalBasicCreds {
        user: "folio".into(),
        password,
    }
}
