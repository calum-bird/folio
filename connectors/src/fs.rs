use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tokio::fs;

use crate::model::{ConnectionRecord, RenderedFile};

pub async fn replace_rendered_tree(root: &Path, connection: &ConnectionRecord, files: &[RenderedFile]) -> Result<()> {
    let target = root.join(&connection.output_prefix);
    let suffix = temp_suffix();
    let next = sibling_path(&target, &format!("next-{suffix}"));
    let old = sibling_path(&target, &format!("old-{suffix}"));

    fs::create_dir_all(&next)
        .await
        .with_context(|| format!("create {}", next.display()))?;

    for file in files {
        write_file(&next, file).await?;
    }

    let _ = fs::remove_dir_all(&old).await;
    if fs::metadata(&target).await.is_ok() {
        fs::rename(&target, &old)
            .await
            .with_context(|| format!("move {} to {}", target.display(), old.display()))?;
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create {}", parent.display()))?;
    }

    fs::rename(&next, &target)
        .await
        .with_context(|| format!("move {} to {}", next.display(), target.display()))?;
    let _ = fs::remove_dir_all(&old).await;
    Ok(())
}

async fn write_file(root: &Path, file: &RenderedFile) -> Result<()> {
    let target = root.join(safe_relative_path(&file.relative_path));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create {}", parent.display()))?;
    }

    fs::write(&target, &file.contents)
        .await
        .with_context(|| format!("write {}", target.display()))
}

fn safe_relative_path(value: &str) -> PathBuf {
    let mut output = PathBuf::new();
    for part in value.split('/').filter(|part| !part.is_empty()) {
        if part == "." || part == ".." {
            continue;
        }

        output.push(part);
    }

    if output.as_os_str().is_empty() {
        return PathBuf::from("index.md");
    }

    output
}

fn sibling_path(target: &Path, suffix: &str) -> PathBuf {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "rendered".into());
    target.with_file_name(format!("{name}.{suffix}"))
}

fn temp_suffix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{}-{millis}", std::process::id())
}
