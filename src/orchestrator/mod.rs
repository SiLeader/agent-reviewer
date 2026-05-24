use crate::config::{AgentModelConfig, StepsConfig, SubagentConfig};
use crate::orchestrator::submit_marker::{
    ReviewModel, ReviewUnit, SubmitReview, SubmitReviewArgs, SubmitReviewResult,
    SubmitReviewResultArgs, SubmitTriage, SubmitTriageArgs,
};
use crate::prompt::PromptManager;
use agent_reviewer_agent::ReActAgent;
use agent_reviewer_agent::builder::ReActAgentBuilder;
use agent_reviewer_agent::tools::subagent::Explorer;
use agent_reviewer_tools::fs::{ListFiles, ReadFile};
use agent_reviewer_tools::git::{
    GitCurrentBranch, GitDefaultBranch, GitDiffCommitRange, GitDiffSingleCommit,
    GitDiffSummaryCommitRange, GitDiffSummarySingleCommit, GitPrBaseBranch,
};
use agent_reviewer_tools::{AgentTool, MarkerAgentTool};
use futures::future::join_all;
use genai::chat::ChatOptions;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub mod submit_marker;

pub(crate) struct Orchestrator {
    prompts: PromptManager,
    agent_builder: ReActAgentBuilder,
    steps: StepsConfig,
    subagent: SubagentConfig,
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
        subagent: SubagentConfig,
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
            subagent,
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
        let options = ChatOptions::default()
            .with_max_tokens(config.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS))
            .with_temperature(config.temperature.unwrap_or(DEFAULT_TEMPERATURE))
            .with_top_p(config.top_p.unwrap_or(DEFAULT_TOP_P));

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
