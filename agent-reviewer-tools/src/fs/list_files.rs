use crate::{AgentTool, tool_description};
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
        description = "The pattern to match files against. '**' can be used to match any number of directories, '*' can be used to match any number of characters in a file or directory name."
    )]
    pattern: String,
    #[schemars(
        description = "The pattern to exclude files against. '**' can be used to match any number of directories, '*' can be used to match any number of characters in a file or directory name."
    )]
    exclude_pattern: Option<String>,
    #[schemars(description = "The root directory to start the search from.")]
    root_dir: Option<String>,
    #[schemars(description = "The maximum number of files to return.")]
    max_files: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ListFilesResult {
    files: Vec<ListedFile>,
    total_matched_files: usize,
    returned_files: usize,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct ListedFile {
    path: String,
    size: usize,
    modified_at: DateTime<Utc>,
}

#[async_trait::async_trait]
impl AgentTool for ListFiles {
    fn tool(&self) -> Tool {
        tool_description::<ListFilesArgs>("listFiles", "Lists files in a directory.")
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: ListFilesArgs = serde_json::from_value(args.clone())?;

        tokio::spawn(async move { for file in glob(&args.pattern)? {} }).await?
    }
}
