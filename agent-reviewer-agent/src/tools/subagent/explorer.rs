use crate::ReActAgent;
use agent_reviewer_tools::fs::{ListFiles, ReadFile};
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

pub struct Explorer {
    agent: ReActAgent,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ExplorerArgs {
    #[schemars(required, description = "The task to explore.")]
    task: String,
    #[schemars(
        required,
        description = "The files that have changed since the last commit."
    )]
    changed_files: Option<Vec<String>>,
    #[schemars(required, description = "The symbols to search for.")]
    symbols: Option<Vec<String>>,
    #[schemars(required, description = "The initial files to start with.")]
    initial_files: Option<Vec<String>>,
    #[schemars(required, description = "The constraints to apply to the search.")]
    constraints: Option<Constraints>,
    #[schemars(required, description = "The repository context to use.")]
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
    #[schemars(required, description = "The programming language of the repository.")]
    language: Option<String>,
    #[schemars(required, description = "The framework used in the repository.")]
    framework: Option<String>,
    #[schemars(
        required,
        description = "The root directories to search within the repository."
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
    #[schemars(required, description = "The start line of the range.")]
    start_line: Option<usize>,
    #[schemars(required, description = "The end line of the range.")]
    end_line: Option<usize>,
    #[schemars(required, description = "The start column of the range.")]
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
        agent.tools.add_tool(Arc::new(GitDiffSingleCommit));
        agent.tools.add_tool(Arc::new(GitDiffCommitRange));
        agent.tools.add_tool(Arc::new(GitDiffSummarySingleCommit));
        agent.tools.add_tool(Arc::new(GitDiffSummaryCommitRange));
        agent.tools.add_tool(Arc::new(GitPrBaseBranch));
        agent.tools.add_tool(Arc::new(GitDefaultBranch));
        agent.tools.add_tool(Arc::new(GitCurrentBranch));
        agent.tools.add_marker(Arc::new(Submit));

        Self { agent }
    }
}

#[async_trait::async_trait]
impl AgentTool for Explorer {
    fn tool(&self) -> Tool {
        tool_description::<ExplorerArgs>(
            "explorer",
            "Explore the codebase and find the most interesting files and symbols",
        )
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let value = self
            .agent
            .run("You are a code explorer", &args.to_string())
            .await?;
        Ok(value.to_string())
    }
}
