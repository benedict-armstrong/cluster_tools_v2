use crate::config::ClusterConfig;
use crate::utils::serde::deserialize_request_gpus;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct Job {
    #[serde(rename = "JobPrio")]
    job_prio: i32,
    #[serde(rename = "JobStatus")]
    job_status: i32,
    #[serde(
        rename = "RequestGPUs",
        default,
        deserialize_with = "deserialize_request_gpus"
    )]
    request_gpus: i32,
}

#[derive(Debug)]
struct PriceStats {
    total_jobs: usize,
    idle_jobs: usize,
    running_jobs: usize,
    avg_price: f64,
    avg_idle_price: f64,
    avg_running_price: f64,
}

impl PriceStats {
    fn new() -> Self {
        Self {
            total_jobs: 0,
            idle_jobs: 0,
            running_jobs: 0,
            avg_price: 0.0,
            avg_idle_price: 0.0,
            avg_running_price: 0.0,
        }
    }
}

fn job_prio_to_price(job_prio: i32) -> f64 {
    // Convert JobPrio range [-1000, 1000] to price range [0, 2000]
    (job_prio + 1000) as f64
}

fn create_combined_stats_table(gpu_stats: &PriceStats, cpu_stats: &PriceStats) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Job Type").add_attribute(Attribute::Bold),
            Cell::new("Status").add_attribute(Attribute::Bold),
            Cell::new("Count")
                .add_attribute(Attribute::Bold)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new("Average Price")
                .add_attribute(Attribute::Bold)
                .set_alignment(comfy_table::CellAlignment::Right),
        ]);

    // GPU Jobs
    if gpu_stats.total_jobs > 0 {
        table.add_row(vec![
            Cell::new("GPU").fg(Color::Green),
            Cell::new("Total"),
            Cell::new(gpu_stats.total_jobs.to_string())
                .fg(Color::Green)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(format!("{:.2}", gpu_stats.avg_price))
                .set_alignment(comfy_table::CellAlignment::Right),
        ]);

        table.add_row(vec![
            Cell::new(""),
            Cell::new("Idle"),
            Cell::new(gpu_stats.idle_jobs.to_string())
                .fg(Color::Blue)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(if gpu_stats.idle_jobs > 0 {
                format!("{:.2}", gpu_stats.avg_idle_price)
            } else {
                "N/A".to_string()
            })
            .set_alignment(comfy_table::CellAlignment::Right),
        ]);

        table.add_row(vec![
            Cell::new(""),
            Cell::new("Running"),
            Cell::new(gpu_stats.running_jobs.to_string())
                .fg(Color::Magenta)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(if gpu_stats.running_jobs > 0 {
                format!("{:.2}", gpu_stats.avg_running_price)
            } else {
                "N/A".to_string()
            })
            .set_alignment(comfy_table::CellAlignment::Right),
        ]);
    } else {
        table.add_row(vec![
            Cell::new("GPU").fg(Color::Green),
            Cell::new("No jobs found"),
            Cell::new("-")
                .fg(Color::DarkGrey)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new("-")
                .fg(Color::DarkGrey)
                .set_alignment(comfy_table::CellAlignment::Right),
        ]);
    }

    // CPU Jobs
    if cpu_stats.total_jobs > 0 {
        table.add_row(vec![
            Cell::new("CPU").fg(Color::Blue),
            Cell::new("Total"),
            Cell::new(cpu_stats.total_jobs.to_string())
                .fg(Color::Green)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(format!("{:.2}", cpu_stats.avg_price))
                .set_alignment(comfy_table::CellAlignment::Right),
        ]);

        table.add_row(vec![
            Cell::new(""),
            Cell::new("Idle"),
            Cell::new(cpu_stats.idle_jobs.to_string())
                .fg(Color::Blue)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(if cpu_stats.idle_jobs > 0 {
                format!("{:.2}", cpu_stats.avg_idle_price)
            } else {
                "N/A".to_string()
            })
            .set_alignment(comfy_table::CellAlignment::Right),
        ]);

        table.add_row(vec![
            Cell::new(""),
            Cell::new("Running"),
            Cell::new(cpu_stats.running_jobs.to_string())
                .fg(Color::Magenta)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(if cpu_stats.running_jobs > 0 {
                format!("{:.2}", cpu_stats.avg_running_price)
            } else {
                "N/A".to_string()
            })
            .set_alignment(comfy_table::CellAlignment::Right),
        ]);
    } else {
        table.add_row(vec![
            Cell::new("CPU").fg(Color::Blue),
            Cell::new("No jobs found"),
            Cell::new("-")
                .fg(Color::DarkGrey)
                .set_alignment(comfy_table::CellAlignment::Right),
            Cell::new("-")
                .fg(Color::DarkGrey)
                .set_alignment(comfy_table::CellAlignment::Right),
        ]);
    }

    table
}

fn calculate_stats(jobs: &[Job], has_gpu: bool) -> PriceStats {
    let filtered_jobs: Vec<&Job> = jobs
        .iter()
        .filter(|job| (job.request_gpus > 0) == has_gpu)
        .collect();

    if filtered_jobs.is_empty() {
        return PriceStats::new();
    }

    let idle_jobs: Vec<&Job> = filtered_jobs
        .iter()
        .filter(|job| job.job_status == 1) // Idle
        .copied()
        .collect();

    let running_jobs: Vec<&Job> = filtered_jobs
        .iter()
        .filter(|job| job.job_status == 2) // Running
        .copied()
        .collect();

    let total_price: f64 = filtered_jobs
        .iter()
        .map(|job| job_prio_to_price(job.job_prio))
        .sum();

    let idle_price: f64 = idle_jobs
        .iter()
        .map(|job| job_prio_to_price(job.job_prio))
        .sum();

    let running_price: f64 = running_jobs
        .iter()
        .map(|job| job_prio_to_price(job.job_prio))
        .sum();

    PriceStats {
        total_jobs: filtered_jobs.len(),
        idle_jobs: idle_jobs.len(),
        running_jobs: running_jobs.len(),
        avg_price: if filtered_jobs.is_empty() {
            0.0
        } else {
            total_price / filtered_jobs.len() as f64
        },
        avg_idle_price: if idle_jobs.is_empty() {
            0.0
        } else {
            idle_price / idle_jobs.len() as f64
        },
        avg_running_price: if running_jobs.is_empty() {
            0.0
        } else {
            running_price / running_jobs.len() as f64
        },
    }
}

fn build_ssh_command(config: &ClusterConfig) -> Vec<String> {
    let login_config = config.login.as_ref().unwrap();
    let mut ssh_args = vec!["ssh".to_string()];

    if let Some(ssh_config_name) = &login_config.ssh_config_name {
        // Use SSH config
        ssh_args.push(ssh_config_name.clone());
    } else {
        // Manual configuration
        if let Some(identity_file) = &login_config.identity_file {
            ssh_args.push("-i".to_string());
            ssh_args.push(identity_file.clone());
        }
        ssh_args.push(format!(
            "{}@{}",
            login_config.username, login_config.hostname
        ));
    }

    ssh_args.push("condor_q".to_string());
    ssh_args.push("-json".to_string());
    ssh_args.push("-attributes".to_string());
    ssh_args.push("JobPrio,JobStatus,RequestGPUs".to_string());

    ssh_args
}

pub fn handle_price() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClusterConfig::load();

    // Check if login configuration exists
    if config.login.is_none() {
        eprintln!("Error: No login configuration found.");
        eprintln!("Please run 'cluster login' first to configure your connection settings.");
        std::process::exit(1);
    }

    println!("Connecting to cluster and fetching job data...");

    // Build SSH command
    let ssh_args = build_ssh_command(&config);

    // Execute SSH command
    let output = Command::new(&ssh_args[0])
        .args(&ssh_args[1..])
        .output()
        .map_err(|e| format!("Failed to execute SSH command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("SSH command failed: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // save stdout to file
    // std::fs::write("stdout.txt", stdout.as_bytes())?;

    // Parse JSON response
    let jobs: Vec<Job> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    if jobs.is_empty() {
        println!("No jobs found in the queue.");
        return Ok(());
    }

    // Calculate statistics
    let gpu_stats = calculate_stats(&jobs, true);
    let no_gpu_stats = calculate_stats(&jobs, false);

    let combined_table = create_combined_stats_table(&gpu_stats, &no_gpu_stats);
    println!("{}", combined_table);

    Ok(())
}
