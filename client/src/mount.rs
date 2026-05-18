//! OS mount lifecycle. macOS only for now.
//!
//! `mount` drives the OS WebDAV client via AppleScript, which handles the
//! Basic creds without any UI. The OS auto-creates a volume under `/Volumes/`;
//! we discover the actual path by snapshotting `/Volumes/` around the mount.
//!
//! `unmount` shells out to `diskutil unmount`.

#[cfg(not(target_os = "macos"))]
compile_error!(
    "foliofs-client currently only supports macOS; Linux/Windows mount adapters come later"
);

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::process::Command;

pub struct LocalCreds<'a> {
    pub user: &'a str,
    pub password: &'a str,
}

/// Mount `url` via the OS WebDAV client. Returns the actual `/Volumes/...`
/// path that was created.
pub async fn mount(url: &str, creds: LocalCreds<'_>) -> Result<PathBuf> {
    let before = volumes_snapshot().await?;
    run_applescript_mount(url, &creds).await?;
    let new_path = find_new_volume(before, Duration::from_secs(10)).await?;
    Ok(new_path)
}

pub async fn unmount(mount_path: &Path) -> Result<()> {
    let output = Command::new("diskutil")
        .arg("unmount")
        .arg(mount_path)
        .stdin(Stdio::null())
        .output()
        .await
        .context("diskutil unmount")?;
    if output.status.success() {
        return Ok(());
    }
    Err(anyhow!(
        "diskutil unmount failed for {}: {}",
        mount_path.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

async fn run_applescript_mount(url: &str, creds: &LocalCreds<'_>) -> Result<()> {
    let script = format!(
        r#"mount volume "{url}" as user name "{user}" with password "{pass}""#,
        url = applescript_escape(url),
        user = applescript_escape(creds.user),
        pass = applescript_escape(creds.password),
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .stdin(Stdio::null())
        .output()
        .await
        .context("osascript")?;
    if output.status.success() {
        return Ok(());
    }
    Err(anyhow!(
        "osascript mount failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// Poll `/Volumes/` for up to `timeout` until a new entry appears.
async fn find_new_volume(before: HashSet<PathBuf>, timeout: Duration) -> Result<PathBuf> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let after = volumes_snapshot().await?;
        let mut new_entries = after.difference(&before).cloned().collect::<Vec<_>>();
        match new_entries.len() {
            1 => return Ok(new_entries.remove(0)),
            n if n > 1 => {
                return Err(anyhow!(
                    "ambiguous mount, multiple new volumes appeared: {new_entries:?}"
                ));
            }
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "mount completed but no new volume appeared under /Volumes"
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn volumes_snapshot() -> Result<HashSet<PathBuf>> {
    let mut entries = HashSet::new();
    let mut dir = tokio::fs::read_dir("/Volumes")
        .await
        .context("read /Volumes")?;
    while let Some(entry) = dir.next_entry().await.context("/Volumes entry")? {
        entries.insert(entry.path());
    }
    Ok(entries)
}

fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
