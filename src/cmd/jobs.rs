use crate::cmd::condor::condor_history_for_user;
use crate::cmd::logs::handle_logs;
use crate::config::ClusterConfig;
use crate::utils::serde::deserialize_request_gpus;
use crate::utils::ssh::{parse_json_relaxed, run_remote};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    cursor, execute,
    style::{Color, ResetColor, SetForegroundColor},
    terminal,
};
use serde::Deserialize;
use std::io::{stdout, Write};
use std::process::Command;
use std::time::{Duration, Instant};

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
    #[serde(rename = "JobStatus")]
    job_status: i32,
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

fn scrolling_window(text: &str, width: usize, offset: usize) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if width == 0 {
        return String::new();
    }
    if chars.len() <= width {
        return format!("{:<width$}", text, width = width);
    }
    // Extended buffer: text + spaces(gap) + text, with a short gap for quicker repeat
    let gap = std::cmp::min(width, 10);
    let mut ext: Vec<char> = Vec::with_capacity(chars.len() * 2 + gap);
    ext.extend_from_slice(&chars);
    ext.extend(std::iter::repeat(' ').take(gap));
    ext.append(&mut chars);
    let max_start = ext.len().saturating_sub(width);
    let start = if max_start == 0 {
        0
    } else {
        offset % max_start
    };
    ext[start..start + width].iter().collect()
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
        "JobStatus",
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

    let running_jobs: Vec<JobRow> = parse_json_relaxed(&out.stdout)?;
    let hist_attrs = [
        "ClusterId",
        "ProcId",
        "Cmd",
        "Args",
        "JobStatus",
        "RequestGPUs",
    ]
    .join(",");
    let recent_hist: Vec<JobRow> = condor_history_for_user(login, &username, &hist_attrs, 10)?;

    let mut rows: Vec<JobRow> = Vec::new();
    rows.extend(running_jobs);
    rows.extend(recent_hist);

    if rows.is_empty() {
        println!("No jobs found for user {}.", username);
        return Ok(());
    }

    let mut sel: usize = 0;

    // Setup terminal
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let res = (|| -> Result<(), Box<dyn std::error::Error>> {
        // Scrolling state
        let mut scroll_offset: usize = 0;
        let mut scroll_start_at: Instant = Instant::now();
        let mut last_scroll_tick: Instant = Instant::now();
        let mut scroll_paused: bool = true;
        loop {
            // Render frame
            execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

            // Terminal width for dynamic column sizing
            let (cols, _) = terminal::size()?;
            let cols = cols as usize;
            const JOBID_W: usize = 10;
            const GPUS_W: usize = 4;
            // sel_prefix (2) + status+space (2) + jobid (10) + two spaces (2) + gpus (4) + two spaces (2)
            let base_consumed: usize = 2 + 2 + JOBID_W + 2 + GPUS_W + 2;
            // Cmd column: min 20 chars, max 20% of terminal width. If 20% < 20, use 20.
            let min_cmd_w: usize = 20;
            let max_cmd_w: usize = cols / 5;
            let mut cmd_w: usize = if max_cmd_w < min_cmd_w {
                min_cmd_w
            } else {
                max_cmd_w
            };
            // Do not exceed available columns
            let max_allowed_cmd = cols.saturating_sub(base_consumed + 2);
            if cmd_w > max_allowed_cmd {
                cmd_w = max_allowed_cmd;
            }
            let args_w: usize = cols.saturating_sub(base_consumed + cmd_w + 2);

            // Row 0: header
            let mut row: u16 = 0;
            execute!(
                stdout,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            writeln!(
                stdout,
                "Use ↑/↓ to navigate, 'l' logs, 's' SSH, 'r' refresh, 'p' toggle scroll, 'q' quit"
            )?;
            row += 1;

            // add row explaining statuses
            execute!(
                stdout,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
            writeln!(stdout, "I: Idle, R: Running, X: Removed, C: Completed, H: Held, O: Transferring Output, S: Suspended")?;
            row += 1;

            // Header row
            execute!(
                stdout,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            let jobid_h = format!("{:>width$}", "JobID", width = JOBID_W);
            let gpus_h = format!("{:>width$}", "GPUs", width = GPUS_W);
            let cmd_h = format!("{:<width$}", "Cmd", width = cmd_w);
            writeln!(stdout, "  S {}  {}  {}  Args", jobid_h, gpus_h, cmd_h)?;
            row += 1;

            // Rows: jobs
            for (idx, j) in rows.iter().enumerate() {
                execute!(
                    stdout,
                    cursor::MoveTo(0, row),
                    terminal::Clear(terminal::ClearType::CurrentLine)
                )?;

                let sel_prefix = if idx == sel { "> " } else { "  " };
                let cmd_text = j.cmd.as_deref().unwrap_or("");
                let args_text = j.args.as_deref().unwrap_or("");
                let cmd_col = scrolling_window(cmd_text, cmd_w, scroll_offset);

                // Build base without args
                let jobid_col = format!("{:>width$}", job_id(j), width = JOBID_W);
                let gpus_col = format!("{:>width$}", j.request_gpus, width = GPUS_W);
                let cmd_col = cmd_col;
                let base = format!("{}  {}  {}  ", jobid_col, gpus_col, cmd_col);

                // Compute remaining columns for args
                let args_display: String = if args_w > 0 {
                    scrolling_window(args_text, args_w, scroll_offset)
                } else {
                    String::new()
                };

                // Selection prefix and colored status
                write!(stdout, "{}", sel_prefix)?;
                let (status_ch, status_color) = match j.job_status {
                    1 => ('I', Color::Blue),      // Idle
                    2 => ('R', Color::Green),     // Running
                    3 => ('X', Color::DarkRed),   // Removed
                    4 => ('C', Color::DarkGreen), // Completed
                    5 => ('H', Color::Yellow),    // Held
                    6 => ('O', Color::Cyan),      // Transferring Output
                    7 => ('S', Color::Magenta),   // Suspended
                    _ => ('?', Color::White),
                };
                execute!(stdout, SetForegroundColor(status_color))?;
                write!(stdout, "{} ", status_ch)?;
                execute!(stdout, ResetColor)?;

                write!(stdout, "{}", base)?;
                writeln!(stdout, "{}", args_display)?;

                row += 1;
            }
            stdout.flush()?;

            // Scroll timing
            let now = Instant::now();
            if !scroll_paused {
                if now.duration_since(scroll_start_at) >= Duration::from_millis(150)
                    && now.duration_since(last_scroll_tick) >= Duration::from_millis(150)
                {
                    scroll_offset = scroll_offset.wrapping_add(1);
                    last_scroll_tick = now;
                }
            }

            // Non-blocking input with timeout so scrolling can advance
            if crossterm::event::poll(Duration::from_millis(100))? {
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
                        if sel + 1 < rows.len() {
                            sel += 1;
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        let selected = &rows[sel];
                        execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
                        terminal::disable_raw_mode()?;
                        let selector =
                            Some(format!("{}.{}", selected.cluster_id, selected.proc_id));
                        if let Err(e) = handle_logs(selector) {
                            eprintln!("Error showing logs: {}", e);
                        }
                        return Ok(());
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('r'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        // Show immediate indicator and refresh data synchronously
                        execute!(
                            stdout,
                            cursor::MoveTo(0, 1),
                            terminal::Clear(terminal::ClearType::CurrentLine)
                        )?;
                        writeln!(stdout, "Refreshing...")?;
                        stdout.flush()?;

                        let cmd = format!("condor_q {} -json -attributes {}", username, attrs);
                        match run_remote(login, &cmd) {
                            Ok(out) => {
                                if !out.status.success() {
                                    let stderr = String::from_utf8_lossy(&out.stderr);
                                    eprintln!("Refresh failed: {}", stderr);
                                } else if let Ok(new_running) =
                                    parse_json_relaxed::<Vec<JobRow>>(&out.stdout)
                                {
                                    if let Ok(new_hist) =
                                        condor_history_for_user(login, &username, &hist_attrs, 10)
                                    {
                                        rows.clear();
                                        rows.extend(new_running);
                                        rows.extend(new_hist);
                                        sel = if rows.is_empty() {
                                            0
                                        } else {
                                            sel.min(rows.len() - 1)
                                        };
                                        scroll_offset = 0;
                                        scroll_start_at = Instant::now();
                                        last_scroll_tick = scroll_start_at;
                                    }
                                }
                            }
                            Err(e) => eprintln!("Refresh error: {}", e),
                        }
                        // Next frame will clear the indicator
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('s'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        // Open new Terminal window and SSH to job via login node
                        let selected = &rows[sel];
                        let jobid = format!("{}.{}", selected.cluster_id, selected.proc_id);
                        let mut ssh_cmd = String::from("ssh ");
                        if let Some(name) = &login.ssh_config_name {
                            ssh_cmd.push_str(name);
                        } else {
                            if let Some(identity) = &login.identity_file {
                                if !identity.is_empty() {
                                    ssh_cmd.push_str("-i '");
                                    ssh_cmd.push_str(&identity.replace('\'', "'\\''"));
                                    ssh_cmd.push_str("' ");
                                }
                            }
                            ssh_cmd.push_str(&format!("{}@{}", login.username, login.hostname));
                        }
                        ssh_cmd.push(' ');
                        ssh_cmd.push_str(&format!("\"condor_ssh_to_job {}\"", jobid));

                        let script_cmd = ssh_cmd.replace('\\', "\\\\").replace('"', "\\\"");
                        let osa = format!(
                            "tell application \"Terminal\" to do script \"{}\"",
                            script_cmd
                        );
                        let _ = Command::new("osascript").arg("-e").arg(osa).spawn();
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('p'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        // Toggle pause, restart delay on resume
                        scroll_paused = !scroll_paused;
                        scroll_start_at = Instant::now();
                        last_scroll_tick = scroll_start_at;
                    }
                    Event::Resize(_, _) => {
                        // Reset scrolling on resize
                        scroll_offset = 0;
                        scroll_start_at = Instant::now();
                        last_scroll_tick = scroll_start_at;
                    }
                    _ => {}
                }
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
