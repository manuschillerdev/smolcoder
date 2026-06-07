use std::{fs, path::PathBuf, time::Duration};

use ::smolvm as smolvm_crate;
use anyhow::{Context, Result, bail};
use smolvm_crate::{
    HostMount,
    data::network::PortMapping,
    machine::{
        CreateMachine, DeleteMachine, ExecMachine, GetMachine, LocalMachineService, MachineService,
        StartMachine, StopMachine, UpdateMachine,
    },
    network::NetworkBackend,
};

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
    pub image: Option<String>,
}

pub const GUEST_IMAGE: &str = "debian:bookworm-slim";

const GUEST_INIT: &str = r#"set -eu
export DEBIAN_FRONTEND=noninteractive

if ! command -v sshd >/dev/null 2>&1; then
  apt-get update
  apt-get install -y --no-install-recommends \
    openssh-server git ca-certificates curl tar gzip unzip procps libstdc++6 bash
  rm -rf /var/lib/apt/lists/*
fi

chown root:root /root 2>/dev/null || true
chmod 700 /root 2>/dev/null || true
install -d -m 700 /root/.ssh
touch /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys

ssh-keygen -A
mkdir -p /run/sshd /etc/ssh/sshd_config.d /var/empty
chown root:root /var/empty
chmod 755 /var/empty
cat > /etc/ssh/sshd_config.d/99-smolcoder.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
PermitRootLogin prohibit-password
SetEnv SSH_AUTH_SOCK=/tmp/ssh-agent.sock
EOF
"#;

#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub workspace: PathBuf,
    pub port: u16,
    pub cpus: u8,
    pub memory_mib: u32,
    pub storage_gb: u64,
    pub overlay_gb: u64,
}

#[derive(Clone)]
pub struct Smolvm {
    service: LocalMachineService,
}

impl Smolvm {
    pub fn new() -> Result<Self> {
        Ok(Self {
            service: LocalMachineService::new().context("initialize smolvm machine service")?,
        })
    }

    pub fn status(&self, name: &str) -> Result<Option<MachineStatus>> {
        let status = self
            .service
            .status(GetMachine::new(name.to_string()))
            .with_context(|| format!("load smolvm machine '{name}'"))?;
        Ok(status.map(|status| MachineStatus {
            state: MachineState::from(status.state.to_string()),
            image: status.record.image.clone(),
        }))
    }

    pub fn create(&self, name: &str, config: &MachineConfig) -> Result<()> {
        let mut request = CreateMachine::new(name.to_string());
        request.image = Some(GUEST_IMAGE.to_string());
        request.init = vec![GUEST_INIT.into()];
        request.cmd = vec!["/usr/sbin/sshd".into(), "-D".into(), "-e".into()];
        request.mounts = vec![workspace_mount(config)?];
        request.ports = vec![ssh_port(config)?];
        request.net = true;
        request.network_backend = Some(NetworkBackend::VirtioNet);
        request.cpus = config.cpus;
        request.memory_mib = config.memory_mib;
        request.storage_gb = Some(config.storage_gb);
        request.overlay_gb = Some(config.overlay_gb);
        request.ssh_agent = true;
        self.service
            .create(request)
            .with_context(|| format!("create smolvm machine '{name}'"))?;
        Ok(())
    }

    pub fn update(&self, name: &str, config: &MachineConfig) -> Result<()> {
        let desired_mount = workspace_mount(config)?;
        let desired_port = ssh_port(config)?;
        let status = self
            .service
            .status(GetMachine::new(name.to_string()))?
            .ok_or_else(|| anyhow::anyhow!("machine '{name}' not found"))?;

        let mut request = UpdateMachine::new(name.to_string());
        let existing_mounts: Vec<_> = status.record.host_mounts();
        request.remove_mounts = existing_mounts
            .iter()
            .filter(|mount| **mount != desired_mount)
            .cloned()
            .collect();
        if !existing_mounts.contains(&desired_mount) {
            request.add_mounts.push(desired_mount);
        }

        let existing_ports: Vec<_> = status.record.port_mappings();
        request.remove_ports = existing_ports
            .iter()
            .filter(|port| **port != desired_port)
            .copied()
            .collect();
        if !existing_ports.contains(&desired_port) {
            request.add_ports.push(desired_port);
        }

        request.cpus = Some(config.cpus);
        request.memory_mib = Some(config.memory_mib);
        request.storage_gb = Some(config.storage_gb);
        request.overlay_gb = Some(config.overlay_gb);
        request.enable_network = true;
        request.network_backend = Some(NetworkBackend::VirtioNet);
        request.enable_ssh_agent = true;

        self.service
            .update(request)
            .with_context(|| format!("update smolvm machine '{name}'"))?;
        Ok(())
    }

    pub fn start(&self, name: &str) -> Result<()> {
        self.service
            .start(StartMachine::new(name.to_string()))
            .with_context(|| format!("start smolvm machine '{name}'"))?;
        Ok(())
    }

    pub fn stop(&self, name: &str) -> Result<()> {
        self.service
            .stop(StopMachine::new(name.to_string()))
            .with_context(|| format!("stop smolvm machine '{name}'"))?;
        Ok(())
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        self.service
            .delete(DeleteMachine::new(name.to_string()))
            .with_context(|| format!("delete smolvm machine '{name}'"))?;
        Ok(())
    }

    pub fn configure_ssh(&self, name: &str, authorized_keys: &PathBuf) -> Result<()> {
        let keys = fs::read_to_string(authorized_keys)
            .with_context(|| format!("read authorized keys {}", authorized_keys.display()))?;
        let script = r#"set -eu
export DEBIAN_FRONTEND=noninteractive

if ! command -v sshd >/dev/null 2>&1; then
  if command -v apk >/dev/null 2>&1; then
    apk add --no-cache \
      openssh-server openssh-client git ca-certificates curl tar gzip unzip procps libstdc++ bash
  elif command -v apt-get >/dev/null 2>&1; then
    apt-get update
    apt-get install -y --no-install-recommends \
      openssh-server git ca-certificates curl tar gzip unzip procps libstdc++6 bash
    rm -rf /var/lib/apt/lists/*
  else
    echo 'no supported package manager found for SSH bootstrap' >&2
    exit 1
  fi
fi

: "${AUTHORIZED_KEYS:?smolcoder did not provide AUTHORIZED_KEYS}"
chown root:root /root 2>/dev/null || true
chmod 700 /root 2>/dev/null || true
install -d -m 700 /root/.ssh
printf '%s\n' "$AUTHORIZED_KEYS" > /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys
unset AUTHORIZED_KEYS

ssh-keygen -A
mkdir -p /run/sshd /etc/ssh/sshd_config.d /var/empty
chown root:root /var/empty
chmod 755 /var/empty
cat > /etc/ssh/sshd_config.d/99-smolcoder.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
PermitRootLogin prohibit-password
SetEnv SSH_AUTH_SOCK=/tmp/ssh-agent.sock
EOF

if command -v pgrep >/dev/null 2>&1 && pgrep -x sshd >/dev/null 2>&1; then
  exit 0
fi

(
  exec </dev/null
  exec /usr/sbin/sshd -D -e
) >/tmp/smolcoder-sshd.log 2>&1 &

sleep 1
if command -v pgrep >/dev/null 2>&1 && pgrep -x sshd >/dev/null 2>&1; then
  exit 0
fi

cat /tmp/smolcoder-sshd.log >&2 2>/dev/null || true
exit 1
"#;
        let mut request = ExecMachine::new(
            name.to_string(),
            vec!["sh".into(), "-lc".into(), script.into()],
        );
        request.env = vec![("AUTHORIZED_KEYS".into(), keys)];
        request.timeout = Some(Duration::from_secs(600));
        request.include_record_env = false;
        let result = self
            .service
            .exec(request)
            .with_context(|| format!("bootstrap SSH inside smolvm machine '{name}'"))?;
        if result.exit_code != 0 {
            bail!(
                "SSH bootstrap failed in machine '{name}' with exit code {}: {}",
                result.exit_code,
                format_output_bytes(&result.stdout, &result.stderr)
            );
        }
        Ok(())
    }

    pub fn version(&self) -> String {
        format!("smolvm {} (machine service)", smolvm_crate::VERSION)
    }
}

fn workspace_mount(config: &MachineConfig) -> Result<HostMount> {
    HostMount::new(&config.workspace, "/workspace", false)
        .with_context(|| format!("prepare workspace mount {}", config.workspace.display()))
}

fn ssh_port(config: &MachineConfig) -> Result<PortMapping> {
    PortMapping::parse(&format!("{}:22", config.port))
        .map_err(|error| anyhow::anyhow!("invalid SSH port mapping: {error}"))
}

fn format_output_bytes(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => "no output".to_string(),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stderr}\n{stdout}"),
    }
}
