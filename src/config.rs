use agent_reviewer_model_provider::{ModelConfig, ModelProviderConfig};
use genai::chat::ReasoningEffort;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Config {
    models: Vec<ModelConfig>,
    model_providers: Vec<ModelProviderConfig>,
    agents: Vec<AgentModelConfig>,
    steps: StepsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StepsConfig {
    pub(crate) triage_agent: String,
    pub(crate) review_light_agent: String,
    pub(crate) review_standard_agent: String,
    pub(crate) review_power_agent: String,
    pub(crate) finalize_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AgentModelConfig {
    pub(crate) id: String,
    pub(crate) model: String,
    pub(crate) effort: Option<ReasoningEffortConfig>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) temperature: Option<f64>,
    pub(crate) top_p: Option<f64>,
    pub(crate) max_loops: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
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
