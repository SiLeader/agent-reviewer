use crate::orchestrator::ReviewedResultMarker;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

impl ReviewedResultMarker for SubmitReviewArgs {}
