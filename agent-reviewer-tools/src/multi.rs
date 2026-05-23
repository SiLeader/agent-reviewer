use crate::{AgentTool, MarkerAgentTool};
use futures::future::join_all;
use genai::chat::{ChatMessage, Tool, ToolCall};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    Called(Vec<ChatMessage>),
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

    pub fn description(&self) -> Vec<Tool> {
        self.description.clone()
    }

    async fn run(&self, call: &ToolCall) -> ChatMessage {
        let tool = match self.tools.get(&call.fn_name) {
            None => {
                return ChatMessage::tool(format!("Unknown tool: {}", call.fn_name));
            }
            Some(t) => t,
        };
        match tool.run(&call.fn_arguments).await {
            Ok(r) => ChatMessage::tool(r),
            Err(e) => ChatMessage::tool(format!("Error running tool '{}': {}", call.fn_name, e)),
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
        ToolCallResponse::Called(
            join_all(non_marker_calls.into_iter().map(|call| self.run(call))).await,
        )
    }
}
