use crate::config::{ClusterConfig, LoginConfig};
use dialoguer::{Input, Select};

pub fn handle_login() -> Result<(), Box<dyn std::error::Error>> {
    println!("Configure cluster login credentials");

    let options = vec![
        "Use SSH config entry (recommended)",
        "Manual configuration (hostname, username, identity file)",
    ];

    let selection = Select::new()
        .with_prompt("Choose configuration method")
        .items(&options)
        .default(0)
        .interact()?;

    let login_config = match selection {
        0 => {
            // SSH config must be non-empty
            let ssh_config_name: String = Input::new()
                .with_prompt("SSH config name")
                .allow_empty(false)
                .interact_text()?;

            LoginConfig {
                hostname: String::new(), // Will be resolved from SSH config
                username: String::new(), // Will be resolved from SSH config
                identity_file: None,
                ssh_config_name: Some(ssh_config_name),
            }
        }
        1 => {
            // Manual configuration
            let hostname: String = Input::new().with_prompt("Hostname").interact_text()?;

            let username: String = Input::new().with_prompt("Username").interact_text()?;

            let identity_file: String = Input::new()
                .with_prompt("Identity file path (optional, press Enter to skip)")
                .allow_empty(true)
                .interact_text()?;

            LoginConfig {
                hostname,
                username,
                identity_file: if identity_file.is_empty() {
                    None
                } else {
                    Some(identity_file)
                },
                ssh_config_name: None,
            }
        }
        _ => unreachable!(),
    };

    let mut config = ClusterConfig::load();
    config.login = Some(login_config);
    config.save()?;

    Ok(())
}
