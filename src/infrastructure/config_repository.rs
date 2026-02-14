use crate::domain::models::AppConfig;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

pub struct ConfigRepository;

impl ConfigRepository
{
    pub fn new() -> Self
    {
        Self
    }

    fn get_config_path(&self) -> Result<PathBuf>
    {
        let project_dirs = ProjectDirs::from("com", "swiftxp", "ananicy-rule-o-matic")
            .context("Could not determine config directory")?;

        Ok(project_dirs.config_dir().join("config.toml"))
    }

    pub fn load(&self) -> Result<AppConfig>
    {
        let config_file = self.get_config_path()?;

        if !config_file.exists()
        {
            let config_dir = config_file.parent().context("Failed to get config directory")?;
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

    pub fn save(&self, config: &AppConfig) -> Result<()>
    {
        let config_file = self.get_config_path()?;
        let config_dir = config_file.parent().context("Failed to get config directory")?;

        if !config_dir.exists()
        {
            fs::create_dir_all(config_dir).context("Failed to create config directory")?;
        }

        let toml_string = toml::to_string_pretty(config)?;
        fs::write(&config_file, toml_string).context("Failed to write config file")?;

        Ok(())
    }
}