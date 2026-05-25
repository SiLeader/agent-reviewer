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

#[derive(Debug, Serialize)]
struct SearchFileResult {
    found_files: Vec<FoundFile>,
}

#[derive(Debug, Serialize)]
struct FoundFile {
    path: String,
    lines: Vec<FoundLine>,
}

#[derive(Debug, Serialize)]
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
            file.strip_prefix(&root_dir).with_context(|| {
                format!(
                    "matched path {} is outside root directory {}",
                    file.display(),
                    root_dir.display()
                )
            })?;

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
