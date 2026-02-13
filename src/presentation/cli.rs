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
    println!();

    let mut sorted_rules = rules.to_vec();

    sorted_rules.sort_by(|a, b| {
        let get_folder = |path: &std::path::Path| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase()
        };

        let category_a = get_folder(&a.source_file);
        let category_b = get_folder(&b.source_file);

        match category_a.cmp(&category_b)
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

    for rule in sorted_rules
    {
        let category = rule
            .source_file
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("root");

        let name = rule.data.name.as_deref().unwrap_or("Unknown");
        let rule_type = rule.data.rule_type.as_deref().unwrap_or("-");

        print!("[{}] {} ({})", category.blue(), name.cyan().bold(), rule_type.white());

        if let Some(nice) = rule.data.nice
        {
            print!(" Nice: {}", nice.to_string().yellow());
        }

        if let Some(ioclass) = &rule.data.ioclass
        {
            print!(" IO: {}", ioclass.magenta());
        }

        println!();

        println!("  File: {}", rule.source_file.to_string_lossy().dimmed());

        if let Some(comment) = &rule.context_comment
        {
            let preview = comment.lines().next().unwrap_or("");
            if !preview.is_empty()
            {
                println!("  Info: {}", preview.italic().dimmed());
            }
        }
        println!();
    }
}
