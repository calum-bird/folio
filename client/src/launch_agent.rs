//! macOS LaunchAgent management for the detached tray app.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};

use crate::ClientConfig;

const LABEL: &str = "dev.foliofs.client";

pub(crate) fn start(config: &ClientConfig) -> Result<()> {
    let plist = plist_path()?;
    write_plist(&plist, config)?;
    let _ = launchctl(&["bootout", &gui_domain()?, plist_string(&plist).as_str()]);
    launchctl(&["bootstrap", &gui_domain()?, plist_string(&plist).as_str()])?;
    launchctl(&["kickstart", "-k", &format!("{}/{}", gui_domain()?, LABEL)])?;
    println!("FolioFS started.");
    Ok(())
}

pub(crate) fn stop() -> Result<()> {
    let plist = plist_path()?;
    if plist.exists() {
        let _ = launchctl(&["bootout", &gui_domain()?, plist_string(&plist).as_str()]);
    }
    unmount_known_volume();
    println!("FolioFS stopped.");
    Ok(())
}

pub(crate) fn status() -> Result<()> {
    let service = format!("{}/{}", gui_domain()?, LABEL);
    match command_output("launchctl", &["print", &service]) {
        Ok(output) => {
            println!("LaunchAgent: loaded");
            print_pid(&output);
        }
        Err(_) => println!("LaunchAgent: not loaded"),
    }

    let mount = command_output("mount", &[])?;
    if let Some(line) = mount.lines().find(|line| line.contains("/Volumes/foliofs.dev")) {
        println!("Mount: {line}");
        return Ok(());
    }
    println!("Mount: not mounted");
    Ok(())
}

pub(crate) fn uninstall() -> Result<()> {
    stop()?;
    let plist = plist_path()?;
    if plist.exists() {
        fs::remove_file(&plist).with_context(|| format!("remove {}", plist.display()))?;
    }
    println!("Removed {}.", plist.display());
    Ok(())
}

fn write_plist(path: &PathBuf, config: &ClientConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let logs = logs_dir()?;
    fs::create_dir_all(&logs).with_context(|| format!("create {}", logs.display()))?;
    let args = program_arguments(config)?;
    let plist = render_plist(&args, &logs);
    fs::write(path, plist).with_context(|| format!("write {}", path.display()))
}

fn program_arguments(config: &ClientConfig) -> Result<Vec<String>> {
    let exe = env::current_exe().context("current executable")?;
    let mut args = vec![
        exe.to_string_lossy().to_string(),
        "tray".to_string(),
        "--upstream".to_string(),
        config.upstream.clone(),
        "--keyring-service".to_string(),
        config.keyring_service.clone(),
        "--scope".to_string(),
        config.scope.clone(),
        "--listen".to_string(),
        config.listen.to_string(),
        "--mount-name".to_string(),
        config.mount_name.clone(),
    ];
    if config.no_auth {
        args.push("--no-auth".to_string());
    }
    Ok(args)
}

fn render_plist(args: &[String], logs: &std::path::Path) -> String {
    let arguments = args
        .iter()
        .map(|arg| format!("    <string>{}</string>", xml_escape(arg)))
        .collect::<Vec<_>>()
        .join("\n");
    let stdout = logs.join("folio.out.log");
    let stderr = logs.join("folio.err.log");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
{arguments}
  </array>
  <key>RunAtLoad</key>
  <false/>
  <key>KeepAlive</key>
  <false/>
  <key>StandardOutPath</key>
  <string>{stdout}</string>
  <key>StandardErrorPath</key>
  <string>{stderr}</string>
</dict>
</plist>
"#,
        label = LABEL,
        stdout = xml_escape(&stdout.to_string_lossy()),
        stderr = xml_escape(&stderr.to_string_lossy()),
    )
}

fn print_pid(output: &str) {
    let Some(line) = output.lines().find(|line| line.trim_start().starts_with("pid = ")) else {
        return;
    };
    println!("Process: {}", line.trim());
}

fn unmount_known_volume() {
    let Ok(mounts) = command_output("mount", &[]) else {
        return;
    };
    for path in folio_mount_paths(&mounts) {
        let _ = Command::new("diskutil")
            .arg("unmount")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn folio_mount_paths(mounts: &str) -> Vec<&str> {
    mounts
        .lines()
        .filter_map(|line| line.split_once(" on ").map(|(_, rest)| rest))
        .filter_map(|rest| rest.split_once(" (").map(|(path, _)| path))
        .filter(|path| {
            *path == "/Volumes/foliofs.dev" || path.starts_with("/Volumes/foliofs.dev-")
        })
        .collect()
}

fn launchctl(args: &[&str]) -> Result<()> {
    let output = Command::new("launchctl")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("launchctl")?;
    if output.status.success() {
        return Ok(());
    }
    Err(anyhow!(
        "launchctl {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn command_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| command.to_string())?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }
    Err(anyhow!(
        "{command} {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn plist_path() -> Result<PathBuf> {
    Ok(home_dir()?.join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

fn logs_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join("Library/Logs/FolioFS"))
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .context("HOME is not set")
}

fn gui_domain() -> Result<String> {
    let uid = command_output("id", &["-u"])?;
    Ok(format!("gui/{}", uid.trim()))
}

fn plist_string(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
