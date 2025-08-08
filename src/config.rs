use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginConfig {
    pub hostname: String,
    pub username: String,
    pub identity_file: Option<String>,
    pub ssh_config_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ClusterConfig {
    pub login: Option<LoginConfig>,
}

impl ClusterConfig {
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".cluster_tools")
    }

    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path).expect("Failed to read config file");
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, contents)?;
        println!("Configuration saved to {}", config_path.display());
        Ok(())
    }

    // get username from login config or from ssh config
    pub fn get_username(&self) -> Option<String> {
        let login = self.login.as_ref()?;
        if !login.username.is_empty() {
            return Some(login.username.clone());
        }

        // Fallback: resolve from SSH config using `ssh -G <alias>`
        if let Some(alias) = &login.ssh_config_name {
            if let Ok(output) = Command::new("ssh").args(["-G", alias]).output() {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        for line in text.lines() {
                            let line_trim = line.trim();
                            // OpenSSH outputs lowercase keys in `ssh -G`
                            if let Some(rest) = line_trim.strip_prefix("user ") {
                                let value = rest.trim();
                                if !value.is_empty() {
                                    return Some(value.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}
