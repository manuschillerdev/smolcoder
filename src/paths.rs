use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

pub fn canonical_workspace(path: &Path) -> Result<PathBuf> {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };

    let canonical = fs::canonicalize(path)
        .with_context(|| format!("canonicalize workspace {}", path.display()))?;
    if !canonical.is_dir() {
        bail!("workspace is not a directory: {}", canonical.display());
    }
    Ok(canonical)
}

pub fn workspace_id(workspace: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..8])
}

pub fn workspace_slug(workspace: &Path) -> String {
    let raw = workspace
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace");

    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        slug.push_str("workspace");
    }

    if slug.len() > 32 {
        slug.truncate(32);
        while slug.ends_with('-') {
            slug.pop();
        }
    }

    slug
}

pub fn default_machine_name(workspace: &Path) -> String {
    let hash = workspace_id(workspace);
    format!("smolcoder-{}-{}", workspace_slug(workspace), &hash[..8])
}

pub fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| anyhow::anyhow!("HOME is not set"))
}

pub fn state_root() -> Result<PathBuf> {
    let root = match env::var_os("XDG_STATE_HOME") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => home_dir()?.join(".local/state"),
    };
    Ok(root.join("smolcoder"))
}

pub fn workspace_state_dir(workspace_id: &str) -> Result<PathBuf> {
    Ok(state_root()?.join(workspace_id))
}

pub fn runtime_dir(workspace_id: &str) -> PathBuf {
    PathBuf::from("/tmp").join("smolcoder").join(workspace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_vm_name_safe() {
        assert_eq!(
            workspace_slug(Path::new("/tmp/My Cool_App!!")),
            "my-cool-app"
        );
        assert_eq!(workspace_slug(Path::new("/tmp/---")), "workspace");
    }

    #[test]
    fn default_machine_name_starts_with_alnum_and_has_no_double_dash() {
        let name = default_machine_name(Path::new("/tmp/My App"));
        assert!(name.starts_with("smolcoder-my-app-"));
        assert!(!name.contains("--"));
        assert!(name.len() <= 128);
    }
}
