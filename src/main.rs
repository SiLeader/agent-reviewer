extern crate agent_reviewer_agent;
extern crate agent_reviewer_model_provider;
extern crate agent_reviewer_tools;
extern crate genai;

use crate::config::{PromptConfig, StepsPromptConfig};
use crate::instruction::load_instructions;
use crate::orchestrator::Orchestrator;
use crate::prompt::PromptManager;
use agent_reviewer_agent::builder::ReActAgentBuilder;
use agent_reviewer_model_provider::{ModelConfig, ModelProviderConfig, WithProviderConfig};
use clap::Parser;
use genai::Client;
use std::path::Path;
use tracing::info;

mod config;
mod instruction;
mod orchestrator;
mod prompt;

#[derive(clap::Parser)]
struct Args {
    #[arg(short, long, default_value = "agent-reviewer.toml")]
    config: String,

    #[arg(
        short,
        long,
        help = "Output file to write the review result to. If not specified, the result will be printed to stdout."
    )]
    output: Option<String>,

    #[arg(
        short,
        long,
        help = "Allow output to be written to stdout if the output file cannot be written to."
    )]
    allow_output_fallback_to_stdout: bool,

    #[arg(help = "Prompt to use for the review step.")]
    prompt: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let config = match load_config(args.config) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Failed to load configuration: {}", err);
            std::process::exit(1);
        }
    };
    info!("Configuration loaded: {:?}", config);
    let prompts = match load_prompt(&config.prompt) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to load prompt: {}", e);
            std::process::exit(1);
        }
    };
    let client = match create_client(config.models, config.model_providers) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create client: {}", e);
            std::process::exit(1);
        }
    };
    let agent_builder = ReActAgentBuilder::default().override_client(client);

    let orchestrator = Orchestrator::new(prompts, agent_builder, config.steps, config.agents);
    match orchestrator.run(args.prompt).await {
        Ok(result) => {
            if let Some(output_file) = args.output {
                match std::fs::write(&output_file, &result) {
                    Ok(_) => info!("Review result written to {}", output_file),
                    Err(e) => {
                        if args.allow_output_fallback_to_stdout {
                            tracing::warn!(
                                "Failed to write review result to file, falling back to stdout: {}",
                                e
                            );
                            println!("{}", result);
                        } else {
                            tracing::error!("Failed to write review result to file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            } else {
                println!("{}", result);
            }
        }
        Err(e) => {
            tracing::error!("Failed to run orchestrator: {}", e);
            std::process::exit(1);
        }
    }
}

fn load_config(config_path: impl AsRef<Path>) -> anyhow::Result<config::Config> {
    info!(
        "Loading configuration from: {}",
        config_path.as_ref().display()
    );
    let config: config::Config = toml::from_str(&std::fs::read_to_string(config_path)?)?;
    Ok(config)
}

fn load_prompt(config: &StepsPromptConfig) -> anyhow::Result<PromptManager> {
    let (triage_system, triage_user) = load_prompt_impl(&config.triage)?;
    let (review_system, review_user) = load_prompt_impl(&config.review)?;
    let (finalize_system, finalize_user) = load_prompt_impl(&config.finalize)?;
    let instructions = load_instructions();

    PromptManager::new(
        instructions,
        triage_system,
        triage_user,
        review_system,
        review_user,
        finalize_system,
        finalize_user,
    )
}

fn load_prompt_impl(config: &PromptConfig) -> anyhow::Result<(Option<String>, Option<String>)> {
    let system = match &config.system_file {
        None => None,
        Some(p) => Some(std::fs::read_to_string(p)?),
    };
    let user = match &config.user_template_file {
        None => None,
        Some(p) => Some(std::fs::read_to_string(p)?),
    };
    Ok((system, user))
}

fn create_client(
    models: Vec<ModelConfig>,
    providers: Vec<ModelProviderConfig>,
) -> anyhow::Result<Client> {
    Ok(Client::builder()
        .with_provider_config(models, providers)?
        .build())
}
