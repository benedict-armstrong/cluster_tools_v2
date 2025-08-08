use crate::cmd::logs::handle_logs;
use crate::config::ClusterConfig;
use crate::utils::serde::deserialize_request_gpus;
use crate::utils::ssh::{parse_json_relaxed, run_remote};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{cursor, execute, terminal};
use serde::Deserialize;
use std::io::{stdout, Write};

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
    #[serde(
        rename = "RequestGPUs",
        default,
        deserialize_with = "deserialize_request_gpus"
    )]
    request_gpus: i32,
}

fn job_id(j: &JobRow) -> String {
    format!("{}.{}", j.cluster_id, j.proc_id)
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

    let attrs = [
        "ClusterId",
        "ProcId",
        "Cmd",
        "Args",
        "RequestGPUs",
        "JobStartDate",
    ]
    .join(",");
    let cmd = format!("condor_q {} -json -attributes {}", username, attrs);
    let out = run_remote(login, &cmd)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("SSH command failed: {}", stderr).into());
    }
    let jobs: Vec<JobRow> = parse_json_relaxed(&out.stdout)?;
    if jobs.is_empty() {
        println!("No jobs found.");
        return Ok(());
    }

    let mut sel: usize = 0;

    // Setup terminal
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let res = (|| -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Render frame
            execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

            // Terminal width for dynamic args column
            let (cols, _) = terminal::size()?;

            // Row 0: header
            let mut row: u16 = 0;
            execute!(
                stdout,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            writeln!(stdout, "Use ↑/↓ to navigate, 'l' for logs, 'q' to quit")?;
            row += 1;

            // Rows: jobs
            for (idx, j) in jobs.iter().enumerate() {
                execute!(
                    stdout,
                    cursor::MoveTo(0, row),
                    terminal::Clear(terminal::ClearType::CurrentLine)
                )?;

                let sel_prefix = if idx == sel { "> " } else { "  " };
                let cmd_display: String = j.cmd.as_deref().unwrap_or("").chars().take(50).collect();
                let args_full = j.args.as_deref().unwrap_or("");

                // Build base without args
                let base = format!(
                    "{}{:>10}  GPUs: {}  Cmd: {} ",
                    sel_prefix,
                    job_id(j),
                    j.request_gpus,
                    cmd_display,
                );

                // Compute remaining columns for args
                let total_cols = cols as usize;
                let base_width = base.len();
                let avail = total_cols.saturating_sub(base_width);
                let args_display: String = if avail > 0 {
                    args_full.chars().take(avail).collect()
                } else {
                    String::new()
                };

                write!(stdout, "{}", base)?;
                writeln!(stdout, "{}", args_display)?;

                row += 1;
            }
            stdout.flush()?;

            // Input
            match read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => break,
                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                }) => {
                    if sel > 0 {
                        sel -= 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    ..
                }) => {
                    if sel + 1 < jobs.len() {
                        sel += 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    let selected = &jobs[sel];
                    // Restore terminal and show logs, then exit without returning to menu
                    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
                    terminal::disable_raw_mode()?;
                    let selector = Some(format!("{}.{}", selected.cluster_id, selected.proc_id));
                    if let Err(e) = handle_logs(selector) {
                        eprintln!("Error showing logs: {}", e);
                    }
                    return Ok(()); // exit after showing logs
                }
                _ => {}
            }
        }
        Ok(())
    })();

    // Restore terminal
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    if let Err(e) = res {
        return Err(e);
    }

    Ok(())
}
