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
                let match_str =
                    |opt: &Option<String>| opt.as_deref().unwrap_or("").to_lowercase().contains(&query_lower);

                let match_num =
                    |opt: &Option<i32>| opt.map(|n| n.to_string()).unwrap_or_default().contains(&query_lower);

                match_str(&r.data.name)
                    || match_str(&r.data.rule_type)
                    || match_str(&r.data.sched)
                    || match_str(&r.data.ioclass)
                    || match_str(&r.data.cgroup)
                    || match_num(&r.data.nice)
                    || match_num(&r.data.latency_nice)
                    || match_num(&r.data.rtprio)
                    || match_num(&r.data.oom_score_adj)
                    || match_str(&r.context_comment)
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
