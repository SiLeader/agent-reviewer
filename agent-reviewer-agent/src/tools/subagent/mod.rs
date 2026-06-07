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

mod advisor;
mod explorer;
mod verifier;

use crate::ReActAgent;
pub use advisor::*;
use agent_reviewer_tools::MarkerAgentTool;
use agent_reviewer_tools::fs::{ListFiles, ReadFile, SearchFile};
use agent_reviewer_tools::git::{
    GitCurrentBranch, GitDefaultBranch, GitDiffCommitRange, GitDiffSingleCommit,
    GitDiffSummaryCommitRange, GitDiffSummarySingleCommit, GitPrBaseBranch,
};
pub use explorer::*;
use serde_json::Value;
use std::sync::Arc;
pub use verifier::*;

fn setup_tools(agent: &mut ReActAgent, submit: impl MarkerAgentTool + 'static) {
    agent.submit_tool_name = submit.tool().name.to_string();
    agent.tools.add_tool(Arc::new(ReadFile));
    agent.tools.add_tool(Arc::new(ListFiles));
    agent.tools.add_tool(Arc::new(SearchFile));
    agent.tools.add_tool(Arc::new(GitDiffSingleCommit));
    agent.tools.add_tool(Arc::new(GitDiffCommitRange));
    agent.tools.add_tool(Arc::new(GitDiffSummarySingleCommit));
    agent.tools.add_tool(Arc::new(GitDiffSummaryCommitRange));
    agent.tools.add_tool(Arc::new(GitPrBaseBranch));
    agent.tools.add_tool(Arc::new(GitDefaultBranch));
    agent.tools.add_tool(Arc::new(GitCurrentBranch));
    agent.tools.add_marker(Arc::new(submit));
}

async fn run_subagent(
    agent: &ReActAgent,
    system_prompt: &'static str,
    args: &Value,
) -> anyhow::Result<String> {
    let value = agent.run(system_prompt, &args.to_string()).await?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn verifier_module_is_exported() {
        // This test just verifies the module is accessible.
        let _desc = super::VERIFIER_TOOL_DESCRIPTION;
        assert!(!_desc.is_empty());
    }
}
