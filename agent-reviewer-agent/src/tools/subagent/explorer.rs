use crate::ReActAgent;
use agent_reviewer_tools::fs::{ListFiles, ReadFile, SearchFile};
use agent_reviewer_tools::git::{
    GitCurrentBranch, GitDefaultBranch, GitDiffCommitRange, GitDiffSingleCommit,
    GitDiffSummaryCommitRange, GitDiffSummarySingleCommit, GitPrBaseBranch,
};
use agent_reviewer_tools::{AgentTool, MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

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
        description = "Optional changed or already-relevant files to anchor the search. Omit or set to null when the caller does not know them yet."
    )]
    changed_files: Option<Vec<String>>,
    #[schemars(
        description = "Optional functions, types, modules, routes, config keys, or other identifiers to trace through the repository."
    )]
    symbols: Option<Vec<String>>,
    #[schemars(
        description = "Optional seed files or directories to inspect first before branching out to related code."
    )]
    initial_files: Option<Vec<String>>,
    #[schemars(
        description = "Optional limits for keeping the exploration focused. Omit or set to null when no special limits are needed."
    )]
    constraints: Option<Constraints>,
    #[schemars(
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
struct RepoContext {
    #[schemars(description = "Optional primary programming language for the area being explored.")]
    language: Option<String>,
    #[schemars(
        description = "Optional framework, runtime, or platform used by the relevant code."
    )]
    framework: Option<String>,
    #[schemars(description = "Optional root directories to search within the repository.")]
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
        agent.tools.add_tool(Arc::new(ReadFile));
        agent.tools.add_tool(Arc::new(ListFiles));
        agent.tools.add_tool(Arc::new(SearchFile));
        agent.tools.add_tool(Arc::new(GitDiffSingleCommit));
        agent.tools.add_tool(Arc::new(GitDiffCommitRange));
        agent.tools.add_tool(Arc::new(GitDiffSummarySingleCommit));
        agent.tools.add_tool(Arc::new(GitDiffSummaryCommitRange));
        agent.tools.add_tool(Arc::new(GitPrBaseBranch));
        agent.tools.add_tool(Arc::new(GitDefaultBranch));
        agent.tools.add_tool(Arc::new(GitCurrentBranch));
        agent.tools.add_marker(Arc::new(Submit));
        agent.submit_tool_name = Submit.tool().name.to_string();

        Self { agent }
    }
}

#[async_trait::async_trait]
impl AgentTool for Explorer {
    fn tool(&self) -> Tool {
        tool_description::<ExplorerArgs>("explorer", EXPLORER_TOOL_DESCRIPTION)
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let value = self
            .agent
            .run(EXPLORER_SYSTEM_PROMPT, &args.to_string())
            .await?;
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{EXPLORER_TOOL_DESCRIPTION, ExplorerArgs};
    use agent_reviewer_tools::tool_description;

    #[test]
    fn explorer_tool_description_mentions_cross_file_context() {
        let tool = tool_description::<ExplorerArgs>("explorer", EXPLORER_TOOL_DESCRIPTION);

        assert!(
            tool.description
                .expect("tool description should be present")
                .contains("trace symbols across files")
        );
    }

    #[test]
    fn explorer_schema_only_requires_the_task() {
        let tool = tool_description::<ExplorerArgs>("explorer", EXPLORER_TOOL_DESCRIPTION);
        let schema = tool.schema.expect("schema should be present");

        assert_eq!(schema["required"], serde_json::json!(["task"]));
    }
}
