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
    println!("SSH config: {}", ctx.ssh_config.display());
    println!("Host alias: {}", ctx.host_alias);
    println!("Remote path: /workspace");

    if options.no_launch {
        return Ok(());
    }

    if let Some(command) = &options.command {
        return launch_intellij_command(ctx, command, &options.extra_args);
    }

    for app in macos_intellij_apps() {
        if try_open_macos_app(app, &options.extra_args)? {
            println!("Opened {app}. Use the SSH connection details above.");
            return Ok(());
        }
    }

    for binary in ["idea", "jetbrains-gateway", "gateway"] {
        if let Ok(status) = intellij_command(binary, ctx, &options.extra_args).status()
            && status.success()
        {
            return Ok(());
        }
    }

    println!("Could not launch JetBrains Gateway or IntelliJ automatically.");
    println!("Open IntelliJ IDEA manually and use the SSH config above.");
    Ok(())
}

fn launch_intellij_command(ctx: &LaunchContext, command: &str, args: &[String]) -> Result<()> {
    if cfg!(target_os = "macos") {
        let path = Path::new(command);
        if path.extension().is_some_and(|ext| ext == "app") {
            if try_open_macos_app_path(path, args)? {
                return Ok(());
            }
            bail!("could not open macOS application {}", path.display());
        }

        if command.chars().any(char::is_whitespace) {
            if try_open_macos_app(command, args)? {
                return Ok(());
            }
            bail!("could not open macOS application '{command}'");
        }
    }

    let status = intellij_command(command, ctx, args)
        .status()
        .with_context(|| format!("launch IntelliJ/Gateway with {command}"))?;
    if !status.success() {
        bail!("IntelliJ/Gateway command exited with status {status}");
    }
    Ok(())
}

fn intellij_command(command: &str, ctx: &LaunchContext, args: &[String]) -> Command {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .env("SMOLCODER_SSH_CONFIG", &ctx.ssh_config)
        .env("SMOLCODER_HOST", &ctx.host_alias)
        .env("SMOLCODER_REMOTE_PATH", "/workspace");
    cmd
}

fn macos_intellij_apps() -> &'static [&'static str] {
    &[
        "JetBrains Gateway",
        "IntelliJ IDEA",
        "IntelliJ IDEA Ultimate",
        "IntelliJ IDEA CE",
        "IntelliJ IDEA Community Edition",
    ]
}

fn try_open_macos_app(app: &str, args: &[String]) -> Result<bool> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }

    let mut command = Command::new("open");
    command
        .arg("-a")
        .arg(app)
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

fn try_open_macos_app_path(app: &Path, args: &[String]) -> Result<bool> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }

    let mut command = Command::new("open");
    command.arg(app).stdout(Stdio::null()).stderr(Stdio::null());
    if !args.is_empty() {
        command.arg("--args").args(args);
    }

    let status = command
        .status()
        .with_context(|| format!("open macOS application {}", app.display()))?;
    Ok(status.success())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intellij_app_candidates_include_idea() {
        assert!(macos_intellij_apps().contains(&"IntelliJ IDEA"));
        assert!(macos_intellij_apps().contains(&"JetBrains Gateway"));
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
