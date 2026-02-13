use crate::domain::models::EnrichedRule;
use crate::infrastructure::fs_repo::RuleRepository;
use anyhow::Result;
use std::path::Path;

pub struct RuleService
{
    repo: RuleRepository,
}

impl RuleService
{
    pub fn new(repo: RuleRepository) -> Self
    {
        Self { repo }
    }

    pub fn search_rules(&self, query: &str) -> Result<Vec<EnrichedRule>>
    {
        let rules = self.repo.load_rules_from_dir(Path::new("./test_rules"))?;

        let query_lower = query.to_lowercase();

        let filtered: Vec<EnrichedRule> = rules
            .into_iter()
            .filter(|r| {
                r.data
                    .name
                    .as_ref()
                    .map_or(false, |n| n.to_lowercase().contains(&query_lower))
                    || r.context_comment
                        .as_ref()
                        .map_or(false, |c| c.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(filtered)
    }
}
