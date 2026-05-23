use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitTriage;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitTriageArgs {
    pub review_units: Vec<ReviewUnit>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewUnit {
    pub model: ReviewModel,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ReviewModel {
    Light,
    Standard,
    Power,
}

impl MarkerAgentTool for SubmitTriage {
    fn tool(&self) -> Tool {
        tool_description::<SubmitTriageArgs>("submitTriage", "Submit triage result")
    }
}
