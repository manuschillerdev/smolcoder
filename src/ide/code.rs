use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use super::LaunchContext;

#[derive(Debug, Clone)]
pub struct CodeOptions {
    pub binary: Option<PathBuf>,
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
