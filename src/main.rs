use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;

#[macro_use]
extern crate rust_i18n;

mod application;
mod domain;
mod infrastructure;
mod presentation;

use application::rule_service::RuleService;
use infrastructure::config::load_or_create_config;
use infrastructure::fs_repo::FsRuleRepository;

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
    let repo = Arc::new(FsRuleRepository::new(config.rule_paths));
    let rule_service = RuleService::new(repo);

    match args.command
    {
        Some(Commands::Search { query }) =>
        {
            let rules = rule_service.search_rules(&query)?;
            presentation::cli::print_search_results(&rules);
        }
        None =>
        {
            presentation::tui::run_app(&rule_service)?;
        }
    }

    Ok(())
}
