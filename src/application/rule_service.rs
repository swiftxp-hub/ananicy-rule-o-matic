use crate::domain::models::EnrichedRule;
use crate::infrastructure::fs_repo::RuleRepository;
use anyhow::Result;
use std::path::Path;

pub struct RuleService
{
    rule_repository: RuleRepository,
}

impl RuleService
{
    pub fn new(rule_repository: RuleRepository) -> Self
    {
        Self { rule_repository }
    }

    pub fn search_rules(&self, query: &str) -> Result<Vec<EnrichedRule>>
    {
        let mut rules = self.rule_repository.load_rules_from_dir(Path::new("./test_rules"))?;

        let query_lower = query.to_lowercase();

        if !query.is_empty()
        {
            rules.retain(|r| {
                r.data
                    .name
                    .as_ref()
                    .map_or(false, |n| n.to_lowercase().contains(&query_lower))
                    || r.context_comment
                        .as_ref()
                        .map_or(false, |c| c.to_lowercase().contains(&query_lower))
            });
        }

        self.sort_rules(&mut rules);

        Ok(rules)
    }

    fn sort_rules(&self, rules: &mut Vec<EnrichedRule>)
    {
        rules.sort_by(|a, b| {
            let get_folder = |path: &std::path::Path| {
                path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase()
            };

            let cat_a = get_folder(&a.source_file);
            let cat_b = get_folder(&b.source_file);

            match cat_a.cmp(&cat_b)
            {
                std::cmp::Ordering::Equal =>
                {
                    let name_a = a.data.name.as_deref().unwrap_or("").to_lowercase();
                    let name_b = b.data.name.as_deref().unwrap_or("").to_lowercase();
                    name_a.cmp(&name_b)
                }
                other => other,
            }
        });
    }
}
