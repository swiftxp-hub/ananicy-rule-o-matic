use crate::application::process_service::ProcessService;
use crate::domain::models::AppConfig;
use crate::domain::models::EnrichedRule;

use colored::*;
use rust_i18n::t;
use std::borrow::Cow;

pub fn print_config(config: &AppConfig)
{
    println!("{}", "Current Configuration:".bold());
    println!("{}", "Rule Paths:".yellow());

    for path in &config.rule_paths
    {
        println!("  - {}", path.display());
    }
}

pub fn print_config_rule_paths(config: &AppConfig)
{
    println!("{}", "Rule Paths:".yellow());

    for path in &config.rule_paths
    {
        println!("  - {}", path.display());
    }
}

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
            .unwrap_or_else(|| t!("unknown").into());

        let process_infos = process_service.get_process_infos(&name);
        let is_active = !process_infos.is_empty();

        let display_name = if is_active
        {
            let pid_string = process_infos
                .first()
                .map(|info| format!("(PID: {})", info.pid))
                .unwrap_or_default();

            format!("{} [ACTIVE] {}", name, pid_string).green().bold()
        }
        else
        {
            name.cyan().bold()
        };

        let shadowed_marker = if rule.shadowed
        {
            format!(" {}", "(Shadowed)".red())
        }
        else
        {
            String::new()
        };

        print!("[{}] Name: {}{}", category.blue(), display_name, shadowed_marker);

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
            print!(" | Latency: {}", latency_nice.to_string().magenta());
        }

        if let Some(sched) = rule.data.sched.as_deref()
        {
            print!(" | Sched: {}", sched);
        }

        if let Some(ioclass) = rule.data.ioclass.as_deref()
        {
            print!(" | IO: {}", ioclass);
        }

        if let Some(rtprio) = &rule.data.rtprio
        {
            print!(" | Static priority: {}", rtprio);
        }

        if let Some(oom_score_adj) = &rule.data.oom_score_adj
        {
            print!(" | Out of memory killer score: {}", oom_score_adj);
        }

        if let Some(cgroup) = &rule.data.cgroup
        {
            print!(" | Cgroup: {}", ProcessService::shorten_cgroup(cgroup));
        }

        println!();

        if is_active
        {
            let process_info = &process_infos[0];
            let mut status_parts = Vec::new();

            if rule.data.nice.is_some() || process_info.nice.is_some()
            {
                status_parts.push(check_i32("Nice", rule.data.nice, process_info.nice));
            }

            if rule.data.latency_nice.is_some() || process_info.latency_nice.is_some()
            {
                status_parts.push(check_i32("LatNice", rule.data.latency_nice, process_info.latency_nice));
            }

            if rule.data.sched.is_some() || process_info.sched_policy.is_some()
            {
                status_parts.push(check_str("Sched", &rule.data.sched, &process_info.sched_policy));
            }

            if rule.data.ioclass.is_some() || process_info.ioclass.is_some()
            {
                status_parts.push(check_str("IO", &rule.data.ioclass, &process_info.ioclass));
            }

            if rule.data.oom_score_adj.is_some() || process_info.oom_score_adj.is_some()
            {
                status_parts.push(check_i32("OOM", rule.data.oom_score_adj, process_info.oom_score_adj));
            }

            if rule.data.cgroup.is_some() || process_info.cgroup.is_some()
            {
                status_parts.push(check_cgroup("Cgroup", &rule.data.cgroup, &process_info.cgroup));
            }

            let status_line: Vec<String> = status_parts
                .into_iter()
                .filter(|part| !part.is_empty())
                .map(|part| part.to_string())
                .collect();

            if !status_line.is_empty()
            {
                println!("  ↳ Status: {}", status_line.join(" | "));
            }
        }

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

fn check_i32(label: &str, want: Option<i32>, have: Option<i32>) -> ColoredString
{
    if let (Some(wanted), Some(had)) = (want, have)
    {
        if wanted == had
        {
            format!("{} ok", label).green()
        }
        else
        {
            format!("{} {}! (want {})", label, had, wanted).red().bold()
        }
    }
    else if let (None, Some(had)) = (want, have)
    {
        format!("{}: {}", label, had).dimmed()
    }
    else
    {
        "".dimmed()
    }
}

fn check_str(label: &str, want: &Option<String>, have: &Option<String>) -> ColoredString
{
    if let (Some(wanted), Some(had)) = (want, have)
    {
        if wanted.eq_ignore_ascii_case(had)
        {
            format!("{} ok", label).green()
        }
        else
        {
            format!("{} {}! (want {})", label, had, wanted).red().bold()
        }
    }
    else if let (None, Some(had)) = (want, have)
    {
        format!("{}: {}", label, had).dimmed()
    }
    else
    {
        "".dimmed()
    }
}

fn check_cgroup(label: &str, want: &Option<String>, have: &Option<String>) -> ColoredString
{
    if let (Some(wanted), Some(had)) = (want, have)
    {
        if wanted.eq_ignore_ascii_case(had)
        {
            format!("{} ok", label).green()
        }
        else
        {
            let short_had = ProcessService::shorten_cgroup(had);
            let short_wanted = ProcessService::shorten_cgroup(wanted);
            format!("{} {}! (want {})", label, short_had, short_wanted).red().bold()
        }
    }
    else if let (None, Some(had)) = (want, have)
    {
        let short_had = ProcessService::shorten_cgroup(had);
        format!("{}: {}", label, short_had).dimmed()
    }
    else
    {
        "".dimmed()
    }
}
