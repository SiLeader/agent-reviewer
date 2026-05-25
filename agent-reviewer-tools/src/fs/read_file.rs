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
use crate::{AgentTool, tool_description};
use anyhow::Context;
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncBufReadExt;

pub struct ReadFile;

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileArgs {
    #[schemars(required, description = "The path to the file to read.")]
    path: String,
    #[schemars(
        required,
        description = "The line number to start reading from (inclusive). If not provided, reads from the beginning of the file."
    )]
    start_line: Option<usize>,
    #[schemars(
        required,
        description = "The line number to end reading at (inclusive). If not provided, reads until the end of the file."
    )]
    end_line: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ReadFileResult {
    path: String,
    total_lines: usize,
    returned_lines_range: String,
    truncated: bool,
    content: String,
}

#[async_trait::async_trait]
impl AgentTool for ReadFile {
    fn tool(&self) -> Tool {
        tool_description::<ReadFileArgs>("read_file", "Reads a file from the file system.")
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: ReadFileArgs = serde_json::from_value(args.clone())?;

        check_path_location(&args.path)?;

        let file = tokio::fs::File::open(&args.path).await?;
        let reader = tokio::io::BufReader::new(file);

        let mut lines = reader.lines();
        let mut line_number = 0;

        let start_line = args.start_line.unwrap_or(1);
        let end_line = args.end_line;
        let mut content = Vec::new();

        loop {
            let Some(line) = lines.next_line().await? else {
                break;
            };

            line_number += 1;
            if start_line <= line_number && end_line.is_none_or(|end_line| line_number <= end_line)
            {
                content.push(line);
            }
        }

        let total_lines = line_number;
        let returned_end_line = end_line
            .map(|end_line| end_line.min(total_lines))
            .unwrap_or(total_lines);
        let result = ReadFileResult {
            path: args.path,
            total_lines,
            returned_lines_range: format!("{}-{}", start_line, returned_end_line),
            truncated: start_line > 1 || end_line.is_some_and(|end_line| total_lines > end_line),
            content: content.join("\n"),
        };

        serde_json::to_string(&result).context("failed to serialize readFile result")
    }
}
