use crate::orchestrator::submit_marker::{ReviewUnit, SubmitReviewArgs};
use minijinja::Environment;
use schemars::_private::serde_json;
use serde::Serialize;

pub(crate) struct PromptManager {
    instructions: String,
    templates: Environment<'static>,
    triage_system: String,
    review_system: String,
    finalize_system: String,
}

const TRIAGE_USER_KEY: &str = "triage_user";
const REVIEW_USER_KEY: &str = "review_user";
const FINALIZE_USER_KEY: &str = "finalize_user";

impl PromptManager {
    pub fn new(
        instructions: String,
        triage_system: String,
        triage_user: String,
        review_system: String,
        review_user: String,
        finalize_system: String,
        finalize_user: String,
    ) -> anyhow::Result<Self> {
        let mut templates = Environment::new();

        templates.add_template_owned(TRIAGE_USER_KEY, triage_user)?;
        templates.add_template_owned(REVIEW_USER_KEY, review_user)?;
        templates.add_template_owned(FINALIZE_USER_KEY, finalize_user)?;

        Ok(Self {
            instructions,
            templates,
            triage_system,
            review_system,
            finalize_system,
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

    pub fn render_triage_user(&self, prompt: &str) -> anyhow::Result<String> {
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

    pub fn render_finalize_user(&self, ctx: &[SubmitReviewArgs]) -> anyhow::Result<String> {
        self.render_impl(
            FINALIZE_USER_KEY,
            &serde_json::json!({
                "instructions": self.instructions,
                "reviews": ctx,
            }),
        )
    }
}
