use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Ide {
    Code,
    Intellij,
}

#[derive(Debug, Clone)]
pub struct LaunchContext {
    pub host_alias: String,
    pub ssh_config: PathBuf,
    pub runtime_dir: PathBuf,
    pub ssh_port: u16,
    pub identity_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CodeOptions {
    pub binary: Option<PathBuf>,
    pub no_launch: bool,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IntellijOptions {
    pub command: Option<String>,
    pub no_launch: bool,
    pub extra_args: Vec<String>,
}

pub fn open_code(ctx: &LaunchContext, options: &CodeOptions) -> Result<()> {
    let user_data_dir = ctx.runtime_dir.join("code");
    let settings_path = user_data_dir.join("User/settings.json");
    write_code_settings(&settings_path, &ctx.host_alias, &ctx.ssh_config)?;

    let remote = format!("ssh-remote+{}", ctx.host_alias);
    if options.no_launch {
        println!("VS Code user-data dir: {}", user_data_dir.display());
        println!("SSH config: {}", ctx.ssh_config.display());
        println!(
            "Run: {} -n --user-data-dir {} --remote {} /workspace",
            options
                .binary
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "code".to_string()),
            user_data_dir.display(),
            remote
        );
        return Ok(());
    }

    let binary = options
        .binary
        .clone()
        .unwrap_or_else(|| PathBuf::from("code"));
    let status = Command::new(&binary)
        .args(&options.extra_args)
        .arg("-n")
        .arg("--user-data-dir")
        .arg(&user_data_dir)
        .arg("--remote")
        .arg(remote)
        .arg("/workspace")
        .status()
        .with_context(|| format!("launch VS Code with {}", binary.display()))?;

    if !status.success() {
        bail!("VS Code exited with status {status}");
    }
    Ok(())
}

pub fn open_intellij(ctx: &LaunchContext, options: &IntellijOptions) -> Result<()> {
    let identity_file = ctx.identity_file.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "IntelliJ/Gateway automation needs a private key file; rerun with --identity-file or --public-key instead of only --authorized-keys"
        )
    })?;
    let backend = find_intellij_backend()?;
    let remote_arch = remote_uname_m(ctx)?;
    let source_url = backend.download_url_for_arch(&remote_arch)?;
    let ssh_id = gateway_ssh_config_id(&ctx.host_alias);
    let gateway_config = GatewaySshConfig {
        id: ssh_id.clone(),
        name: ctx.host_alias.clone(),
        host: "127.0.0.1".to_string(),
        port: ctx.ssh_port,
        username: "root".to_string(),
        key_path: identity_file.clone(),
    };
    write_gateway_ssh_config(&gateway_config)?;

    let url = gateway_connect_url(&GatewayConnectUrl {
        ssh_id: &ssh_id,
        project_path: "/workspace",
        product_code: &backend.product_code,
        build_number: &backend.build_number,
        source_url: &source_url,
    });

    println!("Gateway URL: {url}");
    if options.no_launch {
        return Ok(());
    }

    open_gateway_url(&url, options)
}

#[derive(Debug, Clone)]
struct GatewaySshConfig {
    id: String,
    name: String,
    host: String,
    port: u16,
    username: String,
    key_path: PathBuf,
}

#[derive(Debug, Clone)]
struct GatewayConnectUrl<'a> {
    ssh_id: &'a str,
    project_path: &'a str,
    product_code: &'a str,
    build_number: &'a str,
    source_url: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JetBrainsBackend {
    product_code: String,
    build_number: String,
    version: String,
}

impl JetBrainsBackend {
    fn download_url_for_arch(&self, arch: &str) -> Result<String> {
        let prefix = match self.product_code.as_str() {
            "IU" => "ideaIU",
            "IC" => "ideaIC",
            other => bail!("unsupported IntelliJ product code '{other}'"),
        };
        let arch_suffix = match arch.trim() {
            "x86_64" | "amd64" => "",
            "aarch64" | "arm64" => "-aarch64",
            other => bail!("unsupported remote architecture '{other}' for JetBrains backend"),
        };
        Ok(format!(
            "https://download.jetbrains.com/idea/{prefix}-{}{arch_suffix}.tar.gz",
            self.version
        ))
    }
}

fn gateway_connect_url(spec: &GatewayConnectUrl<'_>) -> String {
    let params = [
        ("ssh", spec.ssh_id),
        ("projectPath", spec.project_path),
        ("deploy", "true"),
        ("type", "ssh"),
        ("productCode", spec.product_code),
        ("buildNumber", spec.build_number),
        ("sourceUrl", spec.source_url),
    ];
    let encoded = params
        .iter()
        .map(|(key, value)| format!("{key}={}", url_encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("jetbrains-gateway://connect#{encoded}")
}

fn open_gateway_url(url: &str, options: &IntellijOptions) -> Result<()> {
    if let Some(command) = &options.command {
        return launch_gateway_command(command, url, &options.extra_args);
    }

    if cfg!(target_os = "macos") {
        if try_open_macos_app_with_url("Gateway", url, &options.extra_args)?
            || try_open_macos_app_with_url("JetBrains Gateway", url, &options.extra_args)?
        {
            return Ok(());
        }
        return open_url_with_system_handler(url);
    }

    open_url_with_system_handler(url)
}

fn launch_gateway_command(command: &str, url: &str, args: &[String]) -> Result<()> {
    if cfg!(target_os = "macos") {
        let path = Path::new(command);
        if path.extension().is_some_and(|ext| ext == "app") {
            if try_open_macos_app_path_with_url(path, url, args)? {
                return Ok(());
            }
            bail!("could not open macOS application {}", path.display());
        }

        if command.chars().any(char::is_whitespace) {
            if try_open_macos_app_with_url(command, url, args)? {
                return Ok(());
            }
            bail!("could not open macOS application '{command}'");
        }
    }

    let status = Command::new(command)
        .arg(url)
        .args(args)
        .status()
        .with_context(|| format!("launch Gateway with {command}"))?;
    if !status.success() {
        bail!("Gateway command exited with status {status}");
    }
    Ok(())
}

fn open_url_with_system_handler(url: &str) -> Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    let status = command.status().context("open Gateway deep link")?;
    if !status.success() {
        bail!("opening Gateway deep link exited with status {status}");
    }
    Ok(())
}

fn try_open_macos_app_with_url(app: &str, url: &str, args: &[String]) -> Result<bool> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }

    let mut command = Command::new("open");
    command
        .arg("-a")
        .arg(app)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !args.is_empty() {
        command.arg("--args").args(args);
    }

    let status = command
        .status()
        .with_context(|| format!("open macOS application '{app}'"))?;
    Ok(status.success())
}

fn try_open_macos_app_path_with_url(app: &Path, url: &str, args: &[String]) -> Result<bool> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }

    let mut command = Command::new("open");
    command
        .arg("-a")
        .arg(app)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !args.is_empty() {
        command.arg("--args").args(args);
    }

    let status = command
        .status()
        .with_context(|| format!("open macOS application {}", app.display()))?;
    Ok(status.success())
}

fn write_gateway_ssh_config(config: &GatewaySshConfig) -> Result<()> {
    let path = gateway_ssh_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create Gateway config directory {}", parent.display()))?;
    }

    let entry = render_gateway_ssh_config(config);
    let next = if path.exists() {
        merge_gateway_ssh_config(&fs::read_to_string(&path)?, &config.id, &entry)
    } else {
        format!(
            "<application>\n  <component name=\"SshConfigs\">\n    <configs>\n{entry}    </configs>\n  </component>\n</application>\n"
        )
    };

    fs::write(&path, next).with_context(|| format!("write Gateway SSH config {}", path.display()))
}

fn gateway_ssh_config_path() -> Result<PathBuf> {
    Ok(jetbrains_config_dir()?.join("options/sshConfigs.xml"))
}

fn jetbrains_config_dir() -> Result<PathBuf> {
    let base = jetbrains_config_base()?;
    let mut candidates = Vec::new();
    if base.exists() {
        for entry in fs::read_dir(&base).with_context(|| format!("read {}", base.display()))? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("JetBrainsGateway") || name.starts_with("Gateway") {
                candidates.push(entry.path());
            }
        }
    }
    candidates.sort();
    if let Some(path) = candidates.pop() {
        return Ok(path);
    }

    if cfg!(target_os = "macos")
        && let Some(version) = gateway_short_version()?
    {
        let major_minor = version.split('.').take(2).collect::<Vec<_>>().join(".");
        return Ok(base.join(format!("JetBrainsGateway{major_minor}")));
    }

    bail!(
        "could not find JetBrains Gateway config directory under {}; start Gateway once and retry",
        base.display()
    )
}

fn jetbrains_config_base() -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        Ok(crate::paths::home_dir()?.join("Library/Application Support/JetBrains"))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("JetBrains"))
            .ok_or_else(|| anyhow::anyhow!("APPDATA is not set"))
    } else {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or(crate::paths::home_dir()?.join(".config"));
        Ok(base.join("JetBrains"))
    }
}

fn render_gateway_ssh_config(config: &GatewaySshConfig) -> String {
    format!(
        "      <sshConfig id=\"{}\" host=\"{}\" port=\"{}\" username=\"{}\" authType=\"KEY_PAIR\" keyPath=\"{}\" nameFormat=\"CUSTOM\" customName=\"{}\" useOpenSSHConfig=\"false\" />\n",
        xml_escape(&config.id),
        xml_escape(&config.host),
        config.port,
        xml_escape(&config.username),
        xml_escape(&config.key_path.to_string_lossy()),
        xml_escape(&config.name),
    )
}

fn merge_gateway_ssh_config(existing: &str, id: &str, entry: &str) -> String {
    let marker = format!("id=\"{}\"", xml_escape(id));
    let without_existing = existing
        .lines()
        .filter(|line| !line.contains("<sshConfig ") || !line.contains(&marker))
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(index) = without_existing.rfind("    </configs>") {
        let mut out = without_existing;
        out.insert_str(index, entry);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }

    format!(
        "<application>\n  <component name=\"SshConfigs\">\n    <configs>\n{entry}    </configs>\n  </component>\n</application>\n"
    )
}

fn gateway_ssh_config_id(host_alias: &str) -> String {
    host_alias.to_string()
}

fn remote_uname_m(ctx: &LaunchContext) -> Result<String> {
    let output = Command::new("ssh")
        .arg("-F")
        .arg(&ctx.ssh_config)
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(&ctx.host_alias)
        .arg("uname")
        .arg("-m")
        .output()
        .context("query remote architecture")?;
    if !output.status.success() {
        bail!(
            "query remote architecture failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn find_intellij_backend() -> Result<JetBrainsBackend> {
    for app in find_intellij_apps()? {
        if let Some(info) = read_intellij_info(&app)? {
            return Ok(info);
        }
    }
    bail!("could not find IntelliJ IDEA.app; install IntelliJ IDEA or pass a Gateway URL manually")
}

fn find_intellij_apps() -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    if cfg!(target_os = "macos") {
        roots.push(PathBuf::from("/Applications"));
        roots.push(crate::paths::home_dir()?.join("Applications"));
    }

    let mut apps = Vec::new();
    for root in roots {
        for name in [
            "IntelliJ IDEA.app",
            "IntelliJ IDEA Ultimate.app",
            "IntelliJ IDEA CE.app",
            "IntelliJ IDEA Community Edition.app",
        ] {
            let app = root.join(name);
            if app.exists() {
                apps.push(app);
            }
        }
    }
    Ok(apps)
}

fn read_intellij_info(app: &Path) -> Result<Option<JetBrainsBackend>> {
    if !cfg!(target_os = "macos") {
        return Ok(None);
    }
    let plist = app.join("Contents/Info.plist");
    if !plist.exists() {
        return Ok(None);
    }

    let bundle_version = plutil_extract(&plist, "CFBundleVersion")?;
    let short_version = plutil_extract(&plist, "CFBundleShortVersionString")?;
    let Some((product_code, build_number)) = bundle_version.split_once('-') else {
        return Ok(None);
    };

    if product_code != "IU" && product_code != "IC" {
        return Ok(None);
    }

    Ok(Some(JetBrainsBackend {
        product_code: product_code.to_string(),
        build_number: build_number.to_string(),
        version: short_version,
    }))
}

fn gateway_short_version() -> Result<Option<String>> {
    if !cfg!(target_os = "macos") {
        return Ok(None);
    }
    for app in [
        crate::paths::home_dir()?.join("Applications/Gateway.app"),
        PathBuf::from("/Applications/Gateway.app"),
        crate::paths::home_dir()?.join("Applications/JetBrains Gateway.app"),
        PathBuf::from("/Applications/JetBrains Gateway.app"),
    ] {
        let plist = app.join("Contents/Info.plist");
        if plist.exists() {
            return Ok(Some(plutil_extract(&plist, "CFBundleShortVersionString")?));
        }
    }
    Ok(None)
}

fn plutil_extract(plist: &Path, key: &str) -> Result<String> {
    let output = Command::new("plutil")
        .arg("-extract")
        .arg(key)
        .arg("raw")
        .arg("-o")
        .arg("-")
        .arg(plist)
        .output()
        .with_context(|| format!("read {key} from {}", plist.display()))?;
    if !output.status.success() {
        bail!(
            "plutil failed for {}: {}",
            plist.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_code_settings(path: &Path, host_alias: &str, ssh_config: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create VS Code settings directory {}", parent.display()))?;
    }

    let mut remote_platform = Map::new();
    remote_platform.insert(host_alias.to_string(), Value::String("linux".to_string()));

    let settings = json!({
        "remote.SSH.enableDynamicForwarding": false,
        "remote.SSH.useExecServer": false,
        "remote.SSH.configFile": ssh_config.to_string_lossy(),
        "remote.SSH.remotePlatform": remote_platform,
    });

    let data = serde_json::to_string_pretty(&settings).context("serialize VS Code settings")?;
    fs::write(path, format!("{data}\n"))
        .with_context(|| format!("write VS Code settings {}", path.display()))
}

fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_url_is_deploy_deep_link() {
        let url = gateway_connect_url(&GatewayConnectUrl {
            ssh_id: "smolcoder-test",
            project_path: "/workspace",
            product_code: "IU",
            build_number: "253.1",
            source_url: "https://download.jetbrains.com/idea/ideaIU-2025.3.4-aarch64.tar.gz",
        });
        assert!(url.starts_with("jetbrains-gateway://connect#"));
        assert!(url.contains("ssh=smolcoder-test"));
        assert!(url.contains("projectPath=%2Fworkspace"));
        assert!(url.contains("deploy=true"));
        assert!(url.contains("sourceUrl=https%3A%2F%2Fdownload.jetbrains.com"));
    }

    #[test]
    fn gateway_ssh_config_entry_is_stable() {
        let rendered = render_gateway_ssh_config(&GatewaySshConfig {
            id: "smolcoder-test".to_string(),
            name: "smolcoder-test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 2222,
            username: "root".to_string(),
            key_path: PathBuf::from("/Users/me/.ssh/id_ed25519"),
        });
        assert!(rendered.contains("<sshConfig "));
        assert!(rendered.contains("id=\"smolcoder-test\""));
        assert!(rendered.contains("authType=\"KEY_PAIR\""));
        assert!(rendered.contains("keyPath=\"/Users/me/.ssh/id_ed25519\""));
    }

    #[test]
    fn backend_download_url_tracks_arch() {
        let backend = JetBrainsBackend {
            product_code: "IU".to_string(),
            build_number: "253.1".to_string(),
            version: "2025.3.4".to_string(),
        };
        assert_eq!(
            backend.download_url_for_arch("x86_64").unwrap(),
            "https://download.jetbrains.com/idea/ideaIU-2025.3.4.tar.gz"
        );
        assert_eq!(
            backend.download_url_for_arch("aarch64").unwrap(),
            "https://download.jetbrains.com/idea/ideaIU-2025.3.4-aarch64.tar.gz"
        );
    }

    #[test]
    fn code_settings_include_remote_ssh_workarounds() {
        let temp = std::env::temp_dir().join(format!(
            "smolcoder-settings-test-{}-{}.json",
            std::process::id(),
            1
        ));
        write_code_settings(
            &temp,
            "smolcoder-test",
            Path::new("/tmp/smolcoder/ssh_config"),
        )
        .unwrap();
        let data = fs::read_to_string(&temp).unwrap();
        let parsed: Value = serde_json::from_str(&data).unwrap();
        assert_eq!(parsed["remote.SSH.enableDynamicForwarding"], false);
        assert_eq!(parsed["remote.SSH.useExecServer"], false);
        assert!(parsed.get("remote.SSH.useLocalServer").is_none());
        let _ = fs::remove_file(temp);
    }
}
