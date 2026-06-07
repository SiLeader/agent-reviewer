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

use crate::config::{AgentModelConfig, StepsConfig, SubagentConfig};
pub(crate) use crate::orchestrator::review_results::*;
use crate::orchestrator::submit_marker::{
    ReviewModel, ReviewUnit, SubmitReview, SubmitReviewResult, SubmitReviewResultArgs,
    SubmitTriage, SubmitTriageArgs,
};
use crate::prompt::PromptManager;
use agent_reviewer_agent::ReActAgent;
use agent_reviewer_agent::builder::ReActAgentBuilder;
use agent_reviewer_agent::tools::subagent::{
    Advisor, Explorer, Verifier, VerifierDecision, VerifierResult, VerifyArgs,
};
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

mod review_results;
pub mod submit_marker;

pub(crate) struct Orchestrator<R> {
    prompts: PromptManager<R>,
    agent_builder: ReActAgentBuilder,
    steps: StepsConfig,
    subagent: SubagentConfig,
    model_config: HashMap<String, ModelConfig>,
    agent_model_config: HashMap<String, AgentModelConfig>,
}

const DEFAULT_MAX_LOOP_COUNT: usize = 6;
const DEFAULT_MAX_TOKENS: u32 = 50000;
/// Maximum number of verifier retry attempts per review unit.
const MAX_VERIFIER_RETRIES: usize = 3;

fn get_ro_toolset(additional: &[Option<Arc<dyn AgentTool>>]) -> Vec<Arc<dyn AgentTool>> {
    let mut tools: Vec<Arc<dyn AgentTool>> = vec![
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
    ];
    for tool in additional.iter().flatten() {
        tools.push(tool.clone());
    }
    tools
}

impl<R> Orchestrator<R>
where
    R: ReviewedResultMarker,
{
    pub fn new(
        prompts: PromptManager<R>,
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
        id: &str,
        tools: Vec<Arc<dyn AgentTool>>,
        marker_tools: Vec<Arc<dyn MarkerAgentTool>>,
        submit_tool: Option<Arc<dyn MarkerAgentTool>>,
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

        let builder = self
            .agent_builder
            .clone()
            .override_model_name(config.model.clone())
            .override_max_loop_count(config.max_loops.unwrap_or(DEFAULT_MAX_LOOP_COUNT))
            .override_options(options)
            .override_tools(tools)
            .override_marker_tools(marker_tools);
        let builder = if let Some(submit_tool) = submit_tool {
            builder.override_submit_tool(submit_tool)
        } else {
            builder
        };
        Ok(builder.build())
    }

    pub async fn run(&self, prompt: Option<String>) -> anyhow::Result<String> {
        let explorer = Arc::new(Explorer::from(self.build_agent(
            &self.subagent.explorer.agent,
            vec![],
            vec![],
            None,
        )?));

        let agent = self.build_agent(
            &self.steps.triage.agent,
            get_ro_toolset(&[Some(explorer.clone())]),
            vec![],
            Some(Arc::new(SubmitTriage)),
        )?;
        let system_prompt = self.prompts.render_triage_system()?;
        let user_prompt = self.prompts.render_triage_user(prompt)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;

        let result: SubmitTriageArgs = serde_json::from_value(result)?;
        let results =
            join_all(result.review_units.into_iter().map(|unit| async {
                (unit.clone(), self.run_review(unit, explorer.clone()).await)
            }))
            .await;

        let (successes, failures) = {
            let mut successes = vec![];
            let mut failures = vec![];
            for (unit, result) in results {
                match result {
                    Ok(data) => {
                        successes.push(data);
                    }
                    Err(err) => {
                        tracing::error!("Error running review for unit {}: {}", unit.task, err);
                        failures.push(unit);
                    }
                }
            }
            (successes, failures)
        };

        let agent = self.build_agent(
            &self.steps.finalize.agent,
            vec![],
            vec![],
            Some(Arc::new(SubmitReviewResult)),
        )?;
        let system_prompt = self.prompts.render_finalize_system()?;
        let user_prompt = self.prompts.render_finalize_user(&successes, &failures)?;
        let result = agent.run(&system_prompt, &user_prompt).await?;

        let result: SubmitReviewResultArgs = serde_json::from_value(result)?;
        Ok(result.review_result)
    }

    async fn run_review(&self, unit: ReviewUnit, explorer: Arc<Explorer>) -> anyhow::Result<R> {
        let review_conf = match unit.model {
            ReviewModel::Light => &self.steps.review.light,
            ReviewModel::Standard => &self.steps.review.standard,
            ReviewModel::Power => &self.steps.review.power,
        };

        // Build verifier agent if configured
        let verifier_agent: Option<Arc<Verifier>> =
            if let Some(verifier_id) = &review_conf.verifier_agent {
                Some(Arc::new(Verifier::from(self.build_agent(
                    verifier_id,
                    vec![],
                    vec![],
                    None,
                )?)))
            } else {
                None
            };

        // Run initial review, then verify and retry if rejected
        let mut verifier_feedback: Option<String> = None;
        for attempt in 0..=MAX_VERIFIER_RETRIES {
            let advisor_agent = if let Some(a) = &review_conf.advisor_agent {
                Some(
                    Arc::new(Advisor::from(self.build_agent(a, vec![], vec![], None)?))
                        as Arc<dyn AgentTool>,
                )
            } else {
                None
            };

            let agent = self.build_agent(
                &review_conf.main_agent,
                get_ro_toolset(&[Some(explorer.clone()), advisor_agent]),
                vec![],
                Some(Arc::new(SubmitReview::<R>::default())),
            )?;
            let system_prompt = self.prompts.render_review_system()?;

            // On retry, inject verifier feedback into the user prompt
            let user_prompt = if let Some(ref feedback) = verifier_feedback {
                let base_user = self.prompts.render_review_user(&unit)?;
                format!(
                    "{}\n\n## Verifier Feedback (from previous review attempt)\n{}\n\nPlease address the verifier's concerns in your review.",
                    base_user, feedback
                )
            } else {
                self.prompts.render_review_user(&unit)?
            };

            let result = agent.run(&system_prompt, &user_prompt).await?;
            let result: R = serde_json::from_value(result)?;

            // If no verifier is configured, return immediately
            let Some(verifier) = verifier_agent.as_ref() else {
                return Ok(result);
            };

            // Verify the review result
            let verifier_result = verify_review(verifier.as_ref(), &unit.task, &result).await?;

            match verifier_result.decision {
                VerifierDecision::Accept => {
                    tracing::info!(
                        "Verifier accepted review for unit '{}' (attempt {}). Reasons: {:?}",
                        unit.task,
                        attempt + 1,
                        verifier_result.reasons
                    );
                    return Ok(result);
                }
                VerifierDecision::Reject => {
                    let feedback = format!(
                        "Verdict: REJECT\nReasons:\n{}\nSuggested improvements:\n{}",
                        verifier_result
                            .reasons
                            .iter()
                            .map(|r| format!("- {}", r))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        verifier_result
                            .suggested_improvements
                            .iter()
                            .map(|i| format!("- {}", i))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );

                    if attempt < MAX_VERIFIER_RETRIES {
                        tracing::warn!(
                            "Verifier rejected review for unit '{}' (attempt {}/{})\n{}",
                            unit.task,
                            attempt + 1,
                            MAX_VERIFIER_RETRIES + 1,
                            feedback
                        );
                        verifier_feedback = Some(feedback);
                    } else {
                        // Max retries exceeded; return the last review result anyway with a warning
                        tracing::warn!(
                            "Verifier rejected review for unit '{}' after {} retry attempts. Returning last review result.",
                            unit.task,
                            MAX_VERIFIER_RETRIES + 1
                        );
                        return Ok(result);
                    }
                }
            }
        }

        // This should be unreachable due to the return in the loop, but satisfy the compiler
        anyhow::bail!(
            "Unexpected verifier loop completion for unit '{}'",
            unit.task
        )
    }
}

/// Verify a review result using the verifier agent. Returns the verifier's decision and reasoning.
async fn verify_review<R: serde::Serialize>(
    verifier: &Verifier,
    task: &str,
    review_result: &R,
) -> anyhow::Result<VerifierResult> {
    use agent_reviewer_tools::AgentTool;

    let review_json = serde_json::to_string(review_result)
        .map_err(|e| anyhow::anyhow!("Failed to serialize review result for verifier: {}", e))?;

    let args = VerifyArgs {
        task: task.to_string(),
        review_result: review_json,
    };

    let args_value = serde_json::to_value(&args)?;
    let result_str = verifier.run(&args_value).await?;
    let verifier_result: VerifierResult = serde_json::from_str(&result_str)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize verifier result: {}", e))?;

    Ok(verifier_result)
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
