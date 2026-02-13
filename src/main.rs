#[macro_use]
extern crate rust_i18n;
i18n!("locales");

mod application;
mod domain;
mod infrastructure;
mod presentation;

use application::rule_service::RuleService;
use clap::{Parser, Subcommand};
use infrastructure::fs_repo::RuleRepository;

#[derive(Parser)]
#[command(name = "ananicy-rule-o-matic")]
#[command(author = "swiftxp")]
#[command(version = "0.0.1")]
#[command(about = "A lightweight rule manager for Ananicy-Cpp.", long_about = None)]
struct Cli
{
    #[arg(short, long, default_value = "en")]
    language: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands
{
    Search
    {
        query: String
    },
}

fn main() -> anyhow::Result<()>
{
    let cli = Cli::parse();
    rust_i18n::set_locale(&cli.language);

    let rule_repository = RuleRepository::new();
    let rule_service = RuleService::new(rule_repository);

    match &cli.command
    {
        Some(Commands::Search { query }) =>
        {
            let results = rule_service.search_rules(query)?;
            presentation::cli::print_search_results(&results);
        }

        None =>
        {
            presentation::tui::run_app(&rule_service)?;
        }
    }

    Ok(())
}
