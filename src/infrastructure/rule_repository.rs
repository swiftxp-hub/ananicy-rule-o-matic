use crate::domain::models::{AnanicyRule, EnrichedRule};

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct RuleRepository
{
    base_paths: Vec<PathBuf>,
}

impl RuleRepository
{
    pub fn new(base_paths: Vec<PathBuf>) -> Self
    {
        Self { base_paths }
    }

    pub fn load_all(&self) -> Result<Vec<EnrichedRule>>
    {
        let mut all_rules = Vec::new();

        for base_path in &self.base_paths
        {
            let rules = self.load_rules_from_dir(base_path);
            all_rules.extend(rules);
        }

        Ok(all_rules)
    }

    fn load_rules_from_dir(&self, path: &Path) -> Vec<EnrichedRule>
    {
        let mut rules = Vec::new();

        if !path.exists()
        {
            return rules;
        }

        let mut entries: Vec<_> = WalkDir::new(path).into_iter().filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.path().to_path_buf());

        for entry in entries
        {
            if entry.path().extension().map_or(false, |e| e == "rules")
            {
                match self.parse_file(entry.path())
                {
                    Ok(mut file_rules) =>
                    {
                        rules.append(&mut file_rules);
                    }

                    Err(err) =>
                    {
                        eprintln!("Skipping invalid rule file {:?}: {}", entry.path(), err);
                    }
                }
            }
        }

        rules
    }

    fn parse_file(&self, path: &Path) -> Result<Vec<EnrichedRule>>
    {
        let content = fs::read_to_string(path).with_context(|| format!("Failed to read rule file: {:?}", path))?;

        let mut rules = Vec::new();
        let mut comment_buffer = Vec::new();
        let mut rules_processed_in_block = false;

        for line in content.lines()
        {
            let trimmed = line.trim();

            if trimmed.is_empty()
            {
                comment_buffer.clear();
                rules_processed_in_block = false;

                continue;
            }

            if trimmed.starts_with('#')
            {
                if rules_processed_in_block
                {
                    comment_buffer.clear();
                    rules_processed_in_block = false;
                }

                comment_buffer.push(trimmed.to_string());
            }
            else if trimmed.starts_with('{')
            {
                match serde_json::from_str::<AnanicyRule>(trimmed)
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

                    Err(_) =>
                    {
                        continue;
                    }
                }
            }
        }

        Ok(rules)
    }
}
