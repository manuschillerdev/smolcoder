use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SmolfileSpec {
    pub workspace: PathBuf,
    pub authorized_keys: PathBuf,
    pub port: u16,
    pub cpus: u8,
    pub memory_mib: u32,
    pub storage_gb: u64,
    pub overlay_gb: u64,
}

pub fn render(spec: &SmolfileSpec) -> String {
    let volume = toml_string(&format!("{}:/workspace", spec.workspace.display()));
    let port = toml_string(&format!("{}:22", spec.port));
    let authorized_keys = toml_string(&spec.authorized_keys.to_string_lossy());

    format!(
        r#"# smolcoder development machine.
image = "debian:bookworm-slim"
net = true
cpus = {cpus}
memory = {memory}
storage = {storage}
overlay = {overlay}

cmd = ["sh", "-lc", '''
set -eu
export DEBIAN_FRONTEND=noninteractive

if [ ! -x /usr/sbin/sshd ]; then
  apt-get update
  apt-get install -y --no-install-recommends \
    openssh-server git ca-certificates curl tar gzip unzip procps libstdc++6 bash
  rm -rf /var/lib/apt/lists/*
fi

: "${{AUTHORIZED_KEYS:?smolcoder did not provide AUTHORIZED_KEYS}}"

install -d -m 700 /root/.ssh
printf '%s\n' "$AUTHORIZED_KEYS" > /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys
unset AUTHORIZED_KEYS

ssh-keygen -A
mkdir -p /run/sshd /etc/ssh/sshd_config.d
cat > /etc/ssh/sshd_config.d/99-smolcoder.conf <<EOF
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
PermitRootLogin prohibit-password
SetEnv SSH_AUTH_SOCK=/tmp/ssh-agent.sock
EOF

exec /usr/sbin/sshd -D -e
''']

[auth]
ssh_agent = true

[dev]
volumes = [{volume}]
ports = [{port}]

[secrets]
AUTHORIZED_KEYS = {{ from_file = {authorized_keys} }}
"#,
        cpus = spec.cpus,
        memory = spec.memory_mib,
        storage = spec.storage_gb,
        overlay = spec.overlay_gb,
        volume = volume,
        port = port,
        authorized_keys = authorized_keys,
    )
}

fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let code = ch as u32;
                if code <= 0xffff {
                    out.push_str(&format!("\\u{code:04x}"));
                } else {
                    out.push_str(&format!("\\U{code:08x}"));
                }
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_string_escapes_basic_values() {
        assert_eq!(toml_string("a\\b\"c"), r#""a\\b\"c""#);
    }

    #[test]
    fn render_uses_required_base_image_and_secret_shape() {
        let rendered = render(&SmolfileSpec {
            workspace: PathBuf::from("/repo"),
            authorized_keys: PathBuf::from("/keys/authorized_keys"),
            port: 2222,
            cpus: 4,
            memory_mib: 8192,
            storage_gb: 20,
            overlay_gb: 4,
        });

        assert!(rendered.contains("image = \"debian:bookworm-slim\""));
        assert!(rendered.contains("AUTHORIZED_KEYS = { from_file = \"/keys/authorized_keys\" }"));
        assert!(rendered.contains("ports = [\"2222:22\"]"));
        assert!(rendered.contains("ssh_agent = true"));
    }
}
