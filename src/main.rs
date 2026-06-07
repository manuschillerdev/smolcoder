mod app;
mod ide;
mod paths;
mod smolvm;
mod ssh;
mod state;

use clap::Parser;

fn main() {
    let cli = app::Cli::parse();

    if let Err(error) = app::run(cli) {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}
