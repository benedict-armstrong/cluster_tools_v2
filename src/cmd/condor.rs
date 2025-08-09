use crate::config::LoginConfig;
use crate::utils::ssh::{parse_json_relaxed, run_remote};
use serde::de::DeserializeOwned;

pub fn condor_q_for_user<T: DeserializeOwned>(
    login: &LoginConfig,
    username: &str,
    attrs: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let cmd = format!("condor_q {} -json -attributes {}", username, attrs);
    let out = run_remote(login, &cmd)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("SSH command failed: {}", stderr).into());
    }
    let v: Vec<T> = parse_json_relaxed(&out.stdout)?;
    Ok(v)
}

pub fn condor_history_for_user<T: DeserializeOwned>(
    login: &LoginConfig,
    username: &str,
    attrs: &str,
    limit: usize,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let cmd = format!(
        "condor_history {} -json -attributes {} -limit {}",
        username, attrs, limit
    );
    let out = run_remote(login, &cmd)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("SSH command failed: {}", stderr).into());
    }
    let v: Vec<T> = parse_json_relaxed(&out.stdout)?;
    Ok(v)
}
