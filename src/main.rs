mod cmd;
mod config;
mod utils;

use clap::{Parser, Subcommand};
use cmd::{handle_hist, handle_jobs, handle_list_jobs, handle_login, handle_logs, handle_price};

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
        /// Show only stdout
        #[arg(short = 'o', long = "out")]
        out: bool,
        /// Show only condor user log
        #[arg(short = 'l', long = "log")]
        log: bool,
        /// Show only stderr
        #[arg(short = 'e', long = "err")]
        err: bool,
        /// Number of lines to show (default 50; if -o/-l/-e present and not set, defaults to 0). 0 = no limit
        #[arg(short = 'n', long = "lines")]
        lines: Option<i64>,
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
        Commands::Logs {
            selector,
            out,
            log,
            err,
            lines,
        } => handle_logs(selector, out, log, err, lines)?,
        Commands::Ls => handle_list_jobs()?,
        Commands::Jobs => handle_jobs()?,
        Commands::Hist { num } => handle_hist(Some(num))?,
    }

    Ok(())
}
