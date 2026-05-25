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

use crate::Args;
use crate::config::{Config, PromptConfig, StepsPromptConfig};
use crate::instruction::load_instructions;
use crate::orchestrator::{Orchestrator, ReviewedResultMarker};
use crate::prompt::PromptManager;
use agent_reviewer_agent::ConcurrencyLimiter;
use agent_reviewer_agent::builder::ReActAgentBuilder;
use agent_reviewer_model_provider::{ModelConfig, ModelProviderConfig, WithProviderConfig};
use genai::Client;
use serde::Serialize;
use tracing::info;

pub(crate) async fn run<R>(args: Args, config: Config)
where
    R: ReviewedResultMarker,
{
    let prompts = match load_prompt::<R>(&config.prompt) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to load prompt: {}", e);
            std::process::exit(1);
        }
    };
    let models = config.models.clone();
    let client = match create_client(config.models, config.model_providers) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create client: {}", e);
            std::process::exit(1);
        }
    };
    let session_id = args.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    info!("Starting review session with ID: {}", session_id);
    let concurrency_limiter = ConcurrencyLimiter::new(config.concurrency);
    let agent_builder =
        ReActAgentBuilder::new(session_id, concurrency_limiter).override_client(client);

    let orchestrator = Orchestrator::new(
        prompts,
        agent_builder,
        config.steps,
        config.subagent,
        models,
        config.agents,
    );
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

fn load_prompt<R>(config: &StepsPromptConfig) -> anyhow::Result<PromptManager<R>>
where
    R: Serialize,
{
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
