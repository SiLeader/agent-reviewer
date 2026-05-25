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

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewModel {
    Light,
    Standard,
    Power,
}

pub(crate) trait Reviewable:
    Serialize + DeserializeOwned + JsonSchema + Send + Sync + 'static
{
    fn model(&self) -> ReviewModel;
}

// normal review unit
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ReviewUnit {
    #[schemars(required, description = "The task to review.")]
    pub task: String,
    #[schemars(required, description = "The files to focus on.")]
    pub focus_files: Vec<String>,
    #[schemars(required, description = "The model to use for the review.")]
    pub model: ReviewModel,
}

impl Reviewable for ReviewUnit {
    fn model(&self) -> ReviewModel {
        self.model
    }
}
