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

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub(crate) id: String,
    pub(crate) model: String,
    pub(crate) provider: String,
}

impl ModelConfig {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelProviderConfig {
    pub(crate) id: String,
    #[serde(flatten)]
    pub(crate) content: ModelProviderContent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum ModelProviderContent {
    OpenAI {
        base_url: Option<String>,
        key_env: String,
    },
    Anthropic {
        base_url: Option<String>,
        key_env: String,
    },
    GitHub {
        key_env: Option<String>,
    },
    Bedrock {
        region: String,
        access_key_env: Option<String>,
        secret_access_key_env: Option<String>,
    },
}
