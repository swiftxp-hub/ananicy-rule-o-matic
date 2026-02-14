use ananicy_rule_o_matic::domain::models::AppConfig;
use ananicy_rule_o_matic::infrastructure::config_repository::ConfigRepository;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_load_creates_default_config_if_not_exists()
{
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_repository = ConfigRepository::new_with_base_path(temp_dir.path().to_path_buf());

    let config = config_repository.load().expect("Failed to load config");

    assert!(!config.rule_paths.is_empty());
    assert!(config.rule_paths.contains(&PathBuf::from("/etc/ananicy.d")));

    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists());

    let content = fs::read_to_string(&config_path).expect("Failed to read file");
    assert!(content.contains("rule_paths"));
}

#[test]
fn test_load_reads_existing_config()
{
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_repository = ConfigRepository::new_with_base_path(temp_dir.path().to_path_buf());

    let config_path = temp_dir.path().join("config.toml");
    let config_content = r#"
rule_paths = ["/tmp/test/rules"]
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    let config = config_repository.load().expect("Failed to load config");

    assert_eq!(config.rule_paths.len(), 1);
    assert_eq!(config.rule_paths[0], PathBuf::from("/tmp/test/rules"));
}

#[test]
fn test_save_writes_config()
{
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_repository = ConfigRepository::new_with_base_path(temp_dir.path().to_path_buf());

    let new_config = AppConfig {
        rule_paths: vec![PathBuf::from("/custom/path/1"), PathBuf::from("/custom/path/2")],
    };

    config_repository.save(&new_config).expect("Failed to save config");

    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists());

    let content = fs::read_to_string(config_path).expect("Failed to read saved config");
    assert!(content.contains("/custom/path/1"));
    assert!(content.contains("/custom/path/2"));

    let loaded_config = config_repository.load().expect("Failed to reload config");
    assert_eq!(loaded_config.rule_paths.len(), 2);
    assert_eq!(loaded_config.rule_paths[0], PathBuf::from("/custom/path/1"));
}

#[test]
fn test_save_creates_directory_if_missing()
{
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let deep_path = temp_dir.path().join("subdir/another_subdir");
    let config_repository = ConfigRepository::new_with_base_path(deep_path.clone());

    let config = AppConfig::default();

    config_repository
        .save(&config)
        .expect("Failed to save config in non-existent directory");

    let config_path = deep_path.join("config.toml");
    assert!(config_path.exists());
}
