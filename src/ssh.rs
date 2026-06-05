use std::{
    fs,
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone)]
pub struct AuthMaterial {
    pub authorized_keys: PathBuf,
    pub identity_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AuthOptions {
    pub authorized_keys: Option<PathBuf>,
    pub public_key: Option<PathBuf>,
    pub identity_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SshConfigSpec {
    pub host_alias: String,
    pub port: u16,
    pub known_hosts: PathBuf,
    pub identity_file: Option<PathBuf>,
}

pub fn prepare_auth_material(state_dir: &Path, options: &AuthOptions) -> Result<AuthMaterial> {
    if options.authorized_keys.is_some() && options.public_key.is_some() {
        bail!("use either --authorized-keys or --public-key, not both");
    }

    fs::create_dir_all(state_dir)
        .with_context(|| format!("create state directory {}", state_dir.display()))?;

    let (source, identity_file) = if let Some(authorized_keys) = &options.authorized_keys {
        let authorized_keys = canonical_file(authorized_keys, "authorized_keys")?;
        (
            authorized_keys,
            canonical_optional_file(options.identity_file.as_ref(), "identity file")?,
        )
    } else {
        let public_key = match &options.public_key {
            Some(path) => canonical_file(path, "public key")?,
            None => find_default_public_key(options.identity_file.as_ref())?,
        };
        let identity_file = match &options.identity_file {
            Some(path) => Some(canonical_file(path, "identity file")?),
            None => private_key_for_public_key(&public_key),
        };
        (public_key, identity_file)
    };

    let contents = fs::read_to_string(&source)
        .with_context(|| format!("read SSH key material {}", source.display()))?;
    if contents.trim().is_empty() {
        bail!("SSH key material is empty: {}", source.display());
    }

    let staged = state_dir.join("authorized_keys");
    fs::write(&staged, normalize_authorized_keys(&contents))
        .with_context(|| format!("write staged authorized_keys {}", staged.display()))?;
    set_private_file_permissions(&staged)?;

    Ok(AuthMaterial {
        authorized_keys: staged,
        identity_file,
    })
}

pub fn write_ssh_config(path: &Path, spec: &SshConfigSpec) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create SSH config directory {}", parent.display()))?;
    }
    if let Some(parent) = spec.known_hosts.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create known_hosts directory {}", parent.display()))?;
    }

    let mut config = String::new();
    config.push_str(&format!("Host {}\n", spec.host_alias));
    config.push_str("  HostName 127.0.0.1\n");
    config.push_str(&format!("  Port {}\n", spec.port));
    config.push_str("  User root\n");
    config.push_str("  StrictHostKeyChecking accept-new\n");
    config.push_str(&format!(
        "  UserKnownHostsFile {}\n",
        ssh_config_value(&spec.known_hosts)
    ));
    if let Some(identity_file) = &spec.identity_file {
        config.push_str(&format!(
            "  IdentityFile {}\n",
            ssh_config_value(identity_file)
        ));
        config.push_str("  IdentitiesOnly yes\n");
    }
    config.push_str("  ForwardX11 no\n");
    config.push_str("  ForwardAgent no\n");
    config.push_str("  ServerAliveInterval 30\n");
    config.push_str("  ServerAliveCountMax 4\n");

    fs::write(path, config).with_context(|| format!("write SSH config {}", path.display()))?;
    set_private_file_permissions(path)
}

pub fn wait_for_ssh(config: &Path, host_alias: &str, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    let mut last_error = String::new();
    let mut consecutive_successes = 0;

    while start.elapsed() < timeout {
        let output =
            ssh_ready_probe(config, host_alias).with_context(|| "run ssh readiness check")?;

        if output.status.success() {
            consecutive_successes += 1;
            if consecutive_successes >= 3 {
                return Ok(());
            }
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        consecutive_successes = 0;
        last_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        thread::sleep(Duration::from_secs(2));
    }

    bail!(
        "SSH did not become stable for host '{}' within {}s{}",
        host_alias,
        timeout.as_secs(),
        if last_error.is_empty() {
            String::new()
        } else {
            format!(": {last_error}")
        }
    )
}

fn ssh_ready_probe(config: &Path, host_alias: &str) -> std::io::Result<std::process::Output> {
    Command::new("ssh")
        .arg("-F")
        .arg(config)
        .arg("-T")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=3")
        .arg("-o")
        .arg("ConnectionAttempts=1")
        .arg(host_alias)
        .arg("uname -sm >/dev/null && test -d /workspace")
        .output()
}

pub fn choose_port(requested: Option<u16>, reusable: Option<u16>) -> Result<u16> {
    if let Some(port) = requested {
        return validate_port(port);
    }

    if let Some(port) = reusable {
        return validate_port(port);
    }

    if port_is_available(2222) {
        return Ok(2222);
    }

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .context("ask OS for a free localhost port")?;
    Ok(listener.local_addr()?.port())
}

pub fn port_is_available(port: u16) -> bool {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).is_ok()
}

fn validate_port(port: u16) -> Result<u16> {
    if port == 0 {
        bail!("port 0 is not valid for a persistent SSH forward");
    }
    Ok(port)
}

fn canonical_file(path: &Path, label: &str) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("canonicalize {label} {}", path.display()))?;
    if !canonical.is_file() {
        bail!("{label} is not a file: {}", canonical.display());
    }
    Ok(canonical)
}

fn canonical_optional_file(path: Option<&PathBuf>, label: &str) -> Result<Option<PathBuf>> {
    path.map(|path| canonical_file(path, label)).transpose()
}

fn find_default_public_key(identity_file: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(identity_file) = identity_file {
        let public = PathBuf::from(format!("{}.pub", identity_file.display()));
        if public.is_file() {
            return canonical_file(&public, "public key");
        }
        bail!(
            "could not infer public key for identity {}; pass --public-key or --authorized-keys",
            identity_file.display()
        );
    }

    let home = crate::paths::home_dir()?;
    for candidate in [
        ".ssh/id_ed25519.pub",
        ".ssh/id_ecdsa.pub",
        ".ssh/id_rsa.pub",
    ] {
        let path = home.join(candidate);
        if path.is_file() {
            return canonical_file(&path, "public key");
        }
    }

    bail!("no default SSH public key found; pass --public-key or --authorized-keys")
}

fn private_key_for_public_key(public_key: &Path) -> Option<PathBuf> {
    let value = public_key.to_string_lossy();
    value
        .strip_suffix(".pub")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

fn normalize_authorized_keys(contents: &str) -> String {
    let mut out = String::new();
    for line in contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn ssh_config_value(path: &Path) -> String {
    let value = path.to_string_lossy();
    if value.chars().any(char::is_whitespace) {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
    } else {
        value.into_owned()
    }
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("set private permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_authorized_keys_removes_blank_lines() {
        assert_eq!(
            normalize_authorized_keys("\n key1 \n\nkey2\n"),
            "key1\nkey2\n"
        );
    }

    #[test]
    fn port_zero_is_rejected() {
        assert!(choose_port(Some(0), None).is_err());
    }
}
