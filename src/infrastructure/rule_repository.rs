use crate::domain::models::{AnanicyRule, EnrichedRule};

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::{env, fs};
use walkdir::WalkDir;

pub struct RuleRepository
{
    base_path: PathBuf,
}

impl RuleRepository
{
    pub fn new() -> Self
    {
        let environment_variable_key = "ANANICY_CPP_CONFDIR";

        match env::var(environment_variable_key)
        {
            Ok(configuration_directory_path) => Self {
                base_path: PathBuf::from(configuration_directory_path),
            },

            Err(_) => Self {
                base_path: PathBuf::from("/etc/ananicy.d"),
            },
        }
    }

    pub fn new_with_base_path(base_path: PathBuf) -> Self
    {
        Self { base_path }
    }

    pub fn load_all(&self) -> Result<(Vec<EnrichedRule>, Vec<String>)>
    {
        let mut rules = Vec::new();
        let mut errors = Vec::new();

        if !self.base_path.exists()
        {
            errors.push(format!("Base path {:?} does not exist", self.base_path));

            return Ok((rules, errors));
        }

        let mut files: Vec<_> = WalkDir::new(self.base_path.as_path())
            .into_iter()
            .filter_map(|e| e.ok())
            .collect();

        files.sort_by_key(|e| e.path().to_path_buf());

        for file in files
        {
            if file.path().extension().map_or(false, |e| e == "rules")
            {
                let (mut file_rules, mut file_errors) = self.parse_file(file.path());

                rules.append(&mut file_rules);
                errors.append(&mut file_errors);
            }
        }

        Ok((rules, errors))
    }

    pub fn save_rule(&self, rule: &AnanicyRule) -> Result<()>
    {
        let rule_name = rule.name.as_deref().unwrap_or("unknown");
        let file_name = format!("{}.rules", rule_name);

        let target_dir = self.base_path.join("99-custom");
        if !target_dir.exists()
        {
            fs::create_dir_all(&target_dir).context("Failed to create custom rules directory")?;
        }

        let file_path = target_dir.join(file_name);
        let json = serde_json::to_string(rule).context("Failed to serialize rule")?;

        fs::write(&file_path, json).context("Failed to write rule file")?;

        Ok(())
    }

    fn parse_file(&self, path: &Path) -> (Vec<EnrichedRule>, Vec<String>)
    {
        let mut rules = Vec::new();
        let mut errors = Vec::new();

        let content = match fs::read_to_string(path)
        {
            Ok(c) => c,

            Err(e) =>
            {
                errors.push(format!("Failed to read rule file {:?}: {}", path, e));

                return (rules, errors);
            }
        };

        let mut comment_buffer = Vec::new();
        let mut rules_processed_in_block = false;

        for (line_idx, line) in content.lines().enumerate()
        {
            let trimmed_line = line.trim();

            if trimmed_line.is_empty()
            {
                comment_buffer.clear();
                rules_processed_in_block = false;

                continue;
            }

            if trimmed_line.starts_with('#')
            {
                if rules_processed_in_block
                {
                    comment_buffer.clear();
                    rules_processed_in_block = false;
                }

                comment_buffer.push(trimmed_line.to_string());
            }
            else if trimmed_line.starts_with('{')
            {
                match serde_json::from_str::<AnanicyRule>(trimmed_line)
                {
                    Ok(data) =>
                    {
                        rules.push(EnrichedRule {
                            data,
                            context_comment: if comment_buffer.is_empty()
                            {
                                None
                            }
                            else
                            {
                                Some(comment_buffer.join("\n"))
                            },
                            source_file: path.to_path_buf(),
                            shadowed: false,
                        });

                        rules_processed_in_block = true;
                    }
                    Err(e) =>
                    {
                        errors.push(format!("Parse error in {:?} at line {}: {}", path, line_idx + 1, e));
                    }
                }
            }
            else
            {
                errors.push(format!(
                    "Invalid syntax in {:?} at line {}: Line must start with '{{' or '#'",
                    path,
                    line_idx + 1
                ));
            }
        }

        (rules, errors)
    }
}
