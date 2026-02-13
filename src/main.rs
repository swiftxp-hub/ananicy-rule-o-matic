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

    let repo = RuleRepository::new();
    let service = RuleService::new(repo);

    match &cli.command
    {
        Some(Commands::Search { query }) =>
        {
            let results = service.search_rules(query)?;
            presentation::cli::print_search_results(&results);
        }

        None =>
        {
            presentation::tui::run_app()?;
        }
    }

    Ok(())
}
