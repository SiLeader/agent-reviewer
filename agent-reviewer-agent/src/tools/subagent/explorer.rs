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

use crate::ReActAgent;
use crate::tools::subagent::{run_subagent, setup_tools};
use agent_reviewer_tools::{AgentTool, MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const EXPLORER_TOOL_DESCRIPTION: &str = "Delegate repository exploration when you need broader context than a single diff or file provides. Use this to trace symbols across files, find related tests or configuration, and map changed files to the code that actually drives behavior.";
const EXPLORER_SYSTEM_PROMPT: &str = "You are a code explorer. Investigate the repository on behalf of another agent. Read only the files needed to answer the task, trace relationships across modules when useful, and finish by calling submit exactly once with the most relevant files, ranges, relationships, unanswered questions, and a confidence score.";

pub struct Explorer {
    agent: ReActAgent,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ExplorerArgs {
    #[schemars(
        required,
        description = "The concrete question to answer for the caller. Describe the behavior, risk, or relationship that needs cross-file investigation."
    )]
    task: String,
    #[schemars(
        required,
        description = "Optional changed or already-relevant files to anchor the search. Omit or set to null when the caller does not know them yet."
    )]
    changed_files: Option<Vec<String>>,
    #[schemars(
        required,
        description = "Optional functions, types, modules, routes, config keys, or other identifiers to trace through the repository."
    )]
    symbols: Option<Vec<String>>,
    #[schemars(
        required,
        description = "Optional seed files or directories to inspect first before branching out to related code."
    )]
    initial_files: Option<Vec<String>>,
    #[schemars(
        required,
        description = "Optional limits for keeping the exploration focused. Omit or set to null when no special limits are needed."
    )]
    constraints: Option<Constraints>,
    #[schemars(
        required,
        description = "Optional repository context that helps the explorer choose where to look, such as language, framework, or root directories."
    )]
    repo_context: Option<RepoContext>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Constraints {
    #[schemars(required, description = "The maximum number of files to return.")]
    max_files: usize,
    #[schemars(
        required,
        description = "The maximum number of snippets to return per file."
    )]
    max_snippets_per_file: usize,
    #[schemars(
        required,
        description = "The maximum number of symbols to return per file."
    )]
    max_depth: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct RepoContext {
    #[schemars(
        required,
        description = "Optional primary programming language for the area being explored."
    )]
    language: Option<String>,
    #[schemars(
        required,
        description = "Optional framework, runtime, or platform used by the relevant code."
    )]
    framework: Option<String>,
    #[schemars(
        required,
        description = "Optional root directories to search within the repository."
    )]
    root_directories: Option<Vec<String>>,
}

struct Submit;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SubmitArgs {
    summary: String,
    files: Vec<FoundFile>,
    relationships: Vec<Relationship>,
    unanswered_questions: Vec<String>,
    #[schemars(range(min = 0, max = 1))]
    confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FoundFile {
    path: String,
    relevance: Relevance,
    content: String,
    ranges: Vec<Range>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Relevance {
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Range {
    #[schemars(description = "The start line of the range.")]
    start_line: Option<usize>,
    #[schemars(description = "The end line of the range.")]
    end_line: Option<usize>,
    #[schemars(description = "Optional symbol or identifier associated with this range.")]
    symbol: Option<String>,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Relationship {
    from: String,
    to: String,
    relationship_type: RelationshipType,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum RelationshipType {
    Imports,
    Calls,
    Tests,
    Config,
    Ownership,
    SimilarCode,
}

impl MarkerAgentTool for Submit {
    fn tool(&self) -> Tool {
        tool_description::<SubmitArgs>("submit", "Submit the task")
    }
}

impl From<ReActAgent> for Explorer {
    fn from(mut agent: ReActAgent) -> Self {
        setup_tools(&mut agent, Submit);
        Self { agent }
    }
}

#[async_trait::async_trait]
impl AgentTool for Explorer {
    fn tool(&self) -> Tool {
        tool_description::<ExplorerArgs>("explorer", EXPLORER_TOOL_DESCRIPTION)
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        run_subagent(&self.agent, EXPLORER_SYSTEM_PROMPT, args).await
    }
}
