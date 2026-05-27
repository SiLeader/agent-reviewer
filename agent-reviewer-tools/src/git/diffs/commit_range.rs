// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
            "git_diff_for_commit_range",
            "Returns the diff of a files with a range of commits.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffCommitRangeArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let diff = Git.diff_commit_range(args.from, args.to, args.files, false)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}

#[async_trait::async_trait]
impl AgentTool for GitDiffSummaryCommitRange {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffCommitRangeArgs>(
            "git_diff_summary_for_commit_range",
            "Returns the diff summary of a files with a range of commits.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffCommitRangeArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let diff_summary = Git.diff_commit_range(args.from, args.to, args.files, true)?;

            Ok(serde_json::to_string(&diff_summary)?)
        })
        .await?
    }
}
