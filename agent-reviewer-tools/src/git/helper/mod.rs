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

use git2::{Diff, DiffOptions, Repository};
use remotes::GitRemote;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

mod remotes;
pub use remotes::github::GitHub;

#[derive(Debug, Serialize)]
pub struct GitDiffResult {
    pub files: Vec<FileDiff>,
    pub summary: DiffSummary,
}

#[derive(Debug, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Serialize)]
pub struct Hunk {
    pub header: String, // @@ -1,7 +1,7 @@ など
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
pub struct DiffLine {
    pub kind: LineKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub enum LineKind {
    Added,
    Deleted,
    Context,
}

#[derive(Debug, Serialize)]
pub struct DiffSummary {
    pub total_files: usize,
    pub total_additions: usize,
    pub total_deletions: usize,
}

#[derive(Debug, Serialize)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
}

pub(super) struct Git;

impl Git {
    fn open_repo(&self) -> anyhow::Result<Repository> {
        let repo = Repository::open(".")?;
        Ok(repo)
    }

    fn get_remote_urls(&self) -> anyhow::Result<Vec<String>> {
        let repo = self.open_repo()?;
        Self::remote_urls(&repo)
    }

    fn remote_urls(repo: &Repository) -> anyhow::Result<Vec<String>> {
        let remotes = repo.remotes()?;
        let mut urls = Vec::with_capacity(remotes.len());
        for remote in remotes.into_iter().flatten() {
            if let Some(remote) = remote
                && let Ok(remote) = repo.find_remote(remote)
            {
                urls.push(remote.url()?.to_string());
            }
        }
        Ok(urls)
    }

    pub fn current_branch(&self) -> anyhow::Result<String> {
        let repo = self.open_repo()?;
        Self::current_branch_name(&repo)
    }

    fn current_branch_name(repo: &Repository) -> anyhow::Result<String> {
        let head = repo.head()?;
        let branch_name = head.shorthand()?;
        Ok(branch_name.to_string())
    }

    pub async fn default_branch(&self) -> anyhow::Result<String> {
        GitHub.get_default_branch().await
    }

    pub async fn get_pr_default_branch(&self) -> anyhow::Result<String> {
        GitHub.get_pull_request_base_branch().await
    }

    pub fn diff_single_commit(
        &self,
        commit_id: String,
        files: Option<Vec<String>>,
        summary: bool,
    ) -> anyhow::Result<GitDiffResult> {
        let repo = self.open_repo()?;
        let oid = repo.revparse_single(&commit_id)?.id();
        let commit = repo.find_commit(oid)?;
        let head = commit.tree()?;
        let base = commit.parent(0).ok().map(|p| p.tree()).transpose()?;

        let diff = repo.diff_tree_to_tree(
            base.as_ref(),
            Some(&head),
            Some(&mut DiffOptions::default()),
        )?;

        Self::diff_to_result(&diff, files, summary)
    }

    pub fn diff_commit_range(
        &self,
        from: String,
        to: String,
        files: Option<Vec<String>>,
        summary: bool,
    ) -> anyhow::Result<GitDiffResult> {
        let repo = self.open_repo()?;
        let from = repo.revparse_single(&from)?.peel_to_tree()?;
        let to = repo.revparse_single(&to)?.peel_to_tree()?;
        let diff =
            repo.diff_tree_to_tree(Some(&from), Some(&to), Some(&mut DiffOptions::default()))?;
        Self::diff_to_result(&diff, files, summary)
    }

    fn diff_to_result(
        diff: &Diff,
        files: Option<Vec<String>>,
        summary: bool,
    ) -> anyhow::Result<GitDiffResult> {
        let files_to_get = files
            .unwrap_or_default()
            .into_iter()
            .map(normalize_repo_relative_path)
            .collect::<Vec<PathBuf>>();

        let diff_result = Self::parse_diff(diff, &files_to_get, summary)?;

        Ok(diff_result)
    }

    fn parse_diff(
        diff: &Diff,
        files_to_get: &[PathBuf],
        summary: bool,
    ) -> Result<GitDiffResult, git2::Error> {
        let mut files: Vec<FileDiff> = Vec::new();
        let mut skip_current_file = false;

        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            match line.origin() {
                'F' => {
                    let path = delta
                        .new_file()
                        .path()
                        .or_else(|| delta.old_file().path())
                        .map(|p| p.to_path_buf())
                        .unwrap_or_default();

                    skip_current_file =
                        !files_to_get.is_empty() && !files_to_get.iter().any(|p| p == &path);

                    if !skip_current_file {
                        let status = match delta.status() {
                            git2::Delta::Added => FileStatus::Added,
                            git2::Delta::Deleted => FileStatus::Deleted,
                            git2::Delta::Modified => FileStatus::Modified,
                            git2::Delta::Renamed => FileStatus::Renamed,
                            _ => FileStatus::Modified,
                        };
                        files.push(FileDiff {
                            path: path.to_string_lossy().into_owned(),
                            status,
                            additions: 0,
                            deletions: 0,
                            hunks: vec![],
                        });
                    }
                }

                'H' => {
                    if !skip_current_file && !summary {
                        let header = hunk
                            .map(|h| {
                                std::str::from_utf8(h.header())
                                    .unwrap_or("")
                                    .trim()
                                    .to_string()
                            })
                            .unwrap_or_default();

                        if let Some(file) = files.last_mut() {
                            file.hunks.push(Hunk {
                                header,
                                lines: vec![],
                            });
                        }
                    }
                }

                origin => {
                    if !skip_current_file {
                        let kind = match origin {
                            '+' => LineKind::Added,
                            '-' => LineKind::Deleted,
                            _ => LineKind::Context,
                        };

                        // summaryの場合はadditions/deletionsのカウントだけ行い、contentは入れない
                        if let Some(file) = files.last_mut() {
                            match kind {
                                LineKind::Added => file.additions += 1,
                                LineKind::Deleted => file.deletions += 1,
                                LineKind::Context => {}
                            }

                            let content = if summary {
                                None
                            } else {
                                Some(
                                    std::str::from_utf8(line.content())
                                        .unwrap_or("")
                                        .trim_end_matches('\n')
                                        .to_string(),
                                )
                            };

                            if let Some(hunk) = file.hunks.last_mut() {
                                hunk.lines.push(DiffLine { kind, content });
                            }
                        }
                    }
                }
            }

            true
        })?;

        let summary_result = DiffSummary {
            total_files: files.len(),
            total_additions: files.iter().map(|f| f.additions).sum(),
            total_deletions: files.iter().map(|f| f.deletions).sum(),
        };

        Ok(GitDiffResult {
            files,
            summary: summary_result,
        })
    }
}

fn normalize_repo_relative_path(path: String) -> PathBuf {
    Path::new(&path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{RepositoryInitOptions, Signature};
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempRepoDir {
        path: PathBuf,
    }

    impl TempRepoDir {
        fn new(test_name: &str) -> anyhow::Result<Self> {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let path = std::env::temp_dir().join(format!(
                "agent-reviewer-tools-{test_name}-{}-{timestamp}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path)?;

            Ok(Self { path })
        }
    }

    impl Drop for TempRepoDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn init_repo(initial_branch: &str) -> anyhow::Result<(TempRepoDir, Repository)> {
        let dir = TempRepoDir::new("git-helper")?;
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head(initial_branch);
        let repo = Repository::init_opts(&dir.path, &opts)?;

        Ok((dir, repo))
    }

    fn commit_file(repo: &Repository, path: &str, contents: &str) -> anyhow::Result<git2::Oid> {
        let workdir = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("test repository must have a workdir"))?;
        fs::write(workdir.join(path), contents)?;

        let mut index = repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = Signature::now("Agent Reviewer", "agent-reviewer@example.com")?;
        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "test commit",
            &tree,
            &[],
        )?;

        Ok(oid)
    }

    #[test]
    fn current_branch_name_returns_initial_branch() -> anyhow::Result<()> {
        let (_dir, repo) = init_repo("main")?;
        commit_file(&repo, "README.md", "test")?;

        assert_eq!(Git::current_branch_name(&repo)?, "main");

        Ok(())
    }

    #[test]
    fn current_branch_name_returns_checked_out_branch() -> anyhow::Result<()> {
        let (_dir, repo) = init_repo("main")?;
        let commit_id = commit_file(&repo, "README.md", "test")?;
        let commit = repo.find_commit(commit_id)?;
        repo.branch("feature/test", &commit, false)?;
        repo.set_head("refs/heads/feature/test")?;

        assert_eq!(Git::current_branch_name(&repo)?, "feature/test");

        Ok(())
    }

    #[test]
    fn remote_urls_returns_configured_urls_not_remote_names() -> anyhow::Result<()> {
        let (_dir, repo) = init_repo("main")?;
        repo.remote("origin", "git@github.com:owner/repo.git")?;
        repo.remote("backup", "https://example.com/owner/repo.git")?;

        let mut urls = Git::remote_urls(&repo)?;
        urls.sort();
        let mut expected = vec![
            "git@github.com:owner/repo.git".to_string(),
            "https://example.com/owner/repo.git".to_string(),
        ];
        expected.sort();

        assert_eq!(urls, expected);

        Ok(())
    }
}
