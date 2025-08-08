use crate::config::LoginConfig;
use serde::de::DeserializeOwned;
use std::process::Command;

pub fn ssh_base_args(login: &LoginConfig) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "ssh".to_string(),
        "-T".to_string(),
        "-o".to_string(),
        "LogLevel=ERROR".to_string(),
    ];
    if let Some(name) = &login.ssh_config_name {
        args.push(name.clone());
    } else {
        if let Some(identity) = &login.identity_file {
            args.push("-i".to_string());
            args.push(identity.clone());
        }
        args.push(format!("{}@{}", login.username, login.hostname));
    }
    args
}

pub fn run_remote(
    login: &LoginConfig,
    remote_cmd: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut args = ssh_base_args(login);
    args.push(remote_cmd.to_string());
    let output = Command::new(&args[0])
        .args(&args[1..])
        .output()
        .map_err(|e| format!("Failed to execute SSH command: {}", e))?;
    Ok(output)
}

pub fn parse_json_relaxed<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    if let Ok(v) = serde_json::from_slice(bytes) {
        return Ok(v);
    }
    let s = String::from_utf8_lossy(bytes);
    if let Some(start) = s.find('[').or_else(|| s.find('{')) {
        let end = s
            .rfind(']')
            .or_else(|| s.rfind('}'))
            .unwrap_or_else(|| s.len().saturating_sub(1));
        let slice = &s[start..=end.min(s.len().saturating_sub(1))];
        return serde_json::from_str(slice);
    }
    serde_json::from_str(s.trim())
}

pub fn shell_escape_single_quotes(input: &str) -> String {
    input.replace('\'', "'\\''")
}

pub fn build_path(iwd: &str, path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{}", iwd, path)
    }
}
