use crate::git::helper::{Git, GitDiffRange};
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub struct GitDiff;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitDiffArgs {
    files: Option<Vec<String>>,
    #[serde(flatten)]
    range: GitDiffRange,
}

#[async_trait::async_trait]
impl AgentTool for GitDiff {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffArgs>(
            "git_diff",
            "Returns the diff of a files with a range of commits.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let git = Git::new()?;
            let diff = git.diff(args.range, args.files, false)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}
