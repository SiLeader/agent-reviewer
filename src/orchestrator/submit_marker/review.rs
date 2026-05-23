use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitReview;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SubmitReviewArgs {}

impl MarkerAgentTool for SubmitReview {
    fn tool(&self) -> Tool {
        tool_description::<SubmitReviewArgs>("submitReview", "Submit review result")
    }
}
