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
use crate::git::helper::remotes::GitRemote;
use serde_json::json;

pub struct GitHub;

impl Default for GitHub {
    fn default() -> Self {
        Self
    }
}

fn parse_owner_and_repo(remote: &str) -> Option<(String, String)> {
    if remote.starts_with("https://github.com/") {
        let remote = remote.trim_start_matches("https://github.com/");
        let (owner, repo) = remote.split_once('/')?;
        let repo = repo.trim_end_matches(".git");
        return Some((owner.to_string(), repo.to_string()));
    }
    if remote.starts_with("git@github.com:") {
        let remote = remote.trim_start_matches("git@github.com:");
        let (owner, repo) = remote.split_once('/')?;
        let repo = repo.trim_end_matches(".git");
        return Some((owner.to_string(), repo.to_string()));
    }
    None
}

fn get_remote() -> anyhow::Result<(String, String)> {
    for remote in Git.get_remotes()? {
        if let Some((owner, repo)) = parse_owner_and_repo(&remote) {
            return Ok((owner, repo));
        }
    }
    anyhow::bail!("Failed to find github remote")
}

#[derive(serde::Deserialize)]
struct GetRepoResponse {
    default_branch: String,
}

#[derive(serde::Deserialize)]
struct GetPullRequestResponse {
    base: GetPullRequestResponseBase,
}

#[derive(serde::Deserialize)]
struct GetPullRequestResponseBase {
    #[serde(rename = "ref")]
    ref_name: String,
}

#[async_trait::async_trait]
impl GitRemote for GitHub {
    async fn get_default_branch(&self) -> anyhow::Result<String> {
        let (owner, repo) = get_remote()?;
        let res: GetRepoResponse = octocrab::instance()
            .get(
                format!("/repos/{owner}/{repo}"),
                Option::<&serde_json::Value>::None,
            )
            .await?;
        Ok(res.default_branch)
    }

    async fn get_pull_request_base_branch(&self) -> anyhow::Result<String> {
        let current_branch = Git.current_branch()?;
        let (owner, repo) = get_remote()?;

        let res: Vec<GetPullRequestResponse> = octocrab::instance()
            .get(
                format!("/repos/{owner}/{repo}/pulls",),
                Some(&json!({"head": format!("{}:{}", owner, current_branch)})),
            )
            .await?;
        let pr = res
            .first()
            .ok_or_else(|| anyhow::anyhow!("No pull request found for current branch"))?;
        Ok(pr.base.ref_name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_owner_and_repo() {
        let (owner, repo) =
            parse_owner_and_repo("https://github.com/https/normal.git").expect("failed to parse");
        assert_eq!(owner, "https");
        assert_eq!(repo, "normal");

        let (owner, repo) =
            parse_owner_and_repo("https://github.com/https/no-suffix").expect("failed to parse");
        assert_eq!(owner, "https");
        assert_eq!(repo, "no-suffix");

        let (owner, repo) =
            parse_owner_and_repo("git@github.com:ssh/normal.git").expect("failed to parse");
        assert_eq!(owner, "ssh");
        assert_eq!(repo, "normal");

        let (owner, repo) =
            parse_owner_and_repo("git@github.com:ssh/no-suffix").expect("failed to parse");
        assert_eq!(owner, "ssh");
        assert_eq!(repo, "no-suffix");
    }
}
