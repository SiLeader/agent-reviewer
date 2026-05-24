use crate::git::helper::Git;
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct GitCurrentBranch;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitCurrentBranchArgs {}

#[derive(Debug, Serialize)]
struct GitCurrentBranchResult {
    branch_name: String,
}

#[async_trait::async_trait]
impl AgentTool for GitCurrentBranch {
    fn tool(&self) -> Tool {
        tool_description::<GitCurrentBranchArgs>("git_current_branch", "Get the current git branch")
    }

    async fn run(&self, _args: &Value) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(|| {
            let name = Git::new()?.current_branch()?;
            Ok(serde_json::to_string(&GitCurrentBranchResult {
                branch_name: name,
            })?)
        })
        .await?
    }
}
