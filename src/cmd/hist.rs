use crate::cmd::condor::condor_history_for_user;
use crate::config::ClusterConfig;
use crate::utils::serde::{deserialize_i64_lenient, deserialize_request_gpus};
use comfy_table::{
    presets::UTF8_FULL, Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize, Debug)]
struct HistRow {
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
    #[serde(
        rename = "QDate",
        default,
        deserialize_with = "deserialize_i64_lenient"
    )]
    q_unix: i64,
    #[serde(
        rename = "JobStartDate",
        default,
        deserialize_with = "deserialize_i64_lenient"
    )]
    start_unix: i64,
}

fn render_hist_table(rows: &[HistRow]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("JobID").add_attribute(Attribute::Bold),
            Cell::new("Cmd").add_attribute(Attribute::Bold),
            Cell::new("Args").add_attribute(Attribute::Bold),
            Cell::new("GPUs")
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Right),
            Cell::new("Queued")
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Right),
            Cell::new("Started")
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Right),
        ]);

    for j in rows {
        let jobid = format!("{}.{}", j.cluster_id, j.proc_id);
        let queued = if j.q_unix > 0 {
            // QDate is seconds since epoch
            let dt = OffsetDateTime::from_unix_timestamp(j.q_unix).ok();
            dt.map(|d| d.format(&Rfc3339).unwrap_or_else(|_| "-".into()))
                .unwrap_or_else(|| "-".into())
        } else {
            "-".into()
        };
        let started = if j.start_unix > 0 {
            let dt = OffsetDateTime::from_unix_timestamp(j.start_unix).ok();
            dt.map(|d| d.format(&Rfc3339).unwrap_or_else(|_| "-".into()))
                .unwrap_or_else(|| "-".into())
        } else {
            "-".into()
        };
        table.add_row(vec![
            Cell::new(jobid).fg(Color::DarkGrey),
            Cell::new(j.cmd.as_deref().unwrap_or("")),
            Cell::new(j.args.as_deref().unwrap_or("")),
            Cell::new(j.request_gpus.to_string()).set_alignment(CellAlignment::Right),
            Cell::new(queued).set_alignment(CellAlignment::Right),
            Cell::new(started).set_alignment(CellAlignment::Right),
        ]);
    }

    table
}

pub fn handle_hist(limit: Option<usize>) -> Result<(), Box<dyn std::error::Error>> {
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

    let limit = limit.unwrap_or(10);

    let attrs = [
        "ClusterId",
        "ProcId",
        "Cmd",
        "Args",
        "RequestGPUs",
        "QDate",
        "JobStartDate",
    ]
    .join(",");

    let rows: Vec<HistRow> = condor_history_for_user(login, &username, &attrs, limit)?;

    if rows.is_empty() {
        println!("No historical jobs found for user {}.", username);
        return Ok(());
    }

    let table = render_hist_table(&rows);
    println!("{}", table);
    Ok(())
}
