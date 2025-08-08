mod cmd;
mod config;
mod utils;

use clap::{Parser, Subcommand};
use cmd::{handle_jobs, handle_logs, handle_login, handle_price};

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
    /// Show logs/out/err for a running job (by id or latest)
    Logs {
        /// Job selector: <ClusterId>[.ProcId] or 'latest'/'l'
        selector: Option<String>,
    },
    /// List and summarize jobs in a table
    Jobs,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => handle_login()?,
        Commands::Price => handle_price()?,
        Commands::Logs { selector } => handle_logs(selector)?,
        Commands::Jobs => handle_jobs()?,
    }

    Ok(())
}
