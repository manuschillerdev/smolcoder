use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceState {
    pub workspace_id: String,
    pub workspace: PathBuf,
    pub machine: String,
    pub host_alias: String,
    pub port: u16,
    pub authorized_keys: PathBuf,
    pub identity_file: Option<PathBuf>,
    pub smolfile: PathBuf,
    pub cpus: u8,
    pub memory_mib: u32,
    pub storage_gb: u64,
    pub overlay_gb: u64,
}

impl WorkspaceState {
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(path)
            .with_context(|| format!("read state file {}", path.display()))?;
        let state = serde_json::from_str(&data)
            .with_context(|| format!("parse state file {}", path.display()))?;
        Ok(Some(state))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create state directory {}", parent.display()))?;
        }

        let data = serde_json::to_string_pretty(self).context("serialize workspace state")?;
        fs::write(path, format!("{data}\n"))
            .with_context(|| format!("write state file {}", path.display()))
    }

    pub fn needs_machine_update(&self, desired: &WorkspaceState) -> bool {
        self.workspace != desired.workspace
            || self.port != desired.port
            || self.cpus != desired.cpus
            || self.memory_mib != desired.memory_mib
            || self.storage_gb != desired.storage_gb
            || self.overlay_gb != desired.overlay_gb
    }
}
