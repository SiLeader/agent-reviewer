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
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;

pub struct GitHub;

struct GitHubHolder {
    base_url: String,
    token: String,
}

static STATIC_INSTANCE: LazyLock<RwLock<GitHubHolder>> = LazyLock::new(|| {
    RwLock::new(GitHubHolder {
        base_url: "https://api.github.com".to_string(),
        token: "".to_string(),
    })
});

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
    get_remotes()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to find github remote"))
}

fn get_remotes() -> anyhow::Result<Vec<(String, String)>> {
    let mut remotes = Vec::new();
    for remote in Git.get_remote_urls()? {
        if let Some((owner, repo)) = parse_owner_and_repo(&remote) {
            remotes.push((owner, repo));
        }
    }
    Ok(remotes)
}

#[derive(serde::Deserialize)]
struct GetRepoResponse {
    default_branch: String,
    fork: bool,
    parent: Option<GetRepoResponseParent>,
}

#[derive(serde::Deserialize)]
struct GetRepoResponseParent {
    full_name: String,
}

#[derive(serde::Deserialize)]
struct GetPullRequestResponse {
    base: GetPullRequestResponseBase,
    head: GetPullRequestResponseHead,
}

#[derive(serde::Deserialize)]
struct GetPullRequestResponseBase {
    #[serde(rename = "ref")]
    ref_name: String,
}

#[derive(serde::Deserialize)]
struct GetPullRequestResponseHead {
    #[serde(rename = "ref")]
    ref_name: String,
}

fn parse_full_name(full_name: &str) -> Option<(String, String)> {
    let (owner, repo) = full_name.split_once('/')?;
    Some((owner.to_string(), repo.to_string()))
}

fn pull_request_search_repositories(
    head_owner: &str,
    head_repo: &str,
    repo: &GetRepoResponse,
) -> Vec<(String, String)> {
    let mut repositories = Vec::new();

    if repo.fork
        && let Some(parent) = &repo.parent
        && let Some((base_owner, base_repo)) = parse_full_name(&parent.full_name)
    {
        repositories.push((base_owner, base_repo));
    }

    let head_repository = (head_owner.to_string(), head_repo.to_string());
    if !repositories.contains(&head_repository) {
        repositories.push(head_repository);
    }

    repositories
}

impl GitHub {
    pub async fn set_token(&self, token: &str) {
        STATIC_INSTANCE.write().await.token = token.trim_end_matches("/").to_string();
    }

    async fn request_get<Res: DeserializeOwned>(
        &self,
        path: &str,
        query: Option<BTreeMap<&str, String>>,
    ) -> anyhow::Result<Res> {
        let holder = STATIC_INSTANCE.read().await;
        if holder.token.is_empty() {
            anyhow::bail!("Failed to request to GitHub: token is not set");
        }

        let query = if let Some(query) = query {
            format!(
                "?{}",
                query
                    .into_iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(&v)))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        } else {
            "".to_string()
        };

        let client = reqwest::Client::new();
        let res = client
            .get(format!(
                "{}/{}{query}",
                holder.base_url,
                path.trim_start_matches('/')
            ))
            .bearer_auth(&holder.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .header("User-Agent", "agent-reviewer")
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await?;
            anyhow::bail!("Failed to request github: {} {}", status, body);
        }
        Ok(res.json().await?)
    }

    async fn find_pull_request_for_branch(
        &self,
        current_branch: &str,
    ) -> anyhow::Result<GetPullRequestResponse> {
        let mut base_repositories = Vec::new();

        for (head_owner, head_repo) in get_remotes()? {
            let repo: GetRepoResponse = self
                .request_get(&format!("/repos/{head_owner}/{head_repo}"), None)
                .await?;
            for (base_owner, base_repo) in
                pull_request_search_repositories(&head_owner, &head_repo, &repo)
            {
                let base_repository = (base_owner.clone(), base_repo.clone());
                if !base_repositories.contains(&base_repository) {
                    base_repositories.push(base_repository);
                }

                let res: Vec<GetPullRequestResponse> = self
                    .request_get(
                        &format!("/repos/{base_owner}/{base_repo}/pulls"),
                        Some(BTreeMap::from([
                            ("head", format!("{}:{}", head_owner, current_branch)),
                            ("state", "open".to_string()),
                        ])),
                    )
                    .await?;

                if let Some(pr) = res.into_iter().next() {
                    return Ok(pr);
                }
            }
        }

        let mut branch_matches = Vec::new();
        for (base_owner, base_repo) in base_repositories {
            for page in 1..=10 {
                let res: Vec<GetPullRequestResponse> = self
                    .request_get(
                        &format!("/repos/{base_owner}/{base_repo}/pulls"),
                        Some(BTreeMap::from([
                            ("state", "open".to_string()),
                            ("per_page", "100".to_string()),
                            ("page", page.to_string()),
                        ])),
                    )
                    .await?;
                let has_next_page = res.len() == 100;
                branch_matches.extend(
                    res.into_iter()
                        .filter(|pr| pr.head.ref_name == current_branch),
                );
                if !has_next_page {
                    break;
                }
            }
        }

        if branch_matches.len() == 1 {
            return Ok(branch_matches.remove(0));
        }
        if branch_matches.len() > 1 {
            anyhow::bail!("Multiple pull requests found for current branch");
        }

        anyhow::bail!("No pull request found for current branch")
    }
}

#[async_trait::async_trait]
impl GitRemote for GitHub {
    async fn get_default_branch(&self) -> anyhow::Result<String> {
        let (owner, repo) = get_remote()?;
        let res: GetRepoResponse = self
            .request_get(&format!("/repos/{owner}/{repo}"), None)
            .await?;
        Ok(res.default_branch)
    }

    async fn get_pull_request_base_branch(&self) -> anyhow::Result<String> {
        let current_branch = Git.current_branch()?;
        let pr = self.find_pull_request_for_branch(&current_branch).await?;
        Ok(pr.base.ref_name)
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

    #[test]
    fn pull_request_search_repositories_prefers_fork_parent() {
        let repo = GetRepoResponse {
            default_branch: "main".to_string(),
            fork: true,
            parent: Some(GetRepoResponseParent {
                full_name: "upstream/project".to_string(),
            }),
        };

        assert_eq!(
            pull_request_search_repositories("fork-owner", "project", &repo),
            vec![
                ("upstream".to_string(), "project".to_string()),
                ("fork-owner".to_string(), "project".to_string()),
            ]
        );
    }

    #[test]
    fn pull_request_search_repositories_uses_head_repository_for_non_fork() {
        let repo = GetRepoResponse {
            default_branch: "main".to_string(),
            fork: false,
            parent: None,
        };

        assert_eq!(
            pull_request_search_repositories("owner", "project", &repo),
            vec![("owner".to_string(), "project".to_string())]
        );
    }
}
