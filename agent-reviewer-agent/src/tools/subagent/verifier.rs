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

use crate::ReActAgent;
use crate::tools::subagent::{run_subagent, setup_tools};
use agent_reviewer_tools::{AgentTool, MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERIFIER_TOOL_DESCRIPTION: &str = "Verify the review result for quality, completeness, and accuracy. Use this to validate that findings are well-grounded in evidence, severities are appropriate, recommendations are actionable, and no major issues were overlooked.";

pub struct Verifier {
    agent: ReActAgent,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VerifyArgs {
    #[schemars(required, description = "The task being reviewed.")]
    pub task: String,
    #[schemars(
        required,
        description = "The review result to verify. Include summary, findings, severities, and recommendations."
    )]
    pub review_result: String,
}

struct Submit;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VerifierResult {
    #[schemars(
        required,
        description = "Whether the verifier accepts or rejects the review result."
    )]
    pub decision: VerifierDecision,
    #[schemars(
        required,
        description = "Detailed reasons for accepting or rejecting the review result."
    )]
    pub reasons: Vec<String>,
    #[schemars(
        required,
        description = "Suggested improvements when the review is rejected. Empty if accepted."
    )]
    pub suggested_improvements: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerifierDecision {
    Accept,
    Reject,
}

impl MarkerAgentTool for Submit {
    fn tool(&self) -> Tool {
        tool_description::<VerifierResult>("submit", "Submit the verification decision")
    }
}

impl From<ReActAgent> for Verifier {
    fn from(mut agent: ReActAgent) -> Self {
        setup_tools(&mut agent, Submit);
        Self { agent }
    }
}

#[async_trait::async_trait]
impl AgentTool for Verifier {
    fn tool(&self) -> Tool {
        tool_description::<VerifyArgs>("verifier", VERIFIER_TOOL_DESCRIPTION)
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        run_subagent(&self.agent, VERIFIER_SYSTEM_PROMPT, args).await
    }
}

pub const VERIFIER_SYSTEM_PROMPT: &str = r#"You are a verifier for code reviews. Your job is to evaluate whether a review result is of sufficient quality and accept or reject it.

Accept criteria:
- Findings are grounded in specific evidence (file paths, line numbers, concrete code).
- Severities match the actual impact described.
- Recommendations are actionable and scoped to each finding.
- The confidence score aligns with the thoroughness of the review.
- No obviously critical or high-severity issues appear to have been missed for the reviewed scope.

Reject criteria:
- Findings lack concrete evidence (vague descriptions, no file/line references).
- Severities seem mismatched with described impact.
- Recommendations are generic or non-actionable.
- The review appears superficial or incomplete for the scope assigned.
- Obvious issues relevant to the task were not addressed.

Call `submit` exactly once with your decision, reasons, and any suggested improvements if rejected."#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConcurrencyLimiter;

    #[test]
    fn verifier_tool_has_correct_name_and_description() {
        let v = Verifier {
            agent: ReActAgent::builder(ConcurrencyLimiter::new(1)).build(),
        };
        let tool = v.tool();
        assert_eq!(tool.name.to_string(), "verifier");
        assert!(
            tool.description
                .as_ref()
                .unwrap()
                .contains("Verify the review result")
        );
    }

    #[test]
    fn verifier_result_serializes_and_deserializes() {
        let result = VerifierResult {
            decision: VerifierDecision::Reject,
            reasons: vec!["Findings lack file references".to_string()],
            suggested_improvements: vec![
                "Include specific file paths for each finding.".to_string(),
                "Add line numbers where possible.".to_string(),
            ],
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["decision"], "reject");
        assert_eq!(json["reasons"].as_array().unwrap().len(), 1);
        assert_eq!(json["suggested_improvements"].as_array().unwrap().len(), 2);

        let deserialized: VerifierResult = serde_json::from_value(json).unwrap();
        assert!(matches!(deserialized.decision, VerifierDecision::Reject));
    }

    #[test]
    fn verifier_accept_result_serializes() {
        let result = VerifierResult {
            decision: VerifierDecision::Accept,
            reasons: vec!["All findings are well-grounded.".to_string()],
            suggested_improvements: vec![],
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["decision"], "accept");
    }
}
