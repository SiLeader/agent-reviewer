use crate::AgentTool;
use genai::chat::Tool;
use serde_json::Value;

pub struct GitDiff;

#[async_trait::async_trait]
impl AgentTool for GitDiff {
    fn tool(&self) -> Tool {
        todo!()
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        todo!()
    }
}
