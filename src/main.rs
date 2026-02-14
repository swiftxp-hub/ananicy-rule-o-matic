use anyhow::Result;
use clap::{Parser, Subcommand};

#[macro_use]
extern crate rust_i18n;

mod application;
mod domain;
mod infrastructure;
mod presentation;

use application::process_service::ProcessService;
use application::rule_service::RuleService;
use infrastructure::config::load_or_create_config;
use infrastructure::rule_repository::RuleRepository;

i18n!("locales");

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
    let args = Args::parse();

    rust_i18n::set_locale(&args.language);

    let config = load_or_create_config()?;
    let rule_repository = RuleRepository::new(config.rule_paths);
    let rule_service = RuleService::new(rule_repository);
    let mut process_service = ProcessService::new();

    match args.command
    {
        Some(Commands::Search { query }) =>
        {
            process_service.update_processes();

            let rules = rule_service.search_rules(&query)?;
            presentation::cli::print_search_results(&rules, &process_service);
        }
        None =>
        {
            presentation::tui::run_app(&rule_service, &mut process_service)?;
        }
    }

    Ok(())
}
