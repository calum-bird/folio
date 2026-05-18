//! Menu bar tray UI.
//!
//! macOS requires the GUI event loop on the main thread. The tray loop owns the
//! main thread, while the FolioFS async supervisor runs on a Tokio runtime in a
//! worker thread. Communication is via small channels and tao user events.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::EventLoopExtMacOS;
use tokio::sync::mpsc;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::supervisor::{run_client, ClientCommand, ClientStatus};
use crate::Args;

const OPEN_ID: &str = "foliofs.open";
const MOUNT_ID: &str = "foliofs.mount";
const UNMOUNT_ID: &str = "foliofs.unmount";
const LOGOUT_ID: &str = "foliofs.logout";
const QUIT_ID: &str = "foliofs.quit";

#[derive(Debug)]
enum UserEvent {
    Menu(MenuEvent),
    Status(ClientStatus),
}

pub(crate) fn run(args: Args) -> Result<()> {
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    hide_dock_icon(&mut event_loop);

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let menu = TrayMenu::new()?;
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (status_tx, status_rx) = mpsc::unbounded_channel();

    spawn_status_bridge(status_rx, event_loop.create_proxy());
    spawn_client(args, command_rx, status_tx);

    let mut state = TrayState {
        menu,
        tray_icon: None,
        mount_path: None,
        command_tx: Some(command_tx),
        quit_requested: false,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => state.create_tray_icon(),
            Event::UserEvent(UserEvent::Status(status)) => {
                state.apply_status(status, control_flow);
            }
            Event::UserEvent(UserEvent::Menu(event)) => {
                state.handle_menu(event.id.as_ref(), control_flow);
            }
            Event::LoopDestroyed => state.request_shutdown(),
            _ => {}
        }
    });
}

#[cfg(target_os = "macos")]
fn hide_dock_icon<T>(event_loop: &mut tao::event_loop::EventLoop<T>) {
    event_loop.set_dock_visibility(false);
}

#[cfg(not(target_os = "macos"))]
fn hide_dock_icon<T>(_event_loop: &mut tao::event_loop::EventLoop<T>) {}

struct TrayMenu {
    menu: Menu,
    auth_status: MenuItem,
    proxy_status: MenuItem,
    mount_status: MenuItem,
    open: MenuItem,
    mount: MenuItem,
    unmount: MenuItem,
    logout: MenuItem,
    quit: MenuItem,
}

impl TrayMenu {
    fn new() -> Result<Self> {
        let auth_status =
            MenuItem::with_id(MenuId::new("foliofs.auth"), "Auth: starting", false, None);
        let proxy_status =
            MenuItem::with_id(MenuId::new("foliofs.proxy"), "Proxy: starting", false, None);
        let mount_status = MenuItem::with_id(
            MenuId::new("foliofs.mount_status"),
            "Mount: starting",
            false,
            None,
        );
        let open = MenuItem::with_id(MenuId::new(OPEN_ID), "Open FolioFS", false, None);
        let mount = MenuItem::with_id(MenuId::new(MOUNT_ID), "Mount", false, None);
        let unmount = MenuItem::with_id(MenuId::new(UNMOUNT_ID), "Unmount", false, None);
        let logout = MenuItem::with_id(MenuId::new(LOGOUT_ID), "Logout", false, None);
        let separator = PredefinedMenuItem::separator();
        let action_separator = PredefinedMenuItem::separator();
        let quit = MenuItem::with_id(MenuId::new(QUIT_ID), "Quit FolioFS", true, None);
        let menu = Menu::with_items(&[
            &auth_status,
            &proxy_status,
            &mount_status,
            &separator,
            &open,
            &mount,
            &unmount,
            &logout,
            &action_separator,
            &quit,
        ])
        .context("create tray menu")?;
        Ok(Self {
            menu,
            auth_status,
            proxy_status,
            mount_status,
            open,
            mount,
            unmount,
            logout,
            quit,
        })
    }
}

struct TrayState {
    menu: TrayMenu,
    tray_icon: Option<TrayIcon>,
    mount_path: Option<PathBuf>,
    command_tx: Option<mpsc::UnboundedSender<ClientCommand>>,
    quit_requested: bool,
}

impl TrayState {
    fn create_tray_icon(&mut self) {
        if self.tray_icon.is_some() {
            return;
        }

        match TrayIconBuilder::new()
            .with_menu(Box::new(self.menu.menu.clone()))
            .with_tooltip("FolioFS")
            .with_icon(folio_icon())
            .build()
        {
            Ok(icon) => self.tray_icon = Some(icon),
            Err(err) => {
                tracing::error!(error = %err, "failed to create tray icon");
                self.menu.mount_status.set_text("Tray failed");
            }
        }
    }

    fn apply_status(&mut self, status: ClientStatus, control_flow: &mut ControlFlow) {
        match status {
            ClientStatus::Starting => {
                self.menu.auth_status.set_text("Auth: starting");
                self.menu.proxy_status.set_text("Proxy: starting");
                self.set_mount_status("Mount: starting", false, false, false);
            }
            ClientStatus::AuthDisabled => {
                self.menu.auth_status.set_text("Auth: local no-auth");
                self.menu.logout.set_enabled(false);
            }
            ClientStatus::Authenticated(user) => {
                self.menu
                    .auth_status
                    .set_text(format!("Auth: {}", user_label(&user)));
                self.menu.logout.set_enabled(true);
            }
            ClientStatus::LoggedOut => {
                self.menu.auth_status.set_text("Auth: logged out");
                self.menu.logout.set_enabled(false);
                self.set_mount_status("Mount: login required", false, false, false);
            }
            ClientStatus::ProxyListening(addr) => {
                self.menu.proxy_status.set_text(format!("Proxy: {addr}"));
            }
            ClientStatus::Mounting => {
                self.set_mount_status("Mount: mounting...", false, false, false)
            }
            ClientStatus::Mounted(path) => {
                self.mount_path = Some(path);
                self.set_mount_status("Mount: mounted", true, false, true);
            }
            ClientStatus::Unmounting(path) => {
                self.set_mount_status(
                    format!("Mount: unmounting {}", path.display()),
                    false,
                    false,
                    false,
                );
            }
            ClientStatus::Unmounted => {
                self.mount_path = None;
                self.set_mount_status("Mount: unmounted", false, true, false);
            }
            ClientStatus::Stopped => {
                self.menu.proxy_status.set_text("Proxy: stopped");
                self.set_mount_status("Mount: stopped", false, false, false);
                if self.quit_requested {
                    *control_flow = ControlFlow::ExitWithCode(0);
                }
            }
            ClientStatus::Failed(message) => {
                self.menu.mount_status.set_text(format!("Error: {message}"));
            }
        }
    }

    fn handle_menu(&mut self, id: &str, control_flow: &mut ControlFlow) {
        match id {
            OPEN_ID => self.open_mount_path(),
            MOUNT_ID => self.send_command(ClientCommand::Mount),
            UNMOUNT_ID => self.send_command(ClientCommand::Unmount),
            LOGOUT_ID => self.send_command(ClientCommand::Logout),
            QUIT_ID => {
                self.menu.quit.set_enabled(false);
                self.quit_requested = true;
                self.request_shutdown();
                self.set_mount_status("Mount: quitting...", false, false, false);
            }
            _ => {}
        }

        if self.quit_requested && self.command_tx.is_none() && self.mount_path.is_none() {
            *control_flow = ControlFlow::ExitWithCode(0);
        }
    }

    fn request_shutdown(&mut self) {
        let Some(tx) = self.command_tx.take() else {
            return;
        };
        let _ = tx.send(ClientCommand::Shutdown);
    }

    fn send_command(&self, command: ClientCommand) {
        let Some(tx) = self.command_tx.as_ref() else {
            return;
        };
        let _ = tx.send(command);
    }

    fn open_mount_path(&self) {
        let Some(path) = self.mount_path.as_ref() else {
            return;
        };
        if let Err(err) = Command::new("open").arg(path).spawn() {
            tracing::warn!(error = %err, path = %path.display(), "failed to open mount");
        }
    }

    fn set_mount_status<S: AsRef<str>>(
        &self,
        text: S,
        open_enabled: bool,
        mount_enabled: bool,
        unmount_enabled: bool,
    ) {
        self.menu.mount_status.set_text(text);
        self.menu.open.set_enabled(open_enabled);
        self.menu.mount.set_enabled(mount_enabled);
        self.menu.unmount.set_enabled(unmount_enabled);
    }
}

fn user_label(user: &crate::auth::AuthUser) -> String {
    if let Some(name) = user.name.as_deref().filter(|s| !s.is_empty()) {
        return name.to_string();
    }
    if let Some(email) = user.email.as_deref().filter(|s| !s.is_empty()) {
        return email.to_string();
    }
    user.subject.clone()
}

fn spawn_client(
    args: Args,
    command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    status_tx: mpsc::UnboundedSender<ClientStatus>,
) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = status_tx.send(ClientStatus::Failed(format!("runtime: {err}")));
                return;
            }
        };

        let tx = status_tx.clone();
        let result = runtime.block_on(run_client(args, command_rx, Some(status_tx), true));
        if let Err(err) = result {
            let _ = tx.send(ClientStatus::Failed(err.to_string()));
        }
    });
}

fn spawn_status_bridge(
    mut status_rx: mpsc::UnboundedReceiver<ClientStatus>,
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
) {
    std::thread::spawn(move || {
        while let Some(status) = status_rx.blocking_recv() {
            if proxy.send_event(UserEvent::Status(status)).is_err() {
                return;
            }
        }
    });
}

fn folio_icon() -> Icon {
    let width = 16;
    let height = 16;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let on = x == 3 || x == 12 || y == 3 || y == 12 || (x > 5 && x < 10 && y > 5 && y < 10);
            if on {
                rgba.extend_from_slice(&[34, 82, 255, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, width as u32, height as u32).expect("valid embedded icon")
}
