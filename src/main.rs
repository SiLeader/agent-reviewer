// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate agent_reviewer_agent;
extern crate agent_reviewer_model_provider;
extern crate agent_reviewer_tools;
extern crate genai;

use crate::orchestrator::{SubmitReviewArgs, SubmitSecurityReviewArgs};
use clap::Parser;
use std::path::Path;
use tracing::{debug, info};

mod config;
mod instruction;
mod orchestrator;
mod prompt;
mod run;

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

    #[arg(short, long, help = "Run the security review.")]
    security_review: bool,

    #[arg(short, long, help = "Unique identifier for the review session")]
    id: Option<String>,

    #[arg(help = "Prompt to use for the review step.")]
    prompt: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let config = match load_config(args.config.clone()) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Failed to load configuration: {}", err);
            std::process::exit(1);
        }
    };
    debug!("Configuration loaded: {:?}", config);

    if args.security_review {
        run::run::<SubmitSecurityReviewArgs>(args, config).await;
    } else {
        run::run::<SubmitReviewArgs>(args, config).await;
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
