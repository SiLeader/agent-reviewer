use crate::ReActAgent;
use crate::concurrency::ConcurrencyLimiter;
use agent_reviewer_tools::{AgentTool, CompoundAgentTools, MarkerAgentTool, tool_description};
use genai::Client;
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub struct Explorer {
    agent: ReActAgent,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ExplorerArgs {
    task: String,
    changed_files: Option<Vec<String>>,
    symbols: Option<Vec<String>>,
    initial_files: Option<Vec<String>>,
    constraints: Option<Constraints>,
    repo_context: Option<RepoContext>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Constraints {
    max_files: usize,
    max_snippets_per_file: usize,
    max_depth: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct RepoContext {
    language: Option<String>,
    framework: Option<String>,
    root_directories: Option<Vec<String>>,
}

struct Submit;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SubmitArgs {
    summary: String,
    files: Vec<FoundFile>,
    relationships: Vec<Relationship>,
    unanswered_questions: Vec<String>,
    #[schemars(range(min = 0, max = 1))]
    confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FoundFile {
    path: String,
    relevance: Relevance,
    content: String,
    ranges: Vec<Range>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum Relevance {
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Range {
    start_line: Option<usize>,
    end_line: Option<usize>,
    symbol: Option<String>,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Relationship {
    from: String,
    to: String,
    relationship_type: RelationshipType,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
    fn from(agent: ReActAgent) -> Self {
        Self { agent }
    }
}

impl Explorer {
    pub fn new(
        model_name: String,
        client: Client,
        concurrency_limiter: ConcurrencyLimiter,
    ) -> Self {
        Self::from(ReActAgent::new(
            model_name,
            client,
            CompoundAgentTools::new(vec![], vec![Arc::new(Submit)]),
            10,
            "submit".to_string(),
            None,
            concurrency_limiter,
        ))
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
