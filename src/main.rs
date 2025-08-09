mod cmd;
mod config;
mod utils;

use clap::{Parser, Subcommand};
use cmd::{handle_jobs, handle_list_jobs, handle_login, handle_logs, handle_price, handle_hist};

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
    Ls,
    /// Get interactive job information
    Jobs,
    /// Show historical jobs for the current user
    Hist {
        /// Limit number of historical jobs (default 10)
        #[arg(short = 'n', long = "num", default_value_t = 10)]
        num: usize,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => handle_login()?,
        Commands::Price => handle_price()?,
        Commands::Logs { selector } => handle_logs(selector)?,
        Commands::Ls => handle_list_jobs()?,
        Commands::Jobs => handle_jobs()?,
        Commands::Hist { num } => handle_hist(Some(num))?,
    }

    Ok(())
}
