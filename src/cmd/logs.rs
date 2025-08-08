use crate::config::ClusterConfig;
use crate::utils::ssh::{build_path, parse_json_relaxed, run_remote, shell_escape_single_quotes};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct JobInfo {
    #[serde(rename = "ClusterId")]
    cluster_id: i32,
    #[serde(rename = "ProcId")]
    proc_id: i32,

    #[serde(rename = "Cmd")]
    cmd: Option<String>,
    #[serde(rename = "Args")]
    args: Option<String>,
    #[serde(rename = "Iwd")]
    iwd: Option<String>,
    #[serde(rename = "UserLog")]
    user_log: Option<String>,
    #[serde(rename = "Err")]
    err: Option<String>,
    #[serde(rename = "Out")]
    out: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "JobStartDate")]
    job_start_date: Option<i64>,
    #[serde(rename = "QDate")]
    q_date: Option<i64>,
}

fn parse_job_selector(selector: &str) -> Option<(i32, Option<i32>)> {
    if selector.eq_ignore_ascii_case("latest") || selector.eq_ignore_ascii_case("l") {
        return None;
    }
    if let Some((c, p)) = selector.split_once('.') {
        if let Ok(cluster_id) = c.parse::<i32>() {
            if let Ok(proc_id) = p.parse::<i32>() {
                return Some((cluster_id, Some(proc_id)));
            }
        }
    }
    if let Ok(cluster_id) = selector.parse::<i32>() {
        return Some((cluster_id, None));
    }
    None
}

pub fn handle_logs(selector: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config = ClusterConfig::load();
    let login = match &config.login {
        Some(l) => l,
        None => {
            eprintln!("Error: No login configuration found. Run 'mct login' first.");
            std::process::exit(1);
        }
    };

    // Fetch running jobs for the user
    let username = config.get_username().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "No username found in login or ssh config",
        )
    })?;

    let attrs = [
        "ClusterId",
        "ProcId",
        "Cmd",
        "Args",
        "Iwd",
        "UserLog",
        "Err",
        "Out",
        "JobStartDate",
        "QDate",
    ]
    .join(",");
    let condor_cmd = format!(
        "condor_q {} -json -attributes {} -constraint 'JobStatus==2'",
        username, attrs
    );

    let output = run_remote(login, &condor_cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query jobs via SSH: {}", stderr).into());
    }

    // if output.stdout is empty, the user is not running any jobs
    if output.stdout.is_empty() {
        println!("No running jobs found for user '{}'.", username);
        return Ok(());
    }

    let jobs: Vec<JobInfo> = parse_json_relaxed(&output.stdout)?;
    if jobs.is_empty() {
        println!("No running jobs found for user '{}'.", username);
        return Ok(());
    }

    // Select job
    let selected: &JobInfo = if let Some(sel) = selector.as_deref() {
        match parse_job_selector(sel) {
            Some((cid, maybe_pid)) => {
                let mut iter = jobs.iter().filter(|j| j.cluster_id == cid);
                if let Some(pid) = maybe_pid {
                    iter.find(|j| j.proc_id == pid).ok_or_else(|| {
                        format!("Job {}.{} not found among running jobs", cid, pid)
                    })?
                } else {
                    iter.next()
                        .ok_or_else(|| format!("Job {} not found among running jobs", cid))?
                }
            }
            None => {
                // 'latest' provided
                jobs.iter()
                    .max_by_key(|j| j.q_date.unwrap_or_default())
                    .expect("non-empty")
            }
        }
    } else {
        // default to latest
        jobs.iter()
            .max_by_key(|j| j.q_date.unwrap_or_default())
            .expect("non-empty")
    };

    let iwd = selected.iwd.as_deref().unwrap_or(".");
    let user_log = selected.user_log.as_deref().unwrap_or("");
    let out = selected.out.as_deref().unwrap_or("");
    let err = selected.err.as_deref().unwrap_or("");

    println!(
        "Showing last 50 lines for job {}.{}\nCmd: {} {}\nIwd: {}",
        selected.cluster_id,
        selected.proc_id,
        selected.cmd.as_deref().unwrap_or(""),
        selected.args.as_deref().unwrap_or(""),
        iwd
    );

    // Helper to tail a file remotely
    let show_file = |label: &str, path: &str| -> Result<(), Box<dyn std::error::Error>> {
        if path.is_empty() {
            println!("\n== {} (not set)", label);
            return Ok(());
        }
        let path_full = build_path(iwd, path);
        // shell-escape for safety
        let path_esc = shell_escape_single_quotes(&path_full);
        let cmd = format!(
            "tail -n 50 '{}' || echo '[{}] file not found: {}'",
            path_esc, label, path_full
        );
        let out = run_remote(login, &cmd)?;
        println!(
            "\n== {}: {}\n{}",
            label,
            path_full,
            String::from_utf8_lossy(&out.stdout)
        );
        Ok(())
    };

    show_file("Log", user_log)?;
    show_file("Out", out)?;
    show_file("Err", err)?;

    Ok(())
}
