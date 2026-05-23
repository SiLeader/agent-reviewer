use crate::config::{AgentModelConfig, StepsConfig};
use crate::orchestrator::submit_marker::{
    ReviewModel, ReviewUnit, SubmitReviewArgs, SubmitReviewResultArgs, SubmitTriageArgs,
};
use crate::prompt::PromptManager;
use agent_reviewer_agent::ReActAgent;
use agent_reviewer_agent::builder::ReActAgentBuilder;
use futures::future::join_all;
use genai::chat::ChatOptions;
use schemars::_private::serde_json;
use std::collections::HashMap;

pub mod submit_marker;

pub(crate) struct Orchestrator {
    instruction: String,
    prompts: PromptManager,
    agent_builder: ReActAgentBuilder,
    steps: StepsConfig,
    agent_model_config: HashMap<String, AgentModelConfig>,
}

const DEFAULT_MAX_LOOP_COUNT: usize = 6;
const DEFAULT_MAX_TOKENS: u32 = 1000;
const DEFAULT_TEMPERATURE: f64 = 0.7;
const DEFAULT_TOP_P: f64 = 0.9;

impl Orchestrator {
    pub fn new(
        prompts: PromptManager,
        agent_builder: ReActAgentBuilder,
        steps: StepsConfig,
        agent_model_config: Vec<AgentModelConfig>,
    ) -> Self {
        let agent_model_config = agent_model_config
            .into_iter()
            .map(|config| (config.id.clone(), config))
            .collect();

        Self {
            prompts,
            agent_builder,
            steps,
            agent_model_config,
        }
    }

    fn build_agent(&self, id: &str) -> anyhow::Result<ReActAgent> {
        let Some(config) = self.agent_model_config.get(id) else {
            anyhow::bail!("Agent model configuration not found for ID: {}", id);
        };
        let options = ChatOptions::default()
            .with_max_tokens(config.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS))
            .with_temperature(config.temperature.unwrap_or(DEFAULT_TEMPERATURE))
            .with_top_p(config.top_p.unwrap_or(DEFAULT_TOP_P));

        Ok(self
            .agent_builder
            .clone()
            .override_max_loop_count(config.max_loops.unwrap_or(DEFAULT_MAX_LOOP_COUNT))
            .override_options(options)
            .build())
    }

    pub async fn run(&self, prompt: String) -> anyhow::Result<String> {
        let agent = self.build_agent(&self.steps.triage_agent)?;
        let system_prompt = self.prompts.render_triage_system()?;
        let user_prompt = self.prompts.render_triage_user(&prompt)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;

        let result: SubmitTriageArgs = serde_json::from_value(result)?;
        let results = join_all(
            result
                .review_units
                .into_iter()
                .map(|unit| self.run_review(unit)),
        )
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

        let agent = self.build_agent(&self.steps.finalize_agent)?;
        let system_prompt = self.prompts.render_finalize_system()?;
        let user_prompt = self.prompts.render_finalize_user(&results)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;

        let result: SubmitReviewResultArgs = serde_json::from_value(result)?;
        Ok(result.review_result)
    }

    async fn run_review(&self, unit: ReviewUnit) -> anyhow::Result<SubmitReviewArgs> {
        let agent = self.build_agent(match unit.model {
            ReviewModel::Light => &self.steps.review_light_agent,
            ReviewModel::Standard => &self.steps.review_standard_agent,
            ReviewModel::Power => &self.steps.review_power_agent,
        })?;
        let system_prompt = self.prompts.render_review_system()?;
        let user_prompt = self.prompts.render_review_user(&unit)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;
        let result: SubmitReviewArgs = serde_json::from_value(result)?;
        Ok(result)
    }
}
