use std::{fs, path::PathBuf, process::Command, time::Duration};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};

use crate::{
    ide::{self, CodeOptions, Ide, IntellijOptions, LaunchContext},
    paths,
    smolfile::{self, SmolfileSpec},
    smolvm::{MachineUpdate, Smolvm},
    ssh::{self, AuthOptions, SshConfigSpec},
    state::WorkspaceState,
};

#[derive(Debug, Parser)]
#[command(
    name = "smolcoder",
    version,
    about = "Open a smolvm-backed remote SSH coding machine"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "smolvm", value_name = "BIN")]
    smolvm: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create/start the machine and open an IDE.
    Open(OpenCmd),
    /// Create/start the machine without launching an IDE.
    Ensure(EnsureCmd),
    /// Print machine and SSH connection status.
    Status(TargetCmd),
    /// Stop the smolcoder machine.
    Stop(TargetCmd),
    /// Delete the smolcoder machine and local wrapper state.
    Delete(TargetCmd),
    /// Print the generated SSH config path, creating it if needed.
    SshConfig(EnsureCmd),
    /// Check required host tools.
    Doctor,
}

#[derive(Debug, Args)]
struct OpenCmd {
    #[arg(long, value_enum, default_value_t = Ide::Code)]
    ide: Ide,

    #[command(flatten)]
    machine: MachineCmd,

    #[arg(long, value_name = "BIN")]
    code: Option<PathBuf>,

    #[arg(long, value_name = "BIN_OR_APP")]
    intellij: Option<String>,

    #[arg(long)]
    no_launch: bool,

    #[arg(last = true)]
    ide_args: Vec<String>,
}

#[derive(Debug, Args)]
struct EnsureCmd {
    #[command(flatten)]
    machine: MachineCmd,
}

#[derive(Debug, Args)]
struct TargetCmd {
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,

    #[arg(long, value_name = "NAME")]
    name: Option<String>,
}

#[derive(Debug, Clone, Args)]
struct MachineCmd {
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,

    #[arg(long, value_name = "NAME")]
    name: Option<String>,

    #[arg(long, value_name = "PORT")]
    port: Option<u16>,

    #[arg(long, value_name = "PATH")]
    authorized_keys: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    public_key: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    identity_file: Option<PathBuf>,

    #[arg(long, default_value_t = 4)]
    cpus: u8,

    #[arg(
        long = "mem",
        alias = "memory",
        default_value_t = 8192,
        value_name = "MiB"
    )]
    memory_mib: u32,

    #[arg(long, default_value_t = 40, value_name = "GiB")]
    storage_gb: u64,

    #[arg(long, default_value_t = 8, value_name = "GiB")]
    overlay_gb: u64,

    #[arg(long)]
    recreate: bool,

    #[arg(long, default_value_t = 90, value_name = "SECONDS")]
    ssh_timeout: u64,
}

#[derive(Debug, Clone)]
struct MachineContext {
    state: WorkspaceState,
    state_path: PathBuf,
    runtime_dir: PathBuf,
    ssh_config: PathBuf,
}

pub fn run(cli: Cli) -> Result<()> {
    let smolvm = Smolvm::new(cli.smolvm);

    match cli.command {
        Commands::Open(cmd) => {
            let ctx = ensure_machine(&smolvm, &cmd.machine)?;
            let launch = LaunchContext {
                host_alias: ctx.state.host_alias.clone(),
                ssh_config: ctx.ssh_config.clone(),
                runtime_dir: ctx.runtime_dir.clone(),
            };
            match cmd.ide {
                Ide::Code => ide::open_code(
                    &launch,
                    &CodeOptions {
                        binary: cmd.code,
                        no_launch: cmd.no_launch,
                        extra_args: cmd.ide_args,
                    },
                ),
                Ide::Intellij => ide::open_intellij(
                    &launch,
                    &IntellijOptions {
                        command: cmd.intellij,
                        no_launch: cmd.no_launch,
                        extra_args: cmd.ide_args,
                    },
                ),
            }
        }
        Commands::Ensure(cmd) => {
            let ctx = ensure_machine(&smolvm, &cmd.machine)?;
            print_ready(&ctx);
            Ok(())
        }
        Commands::Status(cmd) => print_status(&smolvm, &cmd),
        Commands::Stop(cmd) => stop_machine(&smolvm, &cmd),
        Commands::Delete(cmd) => delete_machine(&smolvm, &cmd),
        Commands::SshConfig(cmd) => {
            let ctx = ensure_machine(&smolvm, &cmd.machine)?;
            println!("{}", ctx.ssh_config.display());
            Ok(())
        }
        Commands::Doctor => doctor(&smolvm),
    }
}

fn ensure_machine(smolvm: &Smolvm, opts: &MachineCmd) -> Result<MachineContext> {
    let workspace = paths::canonical_workspace(&opts.workspace)?;
    let workspace_id = paths::workspace_id(&workspace);
    let state_dir = paths::workspace_state_dir(&workspace_id)?;
    let state_path = state_dir.join("state.json");
    fs::create_dir_all(&state_dir)
        .with_context(|| format!("create state directory {}", state_dir.display()))?;

    let previous = WorkspaceState::load(&state_path)?;
    let machine = opts
        .name
        .clone()
        .or_else(|| previous.as_ref().map(|state| state.machine.clone()))
        .unwrap_or_else(|| paths::default_machine_name(&workspace));

    let mut status = if opts.recreate {
        None
    } else {
        smolvm.status(&machine)?
    };

    if status.is_some()
        && previous
            .as_ref()
            .is_none_or(|state| state.machine != machine || state.workspace_id != workspace_id)
        && !opts.recreate
    {
        bail!(
            "machine '{}' already exists but smolcoder state for this workspace is missing or points elsewhere; pass --recreate to rebuild it",
            machine
        );
    }

    if opts.recreate {
        if smolvm.status(&machine)?.is_some() {
            let _ = smolvm.stop(&machine);
            smolvm.delete(&machine)?;
        }
        status = None;
    }

    let reusable_port = previous
        .as_ref()
        .filter(|state| state.machine == machine)
        .map(|state| state.port);
    let status_is_running = status
        .as_ref()
        .is_some_and(|machine_status| machine_status.state.is_running());
    let mut port = ssh::choose_port(opts.port, reusable_port)?;

    if !status_is_running && !ssh::port_is_available(port) {
        if opts.port.is_some() {
            bail!("requested SSH port {port} is already in use on 127.0.0.1");
        }
        port = ssh::choose_port(None, None)?;
    }

    let auth = ssh::prepare_auth_material(
        &state_dir,
        &AuthOptions {
            authorized_keys: opts.authorized_keys.clone(),
            public_key: opts.public_key.clone(),
            identity_file: opts.identity_file.clone(),
        },
    )?;

    let smolfile_path = state_dir.join("Smolfile");
    let desired = WorkspaceState {
        workspace_id: workspace_id.clone(),
        workspace: workspace.clone(),
        machine: machine.clone(),
        host_alias: machine.clone(),
        port,
        authorized_keys: auth.authorized_keys.clone(),
        identity_file: auth.identity_file.clone(),
        smolfile: smolfile_path.clone(),
        cpus: opts.cpus,
        memory_mib: opts.memory_mib,
        storage_gb: opts.storage_gb,
        overlay_gb: opts.overlay_gb,
    };

    let rendered = smolfile::render(&SmolfileSpec {
        workspace: desired.workspace.clone(),
        authorized_keys: desired.authorized_keys.clone(),
        port: desired.port,
        cpus: desired.cpus,
        memory_mib: desired.memory_mib,
        storage_gb: desired.storage_gb,
        overlay_gb: desired.overlay_gb,
    });
    fs::write(&smolfile_path, rendered)
        .with_context(|| format!("write Smolfile {}", smolfile_path.display()))?;

    match &status {
        None => {
            smolvm.create(&machine, &smolfile_path)?;
            status = smolvm.status(&machine)?;
        }
        Some(machine_status) if !machine_status.state.is_running() => {
            if let Some(previous) = previous.as_ref().filter(|state| state.machine == machine)
                && previous.needs_machine_update(&desired)
            {
                smolvm.update(&machine, &update_from_states(previous, &desired))?;
                status = smolvm.status(&machine)?;
            }
        }
        Some(_) => {
            if let Some(previous) = previous.as_ref().filter(|state| state.machine == machine)
                && previous.needs_machine_update(&desired)
            {
                bail!(
                    "machine '{}' is running with older settings; run `smolcoder stop` first or use --recreate",
                    machine
                );
            }
        }
    }

    desired.save(&state_path)?;

    if !status
        .as_ref()
        .is_some_and(|machine_status| machine_status.state.is_running())
    {
        smolvm.start(&machine)?;
    }

    let runtime_dir = paths::runtime_dir(&workspace_id);
    if runtime_dir.exists() {
        fs::remove_dir_all(&runtime_dir)
            .with_context(|| format!("clear runtime directory {}", runtime_dir.display()))?;
    }
    fs::create_dir_all(&runtime_dir)
        .with_context(|| format!("create runtime directory {}", runtime_dir.display()))?;

    let ssh_config = runtime_dir.join("ssh_config");
    let known_hosts = runtime_dir.join("known_hosts");
    ssh::write_ssh_config(
        &ssh_config,
        &SshConfigSpec {
            host_alias: desired.host_alias.clone(),
            port: desired.port,
            known_hosts,
            identity_file: desired.identity_file.clone(),
        },
    )?;

    ssh::wait_for_ssh(
        &ssh_config,
        &desired.host_alias,
        Duration::from_secs(opts.ssh_timeout),
    )?;

    Ok(MachineContext {
        state: desired,
        state_path,
        runtime_dir,
        ssh_config,
    })
}

fn update_from_states(previous: &WorkspaceState, desired: &WorkspaceState) -> MachineUpdate {
    MachineUpdate {
        remove_volume: (previous.workspace != desired.workspace)
            .then(|| smolfile::volume_spec(&previous.workspace)),
        volume: smolfile::volume_spec(&desired.workspace),
        remove_port: (previous.port != desired.port).then(|| smolfile::port_spec(previous.port)),
        port: smolfile::port_spec(desired.port),
        cpus: (previous.cpus != desired.cpus).then_some(desired.cpus),
        memory_mib: (previous.memory_mib != desired.memory_mib).then_some(desired.memory_mib),
        storage_gb: (previous.storage_gb != desired.storage_gb).then_some(desired.storage_gb),
        overlay_gb: (previous.overlay_gb != desired.overlay_gb).then_some(desired.overlay_gb),
    }
}

fn resolve_target(cmd: &TargetCmd) -> Result<(String, PathBuf, PathBuf, Option<WorkspaceState>)> {
    let workspace = paths::canonical_workspace(&cmd.workspace)?;
    let workspace_id = paths::workspace_id(&workspace);
    let state_dir = paths::workspace_state_dir(&workspace_id)?;
    let state_path = state_dir.join("state.json");
    let state = WorkspaceState::load(&state_path)?;
    let machine = cmd
        .name
        .clone()
        .or_else(|| state.as_ref().map(|state| state.machine.clone()))
        .unwrap_or_else(|| paths::default_machine_name(&workspace));
    Ok((machine, state_dir, state_path, state))
}

fn print_ready(ctx: &MachineContext) {
    println!("Machine: {}", ctx.state.machine);
    println!("SSH: root@127.0.0.1:{}", ctx.state.port);
    println!("SSH config: {}", ctx.ssh_config.display());
    println!("State: {}", ctx.state_path.display());
}

fn print_status(smolvm: &Smolvm, cmd: &TargetCmd) -> Result<()> {
    let (machine, state_dir, state_path, state) = resolve_target(cmd)?;
    let status = smolvm.status(&machine)?;

    println!("Machine: {}", machine);
    println!(
        "State: {}",
        status
            .as_ref()
            .map(|status| status.state.as_str())
            .unwrap_or("missing")
    );
    if let Some(state) = state {
        println!("Workspace: {}", state.workspace.display());
        println!("SSH: root@127.0.0.1:{}", state.port);
        println!(
            "SSH config: {}",
            paths::runtime_dir(&state.workspace_id)
                .join("ssh_config")
                .display()
        );
        println!("State file: {}", state_path.display());
    } else {
        println!("State dir: {}", state_dir.display());
    }
    Ok(())
}

fn stop_machine(smolvm: &Smolvm, cmd: &TargetCmd) -> Result<()> {
    let (machine, _, _, _) = resolve_target(cmd)?;
    match smolvm.status(&machine)? {
        Some(status) if status.state.is_running() => {
            smolvm.stop(&machine)?;
            println!("Stopped {}", machine);
        }
        Some(status) => println!("Machine {} is {}", machine, status.state.as_str()),
        None => println!("Machine {} is missing", machine),
    }
    Ok(())
}

fn delete_machine(smolvm: &Smolvm, cmd: &TargetCmd) -> Result<()> {
    let (machine, state_dir, _, _) = resolve_target(cmd)?;
    if smolvm.status(&machine)?.is_some() {
        smolvm.delete(&machine)?;
        println!("Deleted {}", machine);
    } else {
        println!("Machine {} is missing", machine);
    }

    if state_dir.exists() {
        fs::remove_dir_all(&state_dir)
            .with_context(|| format!("remove state directory {}", state_dir.display()))?;
    }
    Ok(())
}

fn doctor(smolvm: &Smolvm) -> Result<()> {
    println!("state root: {}", paths::state_root()?.display());
    println!("runtime root: /tmp/smolcoder");

    match smolvm.version() {
        Ok(version) => println!("smolvm: {version}"),
        Err(error) => println!("smolvm: missing or unusable ({error:#})"),
    }

    match Command::new("ssh").arg("-V").output() {
        Ok(output) => {
            let version = if output.stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            };
            println!("ssh: {version}");
        }
        Err(error) => println!("ssh: missing ({error})"),
    }

    match Command::new("code").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("installed")
                .to_string();
            println!("code: {version}");
        }
        _ => println!("code: not found in PATH"),
    }

    Ok(())
}
