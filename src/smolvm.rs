use std::{
    ffi::OsString,
    path::Path,
    process::{Command, Output},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineState {
    Running,
    Stopped,
    Created,
    Failed,
    Unreachable,
    Other(String),
}

impl MachineState {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Created => "created",
            Self::Failed => "failed",
            Self::Unreachable => "unreachable",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl From<String> for MachineState {
    fn from(value: String) -> Self {
        match value.as_str() {
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "created" => Self::Created,
            "failed" => Self::Failed,
            "unreachable" => Self::Unreachable,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MachineStatus {
    pub state: MachineState,
}

#[derive(Debug, Deserialize)]
struct StatusJson {
    state: String,
}

#[derive(Debug, Clone)]
pub struct Smolvm {
    bin: OsString,
}

impl Smolvm {
    pub fn new(bin: impl Into<OsString>) -> Self {
        Self { bin: bin.into() }
    }

    pub fn status(&self, name: &str) -> Result<Option<MachineStatus>> {
        let output = Command::new(&self.bin)
            .args(["machine", "status", "--name", name, "--json"])
            .output()
            .with_context(|| format!("run {} machine status", self.display_bin()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let combined = format!("{stderr}\n{stdout}");
            if combined.contains("not found") || combined.contains("Vm not found") {
                return Ok(None);
            }
            bail!("smolvm status failed: {}", format_output(&output));
        }

        let parsed: StatusJson = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("parse smolvm status for machine '{name}'"))?;
        Ok(Some(MachineStatus {
            state: MachineState::from(parsed.state),
        }))
    }

    pub fn create(&self, name: &str, smolfile: &Path) -> Result<()> {
        self.checked(
            Command::new(&self.bin)
                .arg("machine")
                .arg("create")
                .arg(name)
                .arg("-s")
                .arg(smolfile)
                .arg("--net-backend")
                .arg("virtio-net"),
            "create machine",
        )
    }

    pub fn start(&self, name: &str) -> Result<()> {
        self.checked(
            Command::new(&self.bin)
                .arg("machine")
                .arg("start")
                .arg("--name")
                .arg(name),
            "start machine",
        )
    }

    pub fn stop(&self, name: &str) -> Result<()> {
        self.checked(
            Command::new(&self.bin)
                .arg("machine")
                .arg("stop")
                .arg("--name")
                .arg(name),
            "stop machine",
        )
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        self.checked(
            Command::new(&self.bin)
                .arg("machine")
                .arg("delete")
                .arg(name)
                .arg("-f"),
            "delete machine",
        )
    }

    pub fn update(&self, name: &str, update: &MachineUpdate) -> Result<()> {
        let mut command = Command::new(&self.bin);
        command.arg("machine").arg("update").arg(name);
        command.arg("--net");

        if let Some(remove_volume) = &update.remove_volume {
            command.arg("--remove-volume").arg(remove_volume);
        }
        command.arg("--volume").arg(&update.volume);

        if let Some(remove_port) = &update.remove_port {
            command.arg("--remove-port").arg(remove_port);
        }
        command.arg("--port").arg(&update.port);

        if let Some(cpus) = update.cpus {
            command.arg("--cpus").arg(cpus.to_string());
        }
        if let Some(memory_mib) = update.memory_mib {
            command.arg("--mem").arg(memory_mib.to_string());
        }
        if let Some(storage_gb) = update.storage_gb {
            command.arg("--storage").arg(storage_gb.to_string());
        }
        if let Some(overlay_gb) = update.overlay_gb {
            command.arg("--overlay").arg(overlay_gb.to_string());
        }

        self.checked(&mut command, "update machine")
    }

    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.bin)
            .arg("--version")
            .output()
            .with_context(|| format!("run {} --version", self.display_bin()))?;
        if !output.status.success() {
            bail!("smolvm --version failed: {}", format_output(&output));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn checked(&self, command: &mut Command, action: &str) -> Result<()> {
        let output = command
            .output()
            .with_context(|| format!("run {} for {action}", self.display_bin()))?;
        if output.status.success() {
            Ok(())
        } else {
            bail!("smolvm {action} failed: {}", format_output(&output));
        }
    }

    fn display_bin(&self) -> String {
        self.bin.to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone)]
pub struct MachineUpdate {
    pub remove_volume: Option<String>,
    pub volume: String,
    pub remove_port: Option<String>,
    pub port: String,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    pub storage_gb: Option<u64>,
    pub overlay_gb: Option<u64>,
}

fn format_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("exit status {}", output.status),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stderr}\n{stdout}"),
    }
}
