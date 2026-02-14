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
use colored::*;
use infrastructure::config_repository::ConfigRepository;
use infrastructure::rule_repository::RuleRepository;
use std::path::PathBuf;

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
    Config
    {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands
{
    Show,
    #[command(name = "rule-paths")]
    RulePaths
    {
        #[command(subcommand)]
        command: RulePathsCommands,
    },
}

#[derive(Subcommand, Debug)]
enum RulePathsCommands
{
    Add
    {
        path: PathBuf,
    },
    Remove
    {
        path: PathBuf,
    },
    Show,
}

fn main() -> Result<()>
{
    let cli_header = format!("{}{}", "Rule-O-Matic v", env!("CARGO_PKG_VERSION"));
    println!("{}", cli_header.cyan().bold());

    let args = Args::parse();

    rust_i18n::set_locale(&args.language);

    let config_repository = ConfigRepository::new();
    let mut config = config_repository.load()?;

    let rule_repository = RuleRepository::new(config.rule_paths.clone());
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
        Some(Commands::Config { command }) => match command
        {
            ConfigCommands::Show =>
            {
                presentation::cli::print_config(&config);
            }

            ConfigCommands::RulePaths { command } => match command
            {
                RulePathsCommands::Add { path } =>
                {
                    if !config.rule_paths.contains(&path)
                    {
                        config.rule_paths.push(path.clone());
                        config_repository.save(&config)?;

                        println!("Added rule path: {}", path.display());
                    }
                    else
                    {
                        println!("{} {}", "Rule path already exists:".yellow(), path.display());
                    }
                }

                RulePathsCommands::Remove { path } =>
                {
                    if let Some(index) = config.rule_paths.iter().position(|p| *p == path)
                    {
                        config.rule_paths.remove(index);
                        config_repository.save(&config)?;

                        println!("Removed rule path: {}", path.display());
                    }
                    else
                    {
                        println!("{} {}", "Rule path not found:".yellow(), path.display());
                    }
                }

                RulePathsCommands::Show =>
                {
                    presentation::cli::print_config_rule_paths(&config);
                }
            },
        },
        None =>
        {
            presentation::tui::run_app(&rule_service, &mut process_service)?;
        }
    }

    Ok(())
}
