use anyhow::Result;
use clap::{Parser, Subcommand};

use ananicy_rule_o_matic::application::process_service::ProcessService;
use ananicy_rule_o_matic::application::rule_service::RuleService;
use ananicy_rule_o_matic::infrastructure::rule_repository::RuleRepository;
use ananicy_rule_o_matic::presentation;
use colored::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args
{
    #[arg(short, long, default_value = "en")]
    language: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands
{
    Search
    {
        query: String
    },
}

fn main() -> Result<()>
{
    let cli_header = format!("{}{}", "Rule-O-Matic v", env!("CARGO_PKG_VERSION"));
    println!("{}", cli_header.cyan().bold());

    let cli_args = Args::parse();

    rust_i18n::set_locale(&cli_args.language);

    let rule_repository = RuleRepository::new();
    let rule_service = RuleService::new(rule_repository);
    let mut process_service = ProcessService::new();

    match cli_args.command
    {
        Some(Commands::Search { query }) =>
        {
            process_service.update_processes();

            let (rules, errors) = rule_service.search_rules(&query)?;
            presentation::cli::print_search_results(&rules, &errors, &process_service);
        }

        None =>
        {
            presentation::tui::run_app(&rule_service, &mut process_service)?;
        }
    }

    Ok(())
}
