use crate::config::{AgentModelConfig, StepsConfig, SubagentConfig};
use crate::orchestrator::submit_marker::{
    ReviewModel, ReviewUnit, SubmitReview, SubmitReviewArgs, SubmitReviewResult,
    SubmitReviewResultArgs, SubmitTriage, SubmitTriageArgs,
};
use crate::prompt::PromptManager;
use agent_reviewer_agent::ReActAgent;
use agent_reviewer_agent::builder::ReActAgentBuilder;
use agent_reviewer_agent::tools::subagent::Explorer;
use agent_reviewer_model_provider::ModelConfig;
use agent_reviewer_tools::fs::{ListFiles, ReadFile, SearchFile};
use agent_reviewer_tools::git::{
    GitCurrentBranch, GitDefaultBranch, GitDiffCommitRange, GitDiffSingleCommit,
    GitDiffSummaryCommitRange, GitDiffSummarySingleCommit, GitPrBaseBranch,
};
use agent_reviewer_tools::{AgentTool, MarkerAgentTool};
use futures::future::join_all;
use genai::chat::ChatOptions;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub mod submit_marker;

pub(crate) struct Orchestrator {
    prompts: PromptManager,
    agent_builder: ReActAgentBuilder,
    steps: StepsConfig,
    subagent: SubagentConfig,
    model_config: HashMap<String, ModelConfig>,
    agent_model_config: HashMap<String, AgentModelConfig>,
}

const DEFAULT_MAX_LOOP_COUNT: usize = 6;
const DEFAULT_MAX_TOKENS: u32 = 1000;

impl Orchestrator {
    pub fn new(
        prompts: PromptManager,
        agent_builder: ReActAgentBuilder,
        steps: StepsConfig,
        subagent: SubagentConfig,
        model_config: Vec<ModelConfig>,
        agent_model_config: Vec<AgentModelConfig>,
    ) -> Self {
        let model_config = model_config
            .into_iter()
            .map(|config| (config.id().to_string(), config))
            .collect();
        let agent_model_config = agent_model_config
            .into_iter()
            .map(|config| (config.id.clone(), config))
            .collect();

        Self {
            prompts,
            agent_builder,
            steps,
            subagent,
            model_config,
            agent_model_config,
        }
    }

    fn build_agent(
        &self,
        agent_name: &str,
        id: &str,
        tools: Vec<Arc<dyn AgentTool>>,
        marker_tools: Vec<Arc<dyn MarkerAgentTool>>,
    ) -> anyhow::Result<ReActAgent> {
        let Some(config) = self.agent_model_config.get(id) else {
            anyhow::bail!("Agent model configuration not found for ID: {}", id);
        };
        let Some(model) = self.model_config.get(&config.model) else {
            anyhow::bail!("Model configuration not found for ID: {}", config.model);
        };
        let max_tokens = config.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        let options = chat_options_for_model(model.model(), max_tokens);

        let options = match config.effort {
            None => options,
            Some(effort) => options.with_reasoning_effort(effort.into()),
        };

        Ok(self
            .agent_builder
            .clone()
            .override_model_name(config.model.clone())
            .override_max_loop_count(config.max_loops.unwrap_or(DEFAULT_MAX_LOOP_COUNT))
            .override_options(options)
            .override_tools(tools)
            .override_marker_tools(marker_tools)
            .build(format!("{agent_name}-{}", Uuid::now_v7())))
    }

    pub async fn run(&self, prompt: Option<String>) -> anyhow::Result<String> {
        let explorer = Arc::new(Explorer::from(self.build_agent(
            "exporter",
            &self.subagent.explorer.agent,
            vec![],
            vec![],
        )?));

        let agent = self.build_agent(
            "triage",
            &self.steps.triage_agent,
            vec![
                Arc::new(ReadFile),
                Arc::new(ListFiles),
                Arc::new(SearchFile),
                Arc::new(GitDiffSingleCommit),
                Arc::new(GitDiffCommitRange),
                Arc::new(GitDiffSummarySingleCommit),
                Arc::new(GitDiffSummaryCommitRange),
                Arc::new(GitPrBaseBranch),
                Arc::new(GitDefaultBranch),
                Arc::new(GitCurrentBranch),
                explorer.clone(),
            ],
            vec![Arc::new(SubmitTriage)],
        )?;
        let system_prompt = self.prompts.render_triage_system()?;
        let user_prompt = self.prompts.render_triage_user(prompt)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;

        let result: SubmitTriageArgs = serde_json::from_value(result)?;
        let results = join_all(
            result
                .review_units
                .into_iter()
                .map(|unit| self.run_review(unit, explorer.clone())),
        )
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

        let agent = self.build_agent(
            "finalize",
            &self.steps.finalize_agent,
            vec![],
            vec![Arc::new(SubmitReviewResult)],
        )?;
        let system_prompt = self.prompts.render_finalize_system()?;
        let user_prompt = self.prompts.render_finalize_user(&results)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;

        let result: SubmitReviewResultArgs = serde_json::from_value(result)?;
        Ok(result.review_result)
    }

    async fn run_review(
        &self,
        unit: ReviewUnit,
        explorer: Arc<Explorer>,
    ) -> anyhow::Result<SubmitReviewArgs> {
        let agent = self.build_agent(
            "review",
            match unit.model {
                ReviewModel::Light => &self.steps.review_light_agent,
                ReviewModel::Standard => &self.steps.review_standard_agent,
                ReviewModel::Power => &self.steps.review_power_agent,
            },
            vec![
                Arc::new(ReadFile),
                Arc::new(ListFiles),
                Arc::new(SearchFile),
                Arc::new(GitDiffSingleCommit),
                Arc::new(GitDiffCommitRange),
                Arc::new(GitDiffSummarySingleCommit),
                Arc::new(GitDiffSummaryCommitRange),
                Arc::new(GitPrBaseBranch),
                Arc::new(GitDefaultBranch),
                Arc::new(GitCurrentBranch),
                explorer,
            ],
            vec![Arc::new(SubmitReview)],
        )?;
        let system_prompt = self.prompts.render_review_system()?;
        let user_prompt = self.prompts.render_review_user(&unit)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;
        let result: SubmitReviewArgs = serde_json::from_value(result)?;
        Ok(result)
    }
}

fn chat_options_for_model(model_name: &str, max_tokens: u32) -> ChatOptions {
    if uses_max_completion_tokens(model_name) {
        ChatOptions::default().with_extra_body(json!({
            "max_completion_tokens": max_tokens,
        }))
    } else {
        ChatOptions::default().with_max_tokens(max_tokens)
    }
}

fn uses_max_completion_tokens(model_name: &str) -> bool {
    let model_name = model_name
        .split_once('/')
        .map_or(model_name, |(_, model_name)| model_name);

    model_name.starts_with("gpt-5")
}

#[cfg(test)]
mod tests {
    use super::{chat_options_for_model, uses_max_completion_tokens};

    #[test]
    fn uses_max_completion_tokens_for_gpt_5_models() {
        assert!(uses_max_completion_tokens("openai/gpt-5-mini"));
        assert!(uses_max_completion_tokens("gpt-5"));
        assert!(!uses_max_completion_tokens("openai/gpt-4.1"));
    }

    #[test]
    fn stores_max_completion_tokens_in_extra_body_for_gpt_5_models() {
        let options = chat_options_for_model("openai/gpt-5-mini", 2048);

        assert_eq!(options.max_tokens, None);
        assert_eq!(
            options.extra_body,
            Some(serde_json::json!({
                "max_completion_tokens": 2048,
            }))
        );
    }

    #[test]
    fn keeps_max_tokens_for_older_models() {
        let options = chat_options_for_model("openai/gpt-4.1", 1024);

        assert_eq!(options.max_tokens, Some(1024));
        assert_eq!(options.extra_body, None);
    }
}
