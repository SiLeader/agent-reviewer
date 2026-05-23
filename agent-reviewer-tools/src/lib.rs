pub mod fs;
pub mod git;
mod multi;

use genai::chat::{Tool, ToolName};
pub use multi::*;
use schemars::JsonSchema;

pub fn tool_description<T: JsonSchema>(name: &'static str, description: &'static str) -> Tool {
    Tool {
        name: ToolName::Custom(name.to_string()),
        description: Some(description.to_string()),
        schema: Some({
            let mut settings = schemars::generate::SchemaSettings::openapi3();
            settings.inline_subschemas = true;
            let generator = schemars::generate::SchemaGenerator::new(settings);
            generator.into_root_schema_for::<T>().to_value()
        }),
        strict: Some(true),
        config: None,
    }
}

pub trait MarkerAgentTool {
    fn tool(&self) -> Tool;
}

#[async_trait::async_trait]
pub trait AgentTool: Send + Sync {
    fn tool(&self) -> Tool;

    async fn run(&self, args: &serde_json::Value) -> anyhow::Result<String>;
}
