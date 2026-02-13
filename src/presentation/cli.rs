use crate::domain::models::EnrichedRule;
use colored::*;

pub fn print_search_results(rules: &[EnrichedRule])
{
    if rules.is_empty()
    {
        println!("{}", "No rules found.".yellow());

        return;
    }

    println!("Found {} rules:", rules.len().to_string().green());

    for rule in rules
    {
        let name = rule.data.name.as_deref().unwrap_or("Unknown");
        let r_type = rule.data.rule_type.as_deref().unwrap_or("-");

        println!("- {} [{}] ({:?})", name.cyan().bold(), r_type, rule.source_file);

        if let Some(comment) = &rule.context_comment
        {
            let preview = comment.lines().next().unwrap_or("");

            println!("  {}", preview.dimmed());
        }
    }
}
