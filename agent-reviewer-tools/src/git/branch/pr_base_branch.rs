use crate::git::helper::Git;
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct GitPrBaseBranch;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitPrBaseBranchArgs {}

#[derive(Debug, Serialize)]
struct GitPrBaseBranchResult {
    branch_name: String,
}

#[async_trait::async_trait]
impl AgentTool for GitPrBaseBranch {
    fn tool(&self) -> Tool {
        tool_description::<GitPrBaseBranchArgs>(
            "git_pull_request_base_branch",
            "Get the base git branch of the pull request",
        )
    }

    async fn run(&self, _args: &Value) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(|| {
            let name = Git::new()?.get_pr_default_branch()?;
            Ok(serde_json::to_string(&GitPrBaseBranchResult {
                branch_name: name,
            })?)
        })
        .await?
    }
}
