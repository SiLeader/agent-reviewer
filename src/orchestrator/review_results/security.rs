use crate::orchestrator::ReviewedResultMarker;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SubmitSecurityReviewArgs {
    #[schemars(required, description = "The summary of the security review.")]
    pub summary: String,
    #[schemars(
        required,
        description = "The overall risk level of the reviewed changes."
    )]
    pub overall_risk: SecurityRisk,
    #[schemars(required, description = "The security findings from the review.")]
    pub findings: Vec<SecurityFinding>,
    #[schemars(
        required,
        description = "The security assumptions made during the review."
    )]
    pub assumptions: Vec<String>,
    #[schemars(
        required,
        description = "The unanswered security questions from the review."
    )]
    pub unanswered_questions: Vec<String>,
    #[schemars(
        required,
        description = "The confidence level of the security review.",
        range(min = 0, max = 1)
    )]
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SecurityFinding {
    #[schemars(required, description = "The severity of the security finding.")]
    pub severity: SecuritySeverity,
    #[schemars(required, description = "The category of the security finding.")]
    pub category: SecurityCategory,
    #[schemars(required, description = "The path of the security finding.")]
    pub path: Option<String>,
    #[schemars(required, description = "The line number of the security finding.")]
    pub line: Option<usize>,
    #[schemars(required, description = "The title of the security finding.")]
    pub title: String,
    #[schemars(
        required,
        description = "The evidence supporting the security finding."
    )]
    pub evidence: String,
    #[schemars(
        required,
        description = "The realistic attack scenario enabled by the finding."
    )]
    pub attack_scenario: Option<String>,
    #[schemars(required, description = "The security impact of the finding.")]
    pub impact: String,
    #[schemars(
        required,
        description = "The recommended action to address the security finding."
    )]
    pub recommendation: Option<String>,
    #[schemars(required, description = "The related CWE identifier, if known.")]
    pub cwe: Option<String>,
    #[schemars(required, description = "The related OWASP category, if known.")]
    pub owasp: Option<String>,
    #[schemars(
        required,
        description = "The external references relevant to the finding."
    )]
    pub references: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SecurityRisk {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SecuritySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SecurityCategory {
    Authentication,
    Authorization,
    Injection,
    CrossSiteScripting,
    Cryptography,
    Secrets,
    DataExposure,
    InputValidation,
    Dependency,
    Configuration,
    LoggingAndMonitoring,
    DenialOfService,
    SupplyChain,
    Other,
}

impl ReviewedResultMarker for SubmitSecurityReviewArgs {}
