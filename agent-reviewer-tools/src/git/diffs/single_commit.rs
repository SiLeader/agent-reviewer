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
            "git_diff_for_commit",
            "Returns the diff of a single commit.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffSingleCommitArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let diff = Git.diff_single_commit(args.commit_id, args.files, false)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}

#[async_trait::async_trait]
impl AgentTool for GitDiffSummarySingleCommit {
    fn tool(&self) -> Tool {
        tool_description::<GitDiffSingleCommitArgs>(
            "git_diff_summary_for_commit",
            "Returns the diff summary of a single commit.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: GitDiffSingleCommitArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(|| {
            let diff = Git.diff_single_commit(args.commit_id, args.files, true)?;

            Ok(serde_json::to_string(&diff)?)
        })
        .await?
    }
}
