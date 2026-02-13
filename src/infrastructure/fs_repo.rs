use crate::domain::models::{AnanicyRule, EnrichedRule};
use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct RuleRepository {}

impl RuleRepository
{
    pub fn new() -> Self
    {
        Self {}
    }

    pub fn load_rules_from_dir(&self, path: &Path) -> Result<Vec<EnrichedRule>>
    {
        let mut rules = Vec::new();

        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok())
        {
            if entry.path().extension().map_or(false, |e| e == "rules")
            {
                if let Ok(mut file_rules) = self.parse_file(entry.path())
                {
                    rules.append(&mut file_rules);
                }
            }
        }

        Ok(rules)
    }

    fn parse_file(&self, path: &Path) -> Result<Vec<EnrichedRule>>
    {
        let content = fs::read_to_string(path)?;
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
                if let Ok(data) = serde_json::from_str::<AnanicyRule>(trimmed)
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
                    });

                    rules_processed_in_block = true;
                }
            }
        }

        Ok(rules)
    }
}
