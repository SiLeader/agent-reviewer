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

use std::{fs, path::Path};

use crate::fs::check_path_location;
use crate::fs::ignore::{Ignore, normalize_path};
use crate::{AgentTool, tool_description};
use anyhow::Context;
use chrono::{DateTime, Utc};
use genai::chat::Tool;
use glob::glob;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct ListFiles;

#[derive(Debug, Deserialize, JsonSchema)]
struct ListFilesArgs {
    #[schemars(
        required,
        description = "The pattern to match files against. '**' can be used to match any number of directories, '*' can be used to match any number of characters in a file or directory name."
    )]
    pattern: String,
    #[schemars(
        required,
        description = "The pattern to exclude files against. '**' can be used to match any number of directories, '*' can be used to match any number of characters in a file or directory name."
    )]
    exclude_patterns: Option<Vec<String>>,
    #[schemars(required, description = "The root directory to start the search from.")]
    root_dir: Option<String>,
    #[schemars(required, description = "The maximum number of files to return.")]
    max_files: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListFilesResult {
    files: Vec<ListedFile>,
    total_matched_files: usize,
    returned_files: usize,
    truncated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListedFile {
    path: String,
    size: usize,
    modified_at: DateTime<Utc>,
}

#[async_trait::async_trait]
impl AgentTool for ListFiles {
    fn tool(&self) -> Tool {
        tool_description::<ListFilesArgs>("list_files", "Lists files in a directory.")
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: ListFilesArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(move || list_files(args)).await?
    }
}

fn list_files(args: ListFilesArgs) -> anyhow::Result<String> {
    let root_dir = args.root_dir.unwrap_or_else(|| ".".to_string());
    check_path_location(&root_dir)?;

    let root_dir = fs::canonicalize(&root_dir)
        .with_context(|| format!("failed to resolve root directory: {root_dir}"))?;

    let search_pattern = search_pattern(&root_dir, &args.pattern);
    let max_files = args.max_files.unwrap_or(usize::MAX);
    let mut ignore = Ignore::new(&root_dir, args.exclude_patterns.unwrap_or_default());
    let mut files = Vec::new();

    for entry in
        glob(&search_pattern).with_context(|| format!("invalid pattern: {search_pattern}"))?
    {
        let path = entry.with_context(|| format!("failed to read glob entry: {search_pattern}"))?;
        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;

        if !metadata.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(&root_dir)
            .with_context(|| {
                format!(
                    "matched path {} is outside root directory {}",
                    path.display(),
                    root_dir.display()
                )
            })?
            .to_path_buf();

        if ignore.contains(&relative_path, &path)? {
            continue;
        }

        files.push(ListedFile {
            path: normalize_path(&relative_path),
            size: usize::try_from(metadata.len())
                .with_context(|| format!("file is too large: {}", path.display()))?,
            modified_at: DateTime::<Utc>::from(
                metadata.modified().with_context(|| {
                    format!("failed to read modified time for {}", path.display())
                })?,
            ),
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    let total_matched_files = files.len();
    let truncated = files.len() > max_files;
    files.truncate(max_files);

    let result = ListFilesResult {
        returned_files: files.len(),
        files,
        total_matched_files,
        truncated,
    };

    serde_json::to_string(&result).context("failed to serialize listFiles result")
}

pub(super) fn search_pattern(root_dir: &Path, pattern: &str) -> String {
    let pattern_path = Path::new(pattern);

    if pattern_path.is_absolute() {
        pattern.to_string()
    } else {
        root_dir.join(pattern_path).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::write_file;
    use std::{ops::Deref, path::PathBuf};

    #[test]
    fn truncates_results_after_counting_matches() {
        let root = test_root("truncates");
        write_file(&root.join("a.txt"), "a");
        write_file(&root.join("b.txt"), "b");

        let args = ListFilesArgs {
            pattern: "**/*".to_string(),
            exclude_patterns: None,
            root_dir: Some(root.to_string_lossy().into_owned()),
            max_files: Some(1),
        };
        let result: ListFilesResult = serde_json::from_str(&list_files(args).unwrap()).unwrap();

        assert_eq!(paths(result.files), vec!["a.txt"]);
        assert_eq!(result.total_matched_files, 2);
        assert_eq!(result.returned_files, 1);
        assert!(result.truncated);
    }

    fn paths(files: Vec<ListedFile>) -> Vec<String> {
        files.into_iter().map(|file| file.path).collect()
    }

    struct TestRoot(PathBuf);

    impl Deref for TestRoot {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_root(name: &str) -> TestRoot {
        let root = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("metsuke-list-files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        TestRoot(root)
    }
}
