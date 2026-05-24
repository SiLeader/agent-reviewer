use crate::{AgentTool, MarkerAgentTool};
use futures::future::join_all;
use genai::chat::{ChatMessage, MessageContent, Tool, ToolCall, ToolResponse};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone, Default)]
pub struct CompoundAgentTools {
    tools: HashMap<String, Arc<dyn AgentTool>>,
    marker: HashSet<String>,
    description: Vec<Tool>,
}

pub enum ToolCallResponse<'a> {
    MarkerFound {
        marker: Vec<&'a ToolCall>,
        non_marker: Vec<&'a ToolCall>,
    },
    Called(ChatMessage),
}

impl CompoundAgentTools {
    pub fn new(tools: Vec<Arc<dyn AgentTool>>, marker: Vec<Arc<dyn MarkerAgentTool>>) -> Self {
        let mut description = tools.iter().map(|tool| tool.tool()).collect::<Vec<_>>();
        description.extend(marker.iter().map(|m| m.tool()));

        let marker = marker
            .into_iter()
            .map(|m| m.tool().name.to_string())
            .collect();

        let tools = tools
            .into_iter()
            .map(|tool| (tool.tool().name.to_string(), tool))
            .collect();

        Self {
            tools,
            description,
            marker,
        }
    }

    pub fn add_tool(&mut self, tool: Arc<dyn AgentTool>) {
        self.description.push(tool.tool());
        self.tools.insert(tool.tool().name.to_string(), tool);
    }

    pub fn add_marker(&mut self, marker: Arc<dyn MarkerAgentTool>) {
        self.description.push(marker.tool());
        self.marker.insert(marker.tool().name.to_string());
    }

    pub fn description(&self) -> Vec<Tool> {
        self.description.clone()
    }

    async fn run(&self, call: &ToolCall) -> String {
        debug!(
            "Tool '{}' called with arguments: {}",
            call.fn_name, call.fn_arguments
        );
        let start = std::time::Instant::now();
        let tool = match self.tools.get(&call.fn_name) {
            None => {
                return format!("Unknown tool: {}", call.fn_name);
            }
            Some(t) => t,
        };
        let fut = tool.run(&call.fn_arguments);
        let duration = start.elapsed();
        info!("Tool '{}' completed in {:?}", call.fn_name, duration);
        match fut.await {
            Ok(r) => r,
            Err(e) => format!("Error running tool '{}': {}", call.fn_name, e),
        }
    }

    pub async fn run_all<'a>(&self, calls: Vec<&'a ToolCall>) -> ToolCallResponse<'a> {
        let (marker_calls, non_marker_calls): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|call| self.marker.contains(&call.fn_name));

        if !marker_calls.is_empty() {
            return ToolCallResponse::MarkerFound {
                marker: marker_calls,
                non_marker: non_marker_calls,
            };
        }
        let calls = join_all(
            non_marker_calls
                .into_iter()
                .map(|call| async { (call.call_id.clone(), self.run(call).await) }),
        )
        .await
        .into_iter()
        .map(|(id, response)| ToolResponse::new(id, response))
        .collect::<Vec<_>>();
        ToolCallResponse::Called(ChatMessage::tool(MessageContent::from_tool_responses(
            calls,
        )))
    }
}
