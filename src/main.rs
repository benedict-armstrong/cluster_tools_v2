mod cmd;
mod config;

use clap::{Parser, Subcommand};
use cmd::handle_login;

#[derive(Parser)]
#[command(name = "cluster")]
#[command(about = "A collection of cluster management commands")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure login credentials for cluster access
    Login,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => handle_login()?,
    }

    Ok(())
}
