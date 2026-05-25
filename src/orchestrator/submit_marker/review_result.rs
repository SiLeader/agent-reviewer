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

use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitReviewResult;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SubmitReviewResultArgs {
    #[schemars(required, description = "The review result.")]
    pub review_result: String,
}

impl MarkerAgentTool for SubmitReviewResult {
    fn tool(&self) -> Tool {
        tool_description::<SubmitReviewResultArgs>("submit_review_result", "Submit review result")
    }
}
