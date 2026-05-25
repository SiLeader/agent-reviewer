use agent_reviewer_model_provider::{ModelConfig, ModelProviderConfig};
use genai::chat::ReasoningEffort;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Config {
    pub(crate) models: Vec<ModelConfig>,
    pub(crate) model_providers: Vec<ModelProviderConfig>,
    pub(crate) agents: Vec<AgentModelConfig>,
    pub(crate) steps: StepsConfig,
    pub(crate) subagent: SubagentConfig,
    #[serde(default)]
    pub(crate) prompt: StepsPromptConfig,
    #[serde(default = "default_concurrency")]
    pub(crate) concurrency: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StepsConfig {
    pub(crate) triage: TriageStepConfig,
    pub(crate) review: ReviewStepConfig,
    pub(crate) finalize: FinalizeStepConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TriageStepConfig {
    pub(crate) agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReviewStepConfig {
    pub(crate) light: ReviewStepAgentConfig,
    pub(crate) standard: ReviewStepAgentConfig,
    pub(crate) power: ReviewStepAgentConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FinalizeStepConfig {
    pub(crate) agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReviewStepAgentConfig {
    pub(crate) main_agent: String,
    pub(crate) advisor_agent: Option<String>,
    // pub(crate) verifier_agent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SubagentConfig {
    pub(crate) explorer: ExplorerSubagentConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ExplorerSubagentConfig {
    pub(crate) agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AgentModelConfig {
    pub(crate) id: String,
    pub(crate) model: String,
    pub(crate) effort: Option<ReasoningEffortConfig>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) max_loops: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct PromptConfig {
    pub(crate) user_template_file: Option<String>,
    pub(crate) system_file: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct StepsPromptConfig {
    pub(crate) triage: PromptConfig,
    pub(crate) review: PromptConfig,
    pub(crate) finalize: PromptConfig,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffortConfig {
    None,
    Low,
    Medium,
    High,
    XHigh,
    Max,
    Budget(u32),
    Minimal,
}

fn default_concurrency() -> usize {
    1
}

impl From<ReasoningEffortConfig> for ReasoningEffort {
    fn from(effort: ReasoningEffortConfig) -> Self {
        match effort {
            ReasoningEffortConfig::None => ReasoningEffort::None,
            ReasoningEffortConfig::Low => ReasoningEffort::Low,
            ReasoningEffortConfig::Medium => ReasoningEffort::Medium,
            ReasoningEffortConfig::High => ReasoningEffort::High,
            ReasoningEffortConfig::XHigh => ReasoningEffort::XHigh,
            ReasoningEffortConfig::Max => ReasoningEffort::Max,
            ReasoningEffortConfig::Budget(budget) => ReasoningEffort::Budget(budget),
            ReasoningEffortConfig::Minimal => ReasoningEffort::Minimal,
        }
    }
}
