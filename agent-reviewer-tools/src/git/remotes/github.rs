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

use crate::git::remotes::GitRemote;

pub struct GitHub;

impl Default for GitHub {
    fn default() -> Self {
        Self
    }
}

impl GitRemote for GitHub {
    fn get_default_branch(&self) -> anyhow::Result<String> {
        let res = std::process::Command::new("gh")
            .args([
                "repo",
                "view",
                "--json",
                "defaultBranchRef",
                "--jq",
                ".defaultBranchRef.name",
            ])
            .output()?;
        if !res.status.success() {
            anyhow::bail!(
                "Failed to get default branch: {}",
                String::from_utf8_lossy(&res.stderr)
            );
        }
        let branch_name = String::from_utf8_lossy(&res.stdout);
        Ok(branch_name.trim().to_string())
    }

    fn get_pull_request_base_branch(&self) -> anyhow::Result<String> {
        let res = std::process::Command::new("gh")
            .args([
                "pr",
                "view",
                "--json",
                "baseRefName",
                "--jq",
                ".baseRefName",
            ])
            .output()?;
        if !res.status.success() {
            anyhow::bail!(
                "Failed to get base branch of current pull request: {}",
                String::from_utf8_lossy(&res.stderr)
            );
        }
        let branch_name = String::from_utf8_lossy(&res.stdout);
        Ok(branch_name.trim().to_string())
    }
}
