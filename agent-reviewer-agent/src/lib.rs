pub mod builder;
mod tools;

use agent_reviewer_tools::{CompoundAgentTools, ToolCallResponse};
use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use tracing::{error, info};

pub struct ReActAgent {
    model_name: String,
    client: Client,
    tools: CompoundAgentTools,
    max_loop_count: usize,
    options: Option<ChatOptions>,
    submit_tool_name: String,
}

impl ReActAgent {
    pub fn builder() -> builder::ReActAgentBuilder {
        builder::ReActAgentBuilder::default()
    }

    pub fn new(
        model_name: String,
        client: Client,
        tools: CompoundAgentTools,
        max_loop_count: usize,
        submit_tool_name: String,
        options: Option<ChatOptions>,
    ) -> Self {
        Self {
            model_name,
            client,
            tools,
            max_loop_count,
            options,
            submit_tool_name,
        }
    }

    fn create_request(&self, system: String, messages: Vec<ChatMessage>) -> ChatRequest {
        ChatRequest {
            system: Some(system),
            messages,
            tools: Some(self.tools.description()),
            previous_response_id: None,
            store: Some(false),
        }
    }

    pub async fn run(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let mut messages = vec![ChatMessage::user(user_prompt)];

        for i in 1..=self.max_loop_count {
            info!(
                "Starting step {}/{} with model {}",
                i, self.max_loop_count, self.model_name
            );

            let request = self.create_request(system_prompt.to_string(), messages.clone());
            let response = self
                .client
                .exec_chat(&self.model_name, request, self.options.as_ref())
                .await?;

            match self.tools.run_all(response.content.tool_calls()).await {
                ToolCallResponse::MarkerFound { marker, non_marker } => {
                    if let Some(call) = marker
                        .iter()
                        .find(|call| call.fn_name == self.submit_tool_name)
                    {
                        return Ok(call.fn_arguments.clone());
                    }
                    match self.tools.run_all(non_marker).await {
                        ToolCallResponse::MarkerFound { .. } => {
                            error!("Unexpected marker found");
                            panic!("Unexpected marker found. this is bug");
                        }
                        ToolCallResponse::Called(tool_responses) => {
                            messages.extend(tool_responses);
                        }
                    }
                }
                ToolCallResponse::Called(tool_responses) => {
                    messages.extend(tool_responses);
                }
            }

            messages.push(ChatMessage::assistant(response.content.clone()));
        }
        anyhow::bail!("Exceeded max loop count")
    }
}
