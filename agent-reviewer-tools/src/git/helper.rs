use crate::git::remotes::GitRemote;
use crate::git::remotes::github::GitHub;
use git2::{Diff, DiffOptions, Repository};
use serde::Serialize;
use std::path::PathBuf;

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

    pub fn current_branch(&self) -> anyhow::Result<String> {
        let head = self.repo.head()?;
        let branch_name = head.shorthand()?;
        Ok(branch_name.to_string())
    }

    pub fn default_branch(&self) -> anyhow::Result<String> {
        GitHub.get_default_branch()
    }

    pub fn get_pr_default_branch(&self) -> anyhow::Result<String> {
        GitHub.get_pull_request_base_branch()
    }

    pub fn diff_single_commit(
        &self,
        commit_id: String,
        files: Option<Vec<String>>,
        summary: bool,
    ) -> anyhow::Result<GitDiffResult> {
        let oid = self.repo.revparse_single(&commit_id)?.id();
        let commit = self.repo.find_commit(oid)?;
        let head = commit.tree()?;
        let base = commit.parent(0).ok().map(|p| p.tree()).transpose()?;

        let diff = self.repo.diff_tree_to_tree(
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
        let from = self.repo.revparse_single(&from)?.peel_to_tree()?;
        let to = self.repo.revparse_single(&to)?.peel_to_tree()?;
        let diff = self.repo.diff_tree_to_tree(
            Some(&from),
            Some(&to),
            Some(&mut DiffOptions::default()),
        )?;
        Self::diff_to_result(&diff, files, summary)
    }

    fn diff_to_result(
        diff: &Diff,
        files: Option<Vec<String>>,
        summary: bool,
    ) -> anyhow::Result<GitDiffResult> {
        let files_to_get = files
            .unwrap_or_else(Vec::new)
            .into_iter()
            .map(PathBuf::from)
            .map(|p| p.canonicalize().unwrap_or(p))
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
