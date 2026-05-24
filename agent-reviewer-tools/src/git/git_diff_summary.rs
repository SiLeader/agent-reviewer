use crate::git::helper::{Git, GitDiffRange};
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub struct GitDiffSummary;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitDiffSummaryArgs {
    files: Option<Vec<String>>,
    #[serde(flatten)]
    range: GitDiffRange,
}

#[async_trait::async_trait]
impl AgentTool for GitDiffSummary {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffSummaryArgs>(
            "git_diff_summary",
            "Returns the summary of a diff of a files with a range of commits.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffSummaryArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let git = Git::new()?;
            let diff = git.diff(args.range, args.files, true)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}
