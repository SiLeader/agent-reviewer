use crate::git::helper::Git;
use crate::{AgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub struct GitDiffSingleCommit;
pub struct GitDiffSummarySingleCommit;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitDiffSingleCommitArgs {
    #[schemars(
        required,
        description = "The files to diff. If not provided, diffs all files."
    )]
    files: Option<Vec<String>>,

    #[schemars(required, description = "The commit ID to diff against.")]
    commit_id: String,
}

#[async_trait::async_trait]
impl AgentTool for GitDiffSingleCommit {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffSingleCommitArgs>(
            "git_diff",
            "Returns the diff of a single commit.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffSingleCommitArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let git = Git::new()?;
            let diff = git.diff_single_commit(args.commit_id, false)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}

#[async_trait::async_trait]
impl AgentTool for GitDiffSummarySingleCommit {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffSingleCommitArgs>(
            "git_diff_summary",
            "Returns the diff summary of a single commit.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffSingleCommitArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let git = Git::new()?;
            let diff = git.diff_single_commit(args.commit_id, true)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}
