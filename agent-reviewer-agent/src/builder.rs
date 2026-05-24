use crate::ReActAgent;
use crate::concurrency::ConcurrencyLimiter;
use crate::notes::ReActAgentNoteManager;
use agent_reviewer_tools::{AgentTool, CompoundAgentTools, MarkerAgentTool};
use genai::Client;
use genai::chat::ChatOptions;
use std::sync::Arc;

#[derive(Clone)]
pub struct ReActAgentBuilder {
    model_name: String,
    client: Client,
    tools: Vec<Arc<dyn AgentTool>>,
    marker_tools: Vec<Arc<dyn MarkerAgentTool>>,
    max_loop_count: usize,
    options: Option<ChatOptions>,
    submit_tool_name: String,
    concurrency_limiter: ConcurrencyLimiter,
    note_manager: ReActAgentNoteManager,
}

impl ReActAgentBuilder {
    pub fn override_model_name(mut self, model_name: String) -> Self {
        self.model_name = model_name;
        self
    }

    pub fn override_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    pub fn override_tools(mut self, tools: Vec<Arc<dyn AgentTool>>) -> Self {
        self.tools = tools;
        self
    }

    pub fn override_marker_tools(mut self, marker_tools: Vec<Arc<dyn MarkerAgentTool>>) -> Self {
        self.marker_tools = marker_tools;
        self
    }

    pub fn override_max_loop_count(mut self, max_loop_count: usize) -> Self {
        self.max_loop_count = max_loop_count;
        self
    }

    pub fn override_submit_tool(mut self, tool: Arc<dyn MarkerAgentTool>) -> Self {
        self.submit_tool_name = tool.tool().name.to_string();
        self.marker_tools.push(tool);
        self
    }

    pub fn clear_tools(mut self) -> Self {
        self.tools = Vec::new();
        self
    }

    pub fn clear_marker_tools(mut self) -> Self {
        self.marker_tools = Vec::new();
        self
    }

    pub fn override_options(mut self, options: ChatOptions) -> Self {
        self.options = Some(options);
        self
    }

    pub fn build(self, agent_id: String) -> ReActAgent {
        ReActAgent::new(
            self.model_name,
            self.client,
            CompoundAgentTools::new(self.tools, self.marker_tools),
            self.max_loop_count,
            self.submit_tool_name,
            self.options,
            self.concurrency_limiter,
            self.note_manager.create_note(agent_id),
        )
    }
}

impl ReActAgentBuilder {
    pub fn new(id: String, concurrency_limiter: ConcurrencyLimiter) -> Self {
        Self {
            model_name: "".to_string(),
            client: Client::default(),
            tools: Vec::new(),
            marker_tools: Vec::new(),
            max_loop_count: 10,
            options: None,
            submit_tool_name: "submit".to_string(),
            concurrency_limiter,
            note_manager: ReActAgentNoteManager::new(id),
        }
    }
}
