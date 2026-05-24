pub mod builder;
mod concurrency;
mod notes;
pub mod tools;

use crate::notes::ReActAgentNote;
use agent_reviewer_tools::{CompoundAgentTools, ToolCallResponse};
pub use concurrency::ConcurrencyLimiter;
use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use tracing::{debug, error, info};

pub struct ReActAgent {
    model_name: String,
    client: Client,
    tools: CompoundAgentTools,
    max_loop_count: usize,
    options: Option<ChatOptions>,
    submit_tool_name: String,
    concurrency_limiter: ConcurrencyLimiter,
    notes: ReActAgentNote,
}

impl ReActAgent {
    pub fn builder(
        session_id: String,
        concurrency_limiter: ConcurrencyLimiter,
    ) -> builder::ReActAgentBuilder {
        builder::ReActAgentBuilder::new(session_id, concurrency_limiter)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_name: String,
        client: Client,
        tools: CompoundAgentTools,
        max_loop_count: usize,
        submit_tool_name: String,
        options: Option<ChatOptions>,
        concurrency_limiter: ConcurrencyLimiter,
        notes: ReActAgentNote,
    ) -> Self {
        Self {
            model_name,
            client,
            tools,
            max_loop_count,
            options,
            submit_tool_name,
            concurrency_limiter,
            notes,
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

    fn step_worder(&self, current: usize) -> String {
        format!("You are in step {} of {}.", current, self.max_loop_count)
    }

    pub async fn run(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let mut messages = Vec::new();

        for i in 1..=self.max_loop_count {
            info!(
                "Starting step {}/{} with model {}",
                i, self.max_loop_count, self.model_name
            );

            if i == 1 {
                messages.push(ChatMessage::user(format!(
                    "{}\n{user_prompt}",
                    self.step_worder(1)
                )));
            } else {
                messages.push(ChatMessage::user(self.step_worder(i)));
            }
            self.notes
                .write(serde_json::to_string_pretty(&messages)?)
                .await?;

            let request = self.create_request(system_prompt.to_string(), messages.clone());
            let response = {
                let _permit = self.concurrency_limiter.acquire().await?;
                self.client
                    .exec_chat(&self.model_name, request, self.options.as_ref())
                    .await?
            };

            let fut = self.tools.run_all(response.content.tool_calls());
            messages.push(ChatMessage::assistant(response.content.clone()));
            self.notes
                .write(serde_json::to_string_pretty(&messages)?)
                .await?;

            match fut.await {
                ToolCallResponse::MarkerFound { marker, non_marker } => {
                    if let Some(call) = marker
                        .iter()
                        .find(|call| call.fn_name == self.submit_tool_name)
                    {
                        return Ok(call.fn_arguments.clone());
                    }
                    match self.tools.run_all(non_marker).await {
                        ToolCallResponse::MarkerFound { .. } => {
                            unreachable!("Marker should not be found in non-marker calls");
                        }
                        ToolCallResponse::Called(tool_responses) => {
                            messages.push(tool_responses);
                            self.notes
                                .write(serde_json::to_string_pretty(&messages)?)
                                .await?;
                        }
                    }
                }
                ToolCallResponse::Called(tool_responses) => {
                    messages.push(tool_responses);
                    self.notes
                        .write(serde_json::to_string_pretty(&messages)?)
                        .await?;
                }
            }
        }
        anyhow::bail!("Exceeded max loop count")
    }
}
