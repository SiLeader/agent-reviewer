extern crate agent_reviewer_agent;
extern crate agent_reviewer_model_provider;
extern crate agent_reviewer_tools;
extern crate genai;

use crate::prompt::PromptManager;
use clap::Parser;
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
    let prompts = PromptManager::new();

    orchestrator::run(config)
        .await
        .expect("Failed to run orchestrator");
}

fn load_config(config_path: impl AsRef<Path>) -> anyhow::Result<config::Config> {
    info!(
        "Loading configuration from: {}",
        config_path.as_ref().display()
    );
    let config: config::Config = toml::from_str(&std::fs::read_to_string(config_path)?)?;
    Ok(config)
}
