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

use crate::fs::check_path_location;
use crate::fs::ignore::Ignore;
use crate::fs::list_files::search_pattern;
use crate::{AgentTool, tool_description};
use anyhow::Context;
use futures::future::join_all;
use genai::chat::Tool;
use glob::glob;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

pub struct SearchFile;

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchFileArgs {
    words: Vec<String>,
    file_pattern: String,
    #[schemars(
        required,
        description = "Optional root directory to search within. Omit or set to null to search the entire repository."
    )]
    root_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchFileResult {
    found_files: Vec<FoundFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FoundFile {
    path: String,
    lines: Vec<FoundLine>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FoundLine {
    line_number: usize,
    content: String,
}

#[async_trait::async_trait]
impl AgentTool for SearchFile {
    fn tool(&self) -> Tool {
        tool_description::<SearchFileArgs>(
            "search_file",
            "Searches for files containing specific words and excluding others.",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: SearchFileArgs = serde_json::from_value(args.clone())?;

        let root_dir = args.root_dir.unwrap_or_else(|| ".".to_string());
        check_path_location(&root_dir)?;

        let root_dir = tokio::fs::canonicalize(&root_dir).await?;
        let pattern = search_pattern(&root_dir, &args.file_pattern);
        let mut ignore = Ignore::new(&root_dir, Vec::new());

        let mut tasks = Vec::new();
        for file in glob(&pattern)? {
            let file = file?;
            let metadata = tokio::fs::metadata(&file)
                .await
                .with_context(|| format!("failed to read metadata for {}", file.display()))?;

            if !metadata.is_file() {
                continue;
            }

            let file = tokio::fs::canonicalize(&file)
                .await
                .with_context(|| format!("failed to resolve matched path {}", file.display()))?;
            let relative_path = file
                .strip_prefix(&root_dir)
                .with_context(|| {
                    format!(
                        "matched path {} is outside root directory {}",
                        file.display(),
                        root_dir.display()
                    )
                })?
                .to_path_buf();

            if ignore.contains(&relative_path, &file)? {
                continue;
            }

            tasks.push(Self::find_impl(file, &args.words));
        }
        let results = join_all(tasks)
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?;

        let res = SearchFileResult {
            found_files: results.into_iter().flatten().collect(),
        };

        Ok(serde_json::to_string(&res)?)
    }
}

impl SearchFile {
    async fn find_impl(file: PathBuf, includes: &[String]) -> anyhow::Result<Option<FoundFile>> {
        let content = tokio::fs::read_to_string(&file).await?;

        let mut lines = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if includes.iter().any(|w| line.contains(w)) {
                lines.push(FoundLine {
                    line_number: i + 1,
                    content: line.to_string(),
                });
            }
        }

        Ok(if lines.is_empty() {
            None
        } else {
            Some(FoundFile {
                path: file.display().to_string(),
                lines,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::write_file;
    use std::{
        fs,
        ops::Deref,
        path::{Path, PathBuf},
    };

    #[tokio::test]
    async fn skips_files_ignored_by_root_gitignore() {
        let root = test_root("root_gitignore");
        write_file(&root.join(".gitignore"), "*.log\n");
        write_file(&root.join("keep.txt"), "needle\n");
        write_file(&root.join("drop.log"), "needle\n");

        let result = search(root.as_ref()).await;

        assert_eq!(
            paths(result),
            vec![root.join("keep.txt").display().to_string()]
        );
    }

    #[tokio::test]
    async fn skips_files_ignored_by_nested_gitignore() {
        let root = test_root("nested_gitignore");
        write_file(&root.join("src/.gitignore"), "generated/\n");
        write_file(&root.join("src/main.rs"), "needle\n");
        write_file(&root.join("src/generated/out.rs"), "needle\n");

        let result = search(root.as_ref()).await;

        assert_eq!(
            paths(result),
            vec![root.join("src/main.rs").display().to_string()]
        );
    }

    async fn search(root: &Path) -> SearchFileResult {
        let args = serde_json::json!({
            "words": ["needle"],
            "file_pattern": "**/*",
            "root_dir": root.to_string_lossy(),
        });

        serde_json::from_str(&SearchFile.run(&args).await.unwrap()).unwrap()
    }

    fn paths(result: SearchFileResult) -> Vec<String> {
        let mut paths = result
            .found_files
            .into_iter()
            .map(|file| file.path)
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    struct TestRoot(PathBuf);

    impl AsRef<Path> for TestRoot {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

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
            .join(format!("metsuke-search-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        TestRoot(root)
    }
}
