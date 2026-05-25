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

use crate::orchestrator::review_unit::Reviewable;
use agent_reviewer_tools::{MarkerAgentTool, tool_description};
use genai::chat::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) struct SubmitTriage<R> {
    _phantom: std::marker::PhantomData<R>,
}

impl<R> Default for SubmitTriage<R>
where
    R: Reviewable,
{
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SubmitTriageArgs<R> {
    #[schemars(required, description = "The review units to submit.")]
    pub review_units: Vec<R>,
}

impl<R> MarkerAgentTool for SubmitTriage<R>
where
    R: Reviewable,
{
    fn tool(&self) -> Tool {
        tool_description::<SubmitTriageArgs<R>>("submit_triage", "Submit triage result")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::ReviewModel;
    use crate::orchestrator::review_unit::ReviewUnit;

    #[test]
    fn deserializes_review_units_with_task_and_focus_files() {
        let args: SubmitTriageArgs<ReviewUnit> = serde_json::from_value(serde_json::json!({
            "review_units": [
                {
                    "task": "Review error handling in the CLI entrypoint",
                    "focus_files": ["src/main.rs"],
                    "model": "standard"
                }
            ]
        }))
        .unwrap();

        assert_eq!(args.review_units.len(), 1);
        assert_eq!(
            args.review_units[0].task,
            "Review error handling in the CLI entrypoint"
        );
        assert_eq!(args.review_units[0].focus_files, vec!["src/main.rs"]);
        assert!(matches!(args.review_units[0].model, ReviewModel::Standard));
    }
}
