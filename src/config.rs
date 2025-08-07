use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
}
