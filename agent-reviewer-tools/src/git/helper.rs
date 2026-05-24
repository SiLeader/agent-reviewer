use crate::git::remotes::GitRemote;
use crate::git::remotes::github::GitHub;
use git2::{Diff, DiffOptions, Repository, Tree};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct GitDiffRange {
    #[serde(rename = "type")]
    #[schemars(description = "The type of the diff range.")]
    pub diff_type: GitDiffType,
    #[schemars(
        description = "The branch or commit range to diff. If type is 'branch', this is the branch name. If type is 'commit_range', this is the format 'from..to'. If type is 'single_commit', this field is ignored."
    )]
    pub from: Option<String>,
    pub to: Option<String>,
    pub commit_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum GitDiffType {
    #[schemars(description = "Get diff between a single commit and its parent.")]
    SingleCommit,
    #[schemars(description = "Get diff between two commits.")]
    CommitRange,
    #[schemars(description = "Get diff in a branch.")]
    Branch,
    #[schemars(description = "Get diff in a pull request for current branch.")]
    PullRequest,
}

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

pub(super) struct Git {
    repo: Repository,
}

impl Git {
    pub fn new() -> anyhow::Result<Self> {
        let repo = Repository::open(".")?;
        Ok(Self { repo })
    }

    fn get_tree_pair(
        &self,
        range: GitDiffRange,
        remote: impl GitRemote,
    ) -> anyhow::Result<(Option<Tree>, Option<Tree>)> {
        match range.diff_type {
            GitDiffType::SingleCommit => {
                let commit_id = range.commit_id.unwrap_or_else(|| "HEAD".to_string());
                let oid = self.repo.revparse_single(&commit_id)?.id();
                let commit = self.repo.find_commit(oid)?;
                let tree = commit.tree()?;
                let parent_tree = commit.parent(0).ok().map(|p| p.tree()).transpose()?;
                Ok((parent_tree, Some(tree)))
            }
            GitDiffType::CommitRange | GitDiffType::Branch => {
                let from = match range.from {
                    None => self
                        .repo
                        .revparse_single(&remote.get_default_branch()?)?
                        .peel_to_tree()?,
                    Some(from) => self.repo.revparse_single(&from)?.peel_to_tree()?,
                };
                let to = match range.to {
                    None => self.repo.head()?.peel_to_commit()?.tree()?,
                    Some(to) => self.repo.revparse_single(&to)?.peel_to_tree()?,
                };
                Ok((Some(from), Some(to)))
            }
            GitDiffType::PullRequest => {
                let from = self
                    .repo
                    .revparse_single(&remote.get_pull_request_base_branch()?)?
                    .peel_to_tree()?;
                let to = self.repo.head()?.peel_to_commit()?.tree()?;
                Ok((Some(from), Some(to)))
            }
        }
    }

    pub fn diff(
        &self,
        range: GitDiffRange,
        files: Option<Vec<String>>,
        summary: bool,
    ) -> anyhow::Result<GitDiffResult> {
        let (base, head) = self.get_tree_pair(range, GitHub::default())?;
        let mut options = DiffOptions::default();

        let diff = self
            .repo
            .diff_tree_to_tree(base.as_ref(), head.as_ref(), Some(&mut options))?;

        let files_to_get = files
            .unwrap_or_else(|| vec![])
            .into_iter()
            .map(|f| PathBuf::from(f))
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect::<Vec<PathBuf>>();

        let diff_result = Self::parse_diff(&diff, &files_to_get, summary)?;

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
