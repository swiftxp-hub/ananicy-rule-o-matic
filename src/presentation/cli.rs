use crate::application::process_service::ProcessService;
use crate::domain::models::EnrichedRule;

use colored::*;
use rust_i18n::t;
use std::borrow::Cow;

pub fn print_search_results(rules: &[EnrichedRule], process_service: &ProcessService)
{
    if rules.is_empty()
    {
        println!("{}", t!("no_rules_found").yellow());

        return;
    }

    let rules_found_message = t!("rules_found", count = rules.len());
    println!("{}", rules_found_message.green());
    println!();

    for rule in rules
    {
        let category = rule
            .source_file
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("root");

        let name = rule
            .data
            .name
            .as_deref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| t!("unknown"));

        let is_active = process_service.is_process_active(&name);

        let display_name = if is_active
        {
            format!("{} [ACTIVE]", name).green().bold()
        }
        else
        {
            name.cyan().bold()
        };

        print!("[{}] Name: {}", category.blue(), display_name);

        if let Some(rule_type) = rule.data.rule_type.as_deref()
        {
            print!(" | Type: {}", rule_type.white());
        }

        if let Some(nice) = rule.data.nice
        {
            print!(" | Nice: {}", nice.to_string().yellow());
        }

        if let Some(latency_nice) = rule.data.latency_nice
        {
            print!(" | Nice latency: {}", latency_nice);
        }

        if let Some(sched) = rule.data.sched.as_deref()
        {
            print!(" | Scheduling policy: {}", sched);
        }

        if let Some(rtprio) = &rule.data.rtprio
        {
            print!(" | Static priority: {}", rtprio);
        }

        if let Some(ioclass) = &rule.data.ioclass
        {
            print!(" | IO class: {}", ioclass);
        }

        if let Some(oom_score_adj) = &rule.data.oom_score_adj
        {
            print!(" | Out of memory killer score: {}", oom_score_adj);
        }

        if let Some(cgroup) = &rule.data.cgroup
        {
            print!(" | CGroup: {}", cgroup);
        }

        println!();

        println!("  {}: {}", t!("file"), rule.source_file.to_string_lossy().dimmed());

        if let Some(comment) = &rule.context_comment
        {
            println!("  {}:", t!("info"));
            for line in comment.lines()
            {
                println!("    {}", line.italic().dimmed());
            }
        }

        println!();
    }
}
