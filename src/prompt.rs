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

const DEFAULT_TRIAGE_SYSTEM: &str = include_str!("default_prompts/triage/system.md");
const DEFAULT_TRIAGE_USER: &str = include_str!("default_prompts/triage/user.md.jinja2");
const DEFAULT_REVIEW_SYSTEM: &str = include_str!("default_prompts/review/system.md");
const DEFAULT_REVIEW_USER: &str = include_str!("default_prompts/review/user.md.jinja2");
const DEFAULT_FINALIZE_SYSTEM: &str = include_str!("default_prompts/finalize/system.md");
const DEFAULT_FINALIZE_USER: &str = include_str!("default_prompts/finalize/user.md.jinja2");

impl<R> PromptManager<R>
where
    R: Serialize,
{
    pub fn new(
        instructions: Option<String>,
        triage_system: Option<String>,
        triage_user: Option<String>,
        review_system: Option<String>,
        review_user: Option<String>,
        finalize_system: Option<String>,
        finalize_user: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut templates = Environment::new();

        templates.add_template_owned(
            TRIAGE_USER_KEY,
            triage_user.unwrap_or_else(|| DEFAULT_TRIAGE_USER.to_string()),
        )?;
        templates.add_template_owned(
            REVIEW_USER_KEY,
            review_user.unwrap_or_else(|| DEFAULT_REVIEW_USER.to_string()),
        )?;
        templates.add_template_owned(
            FINALIZE_USER_KEY,
            finalize_user.unwrap_or_else(|| DEFAULT_FINALIZE_USER.to_string()),
        )?;

        Ok(Self {
            instructions,
            templates,
            triage_system: triage_system.unwrap_or_else(|| DEFAULT_TRIAGE_SYSTEM.to_string()),
            review_system: review_system.unwrap_or_else(|| DEFAULT_REVIEW_SYSTEM.to_string()),
            finalize_system: finalize_system.unwrap_or_else(|| DEFAULT_FINALIZE_SYSTEM.to_string()),
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
    use crate::orchestrator::{ReviewCategory, ReviewFinding, ReviewSeverity, SubmitReviewArgs};

    fn default_prompt_manager() -> PromptManager<SubmitReviewArgs> {
        PromptManager::new(
            Some("Prefer correctness issues over style comments.".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
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
}
