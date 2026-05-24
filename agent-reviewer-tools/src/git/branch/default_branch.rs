use crate::git::helper::Git;
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct GitDefaultBranch;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitDefaultBranchArgs {}

#[derive(Debug, Serialize)]
struct GitDefaultBranchResult {
    branch_name: String,
}

#[async_trait::async_trait]
impl AgentTool for GitDefaultBranch {
    fn tool(&self) -> Tool {
        tool_description::<GitDefaultBranchArgs>("git_default_branch", "Get the default git branch")
    }

    async fn run(&self, _args: &Value) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(|| {
            let name = Git::new()?.default_branch()?;
            Ok(serde_json::to_string(&GitDefaultBranchResult {
                branch_name: name,
            })?)
        })
        .await?
    }
}
