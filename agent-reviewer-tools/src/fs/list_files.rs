use crate::AgentTool;
use genai::chat::Tool;
use serde_json::Value;

pub struct ListFiles;

#[async_trait::async_trait]
impl AgentTool for ListFiles {
    fn tool(&self) -> Tool {
        todo!()
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        todo!()
    }
}
