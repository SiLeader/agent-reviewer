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

pub mod builder;
mod concurrency;
pub mod tools;

use agent_reviewer_tools::{CompoundAgentTools, ToolCallResponse};
pub use concurrency::ConcurrencyLimiter;
use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use tracing::{debug, info};

pub struct ReActAgent {
    model_name: String,
    client: Client,
    tools: CompoundAgentTools,
    max_loop_count: usize,
    options: Option<ChatOptions>,
    submit_tool_name: String,
    concurrency_limiter: ConcurrencyLimiter,
}

impl ReActAgent {
    pub fn builder(concurrency_limiter: ConcurrencyLimiter) -> builder::ReActAgentBuilder {
        builder::ReActAgentBuilder::new(concurrency_limiter)
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
    ) -> Self {
        Self {
            model_name,
            client,
            tools,
            max_loop_count,
            options,
            submit_tool_name,
            concurrency_limiter,
        }
    }

    fn create_request(
        &self,
        system: String,
        messages: Vec<ChatMessage>,
        is_last_turn: bool,
    ) -> ChatRequest {
        ChatRequest {
            system: Some(system),
            messages,
            tools: if is_last_turn {
                Some(
                    self.tools
                        .get_tool_description_by_name(&self.submit_tool_name)
                        .into_iter()
                        .collect(),
                )
            } else {
                Some(self.tools.description())
            },
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

            let is_last_turn = i == self.max_loop_count;

            let user = if i == 1 {
                ChatMessage::user(format!("{}\n{user_prompt}", self.step_worder(1)))
            } else if is_last_turn {
                ChatMessage::user(format!(
                    "{}\nThis is the final step. Make sure to call the '{}' tool if you haven't already.",
                    self.step_worder(i),
                    self.submit_tool_name
                ))
            } else {
                ChatMessage::user(self.step_worder(i))
            };
            debug!("User message: {:?}", user);
            messages.push(user);

            let request =
                self.create_request(system_prompt.to_string(), messages.clone(), is_last_turn);
            let response = {
                let _permit = self.concurrency_limiter.acquire().await?;
                self.client
                    .exec_chat(&self.model_name, request, self.options.as_ref())
                    .await?
            };
            debug!("Model response: {:?}", response);

            let fut = self.tools.run_all(response.content.tool_calls());
            messages.push(ChatMessage::assistant(response.content.clone()));
            match fut.await {
                ToolCallResponse::MarkerFound { marker, non_marker } => {
                    if let Some(call) = marker
                        .iter()
                        .find(|call| call.fn_name == self.submit_tool_name)
                    {
                        debug!("Marker tool call: {:?}", call);
                        return Ok(call.fn_arguments.clone());
                    }
                    match self.tools.run_all(non_marker).await {
                        ToolCallResponse::MarkerFound { .. } => {
                            unreachable!("Marker should not be found in non-marker calls");
                        }
                        ToolCallResponse::Called(tool_responses) => {
                            debug!("Non-marker tool responses: {:?}", tool_responses);
                            messages.push(tool_responses);
                        }
                    }
                }
                ToolCallResponse::Called(tool_responses) => {
                    debug!("Tool responses: {:?}", tool_responses);
                    messages.push(tool_responses);
                }
            }
        }
        anyhow::bail!("Exceeded max loop count")
    }
}
