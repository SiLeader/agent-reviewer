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
    pub task: String,
    pub focus_files: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_review_units_with_task_and_focus_files() {
        let args: SubmitTriageArgs = serde_json::from_value(serde_json::json!({
            "reviewUnits": [
                {
                    "task": "Review error handling in the CLI entrypoint",
                    "focusFiles": ["src/main.rs"],
                    "model": "standard"
                }
            ]
        }))
        .unwrap();

        assert_eq!(args.review_units.len(), 1);
        assert_eq!(
            args.review_units[0].task,
            "Review error handling in the CLI entrypoint"
        );
        assert_eq!(args.review_units[0].focus_files, vec!["src/main.rs"]);
        assert!(matches!(args.review_units[0].model, ReviewModel::Standard));
    }
}
