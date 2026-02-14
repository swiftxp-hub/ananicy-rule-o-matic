use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig
{
    pub rule_paths: Vec<PathBuf>,
}

impl Default for AppConfig
{
    fn default() -> Self
    {
        Self {
            rule_paths: vec![
                PathBuf::from("/etc/ananicy.d"),
                PathBuf::from("/usr/lib/ananicy.d"),
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config/ananicy.d"),
            ],
        }
    }
}

pub fn load_or_create_config() -> Result<AppConfig>
{
    let project_dirs =
        ProjectDirs::from("com", "swiftxp", "ananicy-rule-o-matic").context("Could not determine config directory")?;

    let config_dir = project_dirs.config_dir();
    let config_file = config_dir.join("config.toml");

    if !config_file.exists()
    {
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;

        let default_config = AppConfig::default();
        let toml_string = toml::to_string_pretty(&default_config)?;

        fs::write(&config_file, toml_string).context("Failed to write default config file")?;

        return Ok(default_config);
    }

    let content = fs::read_to_string(&config_file).context("Failed to read config file")?;
    let config: AppConfig = toml::from_str(&content).context("Failed to parse config file")?;

    Ok(config)
}
