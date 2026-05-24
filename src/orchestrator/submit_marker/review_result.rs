use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitReviewResult;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SubmitReviewResultArgs {
    #[schemars(required, description = "The review result.")]
    pub review_result: String,
}

impl MarkerAgentTool for SubmitReviewResult {
    fn tool(&self) -> Tool {
        tool_description::<SubmitReviewResultArgs>("submit_review_result", "Submit review result")
    }
}
