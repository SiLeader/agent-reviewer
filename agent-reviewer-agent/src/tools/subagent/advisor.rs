use crate::ReActAgent;
use crate::tools::subagent::explorer::RepoContext;
use crate::tools::subagent::{run_subagent, setup_tools};
use agent_reviewer_tools::{AgentTool, MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ADVISOR_TOOL_DESCRIPTION: &str = "Provide advice or answer on how to solve a question. Use this to provide guidance on how to solve a problem, or to provide a suggestion for how to improve the code or the problem.";
const ADVISOR_SYSTEM_PROMPT: &str = "You are a advisor. Provide advice on how to solve a problem. Read only the files needed to answer the question, and finish by calling submit exactly once with the most relevant advice.";

pub struct Advisor {
    agent: ReActAgent,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct AdvisorArgs {
    question: String,

    #[schemars(
        required,
        description = "Optional repository context that helps the explorer choose where to look, such as language, framework, or root directories."
    )]
    repo_context: Option<RepoContext>,
}

struct Submit;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SubmitArgs {
    answer: String,
    reason: Reason,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Reason {
    files: Vec<ReasonFile>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ReasonFile {
    path: String,
    line_start: usize,
    line_end: usize,
    reason: String,
}

impl MarkerAgentTool for Submit {
    fn tool(&self) -> Tool {
        tool_description::<SubmitArgs>("submit", "Submit advice for the task")
    }
}

impl From<ReActAgent> for Advisor {
    fn from(mut agent: ReActAgent) -> Self {
        setup_tools(&mut agent, Submit);
        Self { agent }
    }
}

#[async_trait::async_trait]
impl AgentTool for Advisor {
    fn tool(&self) -> Tool {
        tool_description::<AdvisorArgs>("request_for_advice", ADVISOR_TOOL_DESCRIPTION)
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        run_subagent(&self.agent, ADVISOR_SYSTEM_PROMPT, args).await
    }
}
