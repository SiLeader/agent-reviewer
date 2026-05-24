use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitReview;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SubmitReviewArgs {
    pub summary: String,
    pub findings: Vec<ReviewFinding>,
    pub unanswered_questions: Vec<String>,
    #[schemars(range(min = 0, max = 1))]
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ReviewFinding {
    pub severity: ReviewSeverity,
    pub category: ReviewCategory,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub title: String,
    pub comment: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) enum ReviewSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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
            "unansweredQuestions": [],
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
            "unansweredQuestions": ["Whether generated files are in scope."],
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
