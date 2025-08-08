use crate::config::ClusterConfig;
use crate::utils::serde::{
    deserialize_i64_lenient, deserialize_i64_opt_numeric, deserialize_request_gpus,
};
use crate::utils::ssh::{parse_json_relaxed, run_remote};
use comfy_table::{
    presets::UTF8_FULL, Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Debug)]
struct JobRow {
    #[serde(rename = "ClusterId")]
    cluster_id: i64,
    #[serde(rename = "ProcId")]
    proc_id: i64,
    #[serde(rename = "Cmd")]
    cmd: Option<String>,
    #[serde(rename = "Args")]
    args: Option<String>,
    #[serde(rename = "JobPrio")]
    job_prio: i32,
    #[serde(
        rename = "RequestGPUs",
        default,
        deserialize_with = "deserialize_request_gpus"
    )]
    request_gpus: i32,
    #[serde(
        rename = "RequestMemory",
        default,
        deserialize_with = "deserialize_i64_opt_numeric"
    )]
    request_memory: Option<i64>,
    #[serde(
        rename = "MemoryProvisioned",
        default,
        deserialize_with = "deserialize_i64_opt_numeric"
    )]
    memory_provisioned: Option<i64>,
    #[serde(
        rename = "RequestCpus",
        default,
        deserialize_with = "deserialize_i64_opt_numeric"
    )]
    request_cpus: Option<i64>,
    #[serde(
        rename = "CpusProvisioned",
        default,
        deserialize_with = "deserialize_i64_opt_numeric"
    )]
    cpus_provisioned: Option<i64>,
    #[serde(
        rename = "JobStartDate",
        default,
        deserialize_with = "deserialize_i64_lenient"
    )]
    start_unix: i64,
    #[allow(dead_code)]
    #[serde(
        rename = "QDate",
        default,
        deserialize_with = "deserialize_i64_lenient"
    )]
    q_unix: i64,
}

fn price_from_prio(job_prio: i32) -> f64 {
    (job_prio + 1000) as f64
}

fn human_duration_from_unix(start: i64) -> String {
    if start <= 0 {
        return "-".to_string();
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64;
    let secs = (now - start).max(0) as u64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

fn render_table(rows: &[JobRow]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("JobID").add_attribute(Attribute::Bold),
            Cell::new("Cmd").add_attribute(Attribute::Bold),
            Cell::new("Args").add_attribute(Attribute::Bold),
            Cell::new("Runtime").add_attribute(Attribute::Bold),
            Cell::new("GPUs")
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Right),
            Cell::new("Cost")
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Right),
            Cell::new("Mem (req/used MB)").add_attribute(Attribute::Bold),
            Cell::new("CPU (req/used)").add_attribute(Attribute::Bold),
        ]);

    for j in rows {
        let jobid = format!("{}.{}", j.cluster_id, j.proc_id);
        let runtime = human_duration_from_unix(j.start_unix);
        let gpus = j.request_gpus;
        let cost = price_from_prio(j.job_prio);
        let mem_req = j
            .request_memory
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let mem_used = j
            .memory_provisioned
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let cpu_req = j
            .request_cpus
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let cpu_used = j
            .cpus_provisioned
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        table.add_row(vec![
            Cell::new(jobid).fg(Color::Green),
            Cell::new(j.cmd.as_deref().unwrap_or("")),
            Cell::new(j.args.as_deref().unwrap_or("")),
            Cell::new(runtime),
            Cell::new(gpus.to_string()).set_alignment(CellAlignment::Right),
            Cell::new(format!("{:.0}", cost)).set_alignment(CellAlignment::Right),
            Cell::new(format!("{}/{}", mem_req, mem_used)),
            Cell::new(format!("{}/{}", cpu_req, cpu_used)),
        ]);
    }

    table
}

pub fn handle_jobs() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClusterConfig::load();
    let login = match &config.login {
        Some(l) => l,
        None => {
            eprintln!("Error: No login configuration found. Run 'mct login' first.");
            std::process::exit(1);
        }
    };

    let username = config.get_username().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "No username found in login or ssh config",
        )
    })?;

    // Query current user's jobs (all states) with needed attributes
    let attrs = [
        "ClusterId",
        "ProcId",
        "Cmd",
        "Args",
        "JobPrio",
        "RequestGPUs",
        "RequestMemory",
        "MemoryProvisioned",
        "RequestCpus",
        "CpusProvisioned",
        "JobStartDate",
        "QDate",
    ]
    .join(",");
    let condor_cmd = format!("condor_q {} -json -attributes {}", username, attrs);
    let output = run_remote(login, &condor_cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("SSH command failed: {}", stderr).into());
    }

    // Optional: write to disk for debugging
    // std::fs::write("condor_q.json", &output.stdout)?;

    let jobs: Vec<JobRow> = parse_json_relaxed(&output.stdout)?;
    if jobs.is_empty() {
        println!("No jobs found.");
        return Ok(());
    }

    let table = render_table(&jobs);
    println!("{}", table);
    Ok(())
}
