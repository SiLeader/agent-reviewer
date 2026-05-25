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

use crate::orchestrator::submit_marker::ReviewUnit;
use minijinja::Environment;
use serde::Serialize;

pub(crate) struct PromptManager<R> {
    instructions: Option<String>,
    templates: Environment<'static>,
    triage_system: String,
    review_system: String,
    finalize_system: String,
    _phantom: std::marker::PhantomData<R>,
}

const TRIAGE_USER_KEY: &str = "triage_user";
const REVIEW_USER_KEY: &str = "review_user";
const FINALIZE_USER_KEY: &str = "finalize_user";

const DEFAULT_NORMAL_TRIAGE_SYSTEM: &str = include_str!("default_prompts/normal/triage/system.md");
const DEFAULT_NORMAL_TRIAGE_USER: &str =
    include_str!("default_prompts/normal/triage/user.md.jinja2");
const DEFAULT_NORMAL_REVIEW_SYSTEM: &str = include_str!("default_prompts/normal/review/system.md");
const DEFAULT_NORMAL_REVIEW_USER: &str =
    include_str!("default_prompts/normal/review/user.md.jinja2");
const DEFAULT_NORMAL_FINALIZE_SYSTEM: &str =
    include_str!("default_prompts/normal/finalize/system.md");
const DEFAULT_NORMAL_FINALIZE_USER: &str =
    include_str!("default_prompts/normal/finalize/user.md.jinja2");

const DEFAULT_SECURITY_TRIAGE_SYSTEM: &str =
    include_str!("default_prompts/security/triage/system.md");
const DEFAULT_SECURITY_TRIAGE_USER: &str =
    include_str!("default_prompts/security/triage/user.md.jinja2");
const DEFAULT_SECURITY_REVIEW_SYSTEM: &str =
    include_str!("default_prompts/security/review/system.md");
const DEFAULT_SECURITY_REVIEW_USER: &str =
    include_str!("default_prompts/security/review/user.md.jinja2");
const DEFAULT_SECURITY_FINALIZE_SYSTEM: &str =
    include_str!("default_prompts/security/finalize/system.md");
const DEFAULT_SECURITY_FINALIZE_USER: &str =
    include_str!("default_prompts/security/finalize/user.md.jinja2");

pub(crate) enum PromptProfile {
    Normal,
    Security,
}

#[derive(Default)]
pub(crate) struct PromptOverrides {
    pub(crate) instructions: Option<String>,
    pub(crate) triage_system: Option<String>,
    pub(crate) triage_user: Option<String>,
    pub(crate) review_system: Option<String>,
    pub(crate) review_user: Option<String>,
    pub(crate) finalize_system: Option<String>,
    pub(crate) finalize_user: Option<String>,
}

struct DefaultPrompts {
    triage_system: &'static str,
    triage_user: &'static str,
    review_system: &'static str,
    review_user: &'static str,
    finalize_system: &'static str,
    finalize_user: &'static str,
}

impl PromptProfile {
    fn defaults(&self) -> DefaultPrompts {
        match self {
            Self::Normal => DefaultPrompts {
                triage_system: DEFAULT_NORMAL_TRIAGE_SYSTEM,
                triage_user: DEFAULT_NORMAL_TRIAGE_USER,
                review_system: DEFAULT_NORMAL_REVIEW_SYSTEM,
                review_user: DEFAULT_NORMAL_REVIEW_USER,
                finalize_system: DEFAULT_NORMAL_FINALIZE_SYSTEM,
                finalize_user: DEFAULT_NORMAL_FINALIZE_USER,
            },
            Self::Security => DefaultPrompts {
                triage_system: DEFAULT_SECURITY_TRIAGE_SYSTEM,
                triage_user: DEFAULT_SECURITY_TRIAGE_USER,
                review_system: DEFAULT_SECURITY_REVIEW_SYSTEM,
                review_user: DEFAULT_SECURITY_REVIEW_USER,
                finalize_system: DEFAULT_SECURITY_FINALIZE_SYSTEM,
                finalize_user: DEFAULT_SECURITY_FINALIZE_USER,
            },
        }
    }
}

impl<R> PromptManager<R>
where
    R: Serialize,
{
    pub fn new(profile: PromptProfile, overrides: PromptOverrides) -> anyhow::Result<Self> {
        let mut templates = Environment::new();
        let defaults = profile.defaults();

        templates.add_template_owned(
            TRIAGE_USER_KEY,
            overrides
                .triage_user
                .unwrap_or_else(|| defaults.triage_user.to_string()),
        )?;
        templates.add_template_owned(
            REVIEW_USER_KEY,
            overrides
                .review_user
                .unwrap_or_else(|| defaults.review_user.to_string()),
        )?;
        templates.add_template_owned(
            FINALIZE_USER_KEY,
            overrides
                .finalize_user
                .unwrap_or_else(|| defaults.finalize_user.to_string()),
        )?;

        Ok(Self {
            instructions: overrides.instructions,
            templates,
            triage_system: overrides
                .triage_system
                .unwrap_or_else(|| defaults.triage_system.to_string()),
            review_system: overrides
                .review_system
                .unwrap_or_else(|| defaults.review_system.to_string()),
            finalize_system: overrides
                .finalize_system
                .unwrap_or_else(|| defaults.finalize_system.to_string()),
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn render_triage_system(&self) -> anyhow::Result<String> {
        Ok(self.triage_system.clone())
    }

    pub fn render_review_system(&self) -> anyhow::Result<String> {
        Ok(self.review_system.clone())
    }

    pub fn render_finalize_system(&self) -> anyhow::Result<String> {
        Ok(self.finalize_system.clone())
    }

    fn render_impl(&self, key: &str, ctx: &impl Serialize) -> anyhow::Result<String> {
        let template = self.templates.get_template(key)?;
        Ok(template.render(ctx)?)
    }

    pub fn render_triage_user(&self, prompt: Option<String>) -> anyhow::Result<String> {
        self.render_impl(
            TRIAGE_USER_KEY,
            &serde_json::json!({
                "instructions": self.instructions,
                "prompt": prompt,
            }),
        )
    }

    pub fn render_review_user(&self, ctx: &ReviewUnit) -> anyhow::Result<String> {
        self.render_impl(
            REVIEW_USER_KEY,
            &serde_json::json!({
                "instructions": self.instructions,
                "unit": ctx,
            }),
        )
    }

    pub fn render_finalize_user(&self, ctx: &[R]) -> anyhow::Result<String> {
        self.render_impl(
            FINALIZE_USER_KEY,
            &serde_json::json!({
                "instructions": self.instructions,
                "reviews": ctx,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::submit_marker::ReviewModel;
    use crate::orchestrator::{
        ReviewCategory, ReviewFinding, ReviewSeverity, SecurityCategory, SecurityFinding,
        SecurityRisk, SecuritySeverity, SubmitReviewArgs, SubmitSecurityReviewArgs,
    };

    fn default_prompt_manager() -> PromptManager<SubmitReviewArgs> {
        PromptManager::new(
            PromptProfile::Normal,
            PromptOverrides {
                instructions: Some("Prefer correctness issues over style comments.".to_string()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn default_security_prompt_manager() -> PromptManager<SubmitSecurityReviewArgs> {
        PromptManager::new(
            PromptProfile::Security,
            PromptOverrides {
                instructions: Some("Focus on externally reachable attack paths.".to_string()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn renders_default_system_prompts() {
        let prompts = default_prompt_manager();

        let triage = prompts.render_triage_system().unwrap();
        let review = prompts.render_review_system().unwrap();
        let finalize = prompts.render_finalize_system().unwrap();

        assert!(triage.contains("submit_triage"));
        assert!(review.contains("submit_review"));
        assert!(finalize.contains("submit_review_result"));
    }

    #[test]
    fn renders_default_triage_user_template_with_prompt() {
        let prompts = default_prompt_manager();

        let rendered = prompts
            .render_triage_user(Some("Focus on error handling.".to_string()))
            .unwrap();

        assert!(rendered.contains("Prefer correctness issues over style comments."));
        assert!(rendered.contains("Focus on error handling."));
        assert!(rendered.contains("submit_triage"));
    }

    #[test]
    fn renders_default_review_user_template_with_unit() {
        let prompts = default_prompt_manager();
        let unit = ReviewUnit {
            task: "Review CLI output handling".to_string(),
            focus_files: vec!["src/main.rs".to_string(), "src/config.rs".to_string()],
            model: ReviewModel::Standard,
        };

        let rendered = prompts.render_review_user(&unit).unwrap();

        assert!(rendered.contains("Review CLI output handling"));
        assert!(rendered.contains("src/main.rs"));
        assert!(rendered.contains("src/config.rs"));
        assert!(rendered.contains("standard"));
        assert!(rendered.contains("submit_review"));
    }

    #[test]
    fn renders_default_finalize_user_template_with_reviews() {
        let prompts = default_prompt_manager();
        let reviews = vec![SubmitReviewArgs {
            summary: "Found one issue.".to_string(),
            findings: vec![ReviewFinding {
                severity: ReviewSeverity::High,
                category: ReviewCategory::Bug,
                path: Some("src/main.rs".to_string()),
                line: Some(42),
                title: "Fallback hides write failures".to_string(),
                comment: "The code can silently discard the intended output file.".to_string(),
                recommendation: Some(
                    "Return the write error unless fallback is enabled.".to_string(),
                ),
            }],
            unanswered_questions: vec!["Whether generated files are in scope.".to_string()],
            confidence: 0.85,
        }];

        let rendered = prompts.render_finalize_user(&reviews).unwrap();

        assert!(rendered.contains("Found one issue."));
        assert!(rendered.contains("high"));
        assert!(rendered.contains("bug"));
        assert!(rendered.contains("src/main.rs:42"));
        assert!(rendered.contains("Fallback hides write failures"));
        assert!(rendered.contains("Whether generated files are in scope."));
        assert!(rendered.contains("submit_review_result"));
    }

    #[test]
    fn renders_default_security_system_prompts() {
        let prompts = default_security_prompt_manager();

        let triage = prompts.render_triage_system().unwrap();
        let review = prompts.render_review_system().unwrap();
        let finalize = prompts.render_finalize_system().unwrap();

        assert!(triage.contains("security review"));
        assert!(triage.contains("submit_triage"));
        assert!(review.contains("submit_review"));
        assert!(finalize.contains("submit_review_result"));
    }

    #[test]
    fn renders_default_security_finalize_user_template_with_security_reviews() {
        let prompts = default_security_prompt_manager();
        let reviews = vec![SubmitSecurityReviewArgs {
            summary: "Found one externally reachable flaw.".to_string(),
            overall_risk: SecurityRisk::High,
            findings: vec![SecurityFinding {
                severity: SecuritySeverity::High,
                category: SecurityCategory::Authorization,
                path: Some("src/api.rs".to_string()),
                line: Some(77),
                title: "Tenant boundary can be bypassed".to_string(),
                evidence: "The handler trusts a tenant_id request parameter.".to_string(),
                attack_scenario: Some(
                    "An authenticated user can request another tenant's records.".to_string(),
                ),
                impact: "Cross-tenant data exposure.".to_string(),
                recommendation: Some(
                    "Derive the tenant from authenticated session state.".to_string(),
                ),
                cwe: Some("CWE-639".to_string()),
                owasp: Some("A01:2021-Broken Access Control".to_string()),
                references: vec!["https://cwe.mitre.org/data/definitions/639.html".to_string()],
            }],
            assumptions: vec!["The endpoint is reachable by tenant users.".to_string()],
            unanswered_questions: vec![
                "Whether upstream middleware enforces tenant scope.".to_string(),
            ],
            confidence: 0.8,
        }];

        let rendered = prompts.render_finalize_user(&reviews).unwrap();

        assert!(rendered.contains("Found one externally reachable flaw."));
        assert!(rendered.contains("high"));
        assert!(rendered.contains("authorization"));
        assert!(rendered.contains("src/api.rs:77"));
        assert!(rendered.contains("Tenant boundary can be bypassed"));
        assert!(rendered.contains("The handler trusts a tenant_id request parameter."));
        assert!(rendered.contains("Cross-tenant data exposure."));
        assert!(rendered.contains("CWE-639"));
        assert!(rendered.contains("A01:2021-Broken Access Control"));
        assert!(rendered.contains("The endpoint is reachable by tenant users."));
        assert!(rendered.contains("submit_review_result"));
    }
}
