use crate::AgentTool;
use genai::chat::Tool;
use serde_json::Value;

pub struct ReadFile;

#[async_trait::async_trait]
impl AgentTool for ReadFile {
    fn tool(&self) -> Tool {
        todo!()
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        todo!()
    }
}
