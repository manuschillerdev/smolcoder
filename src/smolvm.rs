use std::path::PathBuf;

use ::smolvm as smolvm_crate;
use anyhow::{Context, Result};
use smolvm_crate::{
    HostMount, RecordState,
    data::network::PortMapping,
    machine::{
        CreateMachine, DeleteMachine, GetMachine, LocalMachineService, MachineService,
        StartMachine, StopMachine, UpdateMachine,
    },
    network::NetworkBackend,
};

#[derive(Debug, Clone)]
pub struct MachineStatus {
    pub state: RecordState,
    pub image: Option<String>,
    pub profile_version: Option<String>,
}

impl MachineStatus {
    pub fn is_running(&self) -> bool {
        self.state == RecordState::Running
    }

    pub fn is_compatible(&self) -> bool {
        self.image.as_deref() == Some(GUEST_IMAGE)
            && self.profile_version.as_deref() == Some(GUEST_PROFILE_VERSION)
    }
}

pub const GUEST_IMAGE: &str = "debian:bookworm-slim";
const GUEST_PROFILE_ENV: &str = "SMOLCODER_GUEST_PROFILE";
const GUEST_PROFILE_VERSION: &str = "1";

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
: "${AUTHORIZED_KEYS:?smolcoder did not provide AUTHORIZED_KEYS}"
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
"#;

#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub workspace: PathBuf,
    pub port: u16,
    pub cpus: u8,
    pub memory_mib: u32,
    pub storage_gb: u64,
    pub overlay_gb: u64,
    pub authorized_keys: PathBuf,
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
            state: status.state,
            image: status.record.image.clone(),
            profile_version: status
                .record
                .env
                .iter()
                .find_map(|(key, value)| (key == GUEST_PROFILE_ENV).then(|| value.clone())),
        }))
    }

    pub fn create(&self, name: &str, config: &MachineConfig) -> Result<()> {
        let mut request = CreateMachine::new(name.to_string());
        request.image = Some(GUEST_IMAGE.to_string());
        request.init = vec![GUEST_INIT.into()];
        request.cmd = vec!["/usr/sbin/sshd".into(), "-D".into(), "-e".into()];
        request.env = vec![format!("{GUEST_PROFILE_ENV}={GUEST_PROFILE_VERSION}")];
        request.secret_refs.insert(
            "AUTHORIZED_KEYS".into(),
            smolvm_crate::secrets::file_ref(config.authorized_keys.clone()),
        );
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
        request
            .set_env
            .push((GUEST_PROFILE_ENV.into(), GUEST_PROFILE_VERSION.into()));

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
