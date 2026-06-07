use std::path::PathBuf;

use clap::ValueEnum;

mod code;
mod jetbrains;

pub use code::{CodeOptions, open_code};
pub use jetbrains::{IntellijOptions, open_intellij};

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
