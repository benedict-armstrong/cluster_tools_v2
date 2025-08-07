mod cmd;
mod config;
mod utils;

use clap::{Parser, Subcommand};
use cmd::{handle_login, handle_price};

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
    /// Analyze job prices on the cluster
    Price,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => handle_login()?,
        Commands::Price => handle_price()?,
    }

    Ok(())
}
