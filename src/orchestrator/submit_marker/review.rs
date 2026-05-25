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

use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitReview;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SubmitReviewArgs {
    #[schemars(required, description = "The summary of the review.")]
    pub summary: String,
    #[schemars(required, description = "The findings of the review.")]
    pub findings: Vec<ReviewFinding>,
    #[schemars(required, description = "The unanswered questions from the review.")]
    pub unanswered_questions: Vec<String>,
    #[schemars(
        required,
        description = "The confidence level of the review.",
        range(min = 0, max = 1)
    )]
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ReviewFinding {
    #[schemars(required, description = "The severity of the finding.")]
    pub severity: ReviewSeverity,
    #[schemars(required, description = "The category of the finding.")]
    pub category: ReviewCategory,
    #[schemars(required, description = "The path of the finding.")]
    pub path: Option<String>,
    #[schemars(required, description = "The line number of the finding.")]
    pub line: Option<usize>,
    #[schemars(required, description = "The title of the finding.")]
    pub title: String,
    #[schemars(required, description = "The comment explaining the finding.")]
    pub comment: String,
    #[schemars(
        required,
        description = "The recommended action to address the finding."
    )]
    pub recommendation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewCategory {
    Bug,
    Security,
    Performance,
    Maintainability,
    Test,
    Documentation,
    Other,
}

impl MarkerAgentTool for SubmitReview {
    fn tool(&self) -> Tool {
        tool_description::<SubmitReviewArgs>("submit_review", "Submit review result")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_review_with_findings() {
        let args: SubmitReviewArgs = serde_json::from_value(serde_json::json!({
            "summary": "Found one high impact issue.",
            "findings": [
                {
                    "severity": "high",
                    "category": "bug",
                    "path": "src/main.rs",
                    "line": 42,
                    "title": "Fallback hides write failures",
                    "comment": "The code can silently discard the intended output file.",
                    "recommendation": "Return the write error unless fallback is enabled."
                }
            ],
            "unanswered_questions": [],
            "confidence": 0.85
        }))
        .unwrap();

        assert_eq!(args.summary, "Found one high impact issue.");
        assert_eq!(args.findings.len(), 1);
        assert!(matches!(args.findings[0].severity, ReviewSeverity::High));
        assert!(matches!(args.findings[0].category, ReviewCategory::Bug));
        assert_eq!(args.findings[0].path.as_deref(), Some("src/main.rs"));
        assert_eq!(args.findings[0].line, Some(42));
        assert_eq!(args.confidence, 0.85);
    }

    #[test]
    fn deserializes_review_without_findings() {
        let args: SubmitReviewArgs = serde_json::from_value(serde_json::json!({
            "summary": "No actionable findings.",
            "findings": [],
            "unanswered_questions": ["Whether generated files are in scope."],
            "confidence": 0.6
        }))
        .unwrap();

        assert!(args.findings.is_empty());
        assert_eq!(
            args.unanswered_questions,
            vec!["Whether generated files are in scope."]
        );
    }

    #[test]
    fn schema_contains_confidence_range() {
        let schema = schemars::schema_for!(SubmitReviewArgs);
        let value = serde_json::to_value(schema).unwrap();
        let confidence = &value["properties"]["confidence"];

        assert_eq!(confidence["minimum"], 0.0);
        assert_eq!(confidence["maximum"], 1.0);
    }
}
