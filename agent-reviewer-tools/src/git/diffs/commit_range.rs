use crate::git::helper::Git;
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub struct GitDiffCommitRange;
pub struct GitDiffSummaryCommitRange;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitDiffCommitRangeArgs {
    #[schemars(
        required,
        description = "The files to diff. If not provided, diffs all files."
    )]
    files: Option<Vec<String>>,

    #[schemars(required, description = "The commit ID or branch name to diff from.")]
    from: String,

    #[schemars(required, description = "The commit ID or branch name to diff to.")]
    to: String,
}

#[async_trait::async_trait]
impl AgentTool for GitDiffCommitRange {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffCommitRangeArgs>(
            "git_diff",
            "Returns the diff of a files with a range of commits.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffCommitRangeArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let git = Git::new()?;
            let diff = git.diff_commit_range(args.from, args.to, args.files, false)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}

#[async_trait::async_trait]
impl AgentTool for GitDiffSummaryCommitRange {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffCommitRangeArgs>(
            "git_diff_summary",
            "Returns the diff summary of a files with a range of commits.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffCommitRangeArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let git = Git::new()?;
            let diff_summary = git.diff_commit_range(args.from, args.to, args.files, true)?;

            Ok(serde_json::to_string(&diff_summary)?)
        })
        .await?
    }
}
