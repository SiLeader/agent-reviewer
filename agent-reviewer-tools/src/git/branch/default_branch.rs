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
