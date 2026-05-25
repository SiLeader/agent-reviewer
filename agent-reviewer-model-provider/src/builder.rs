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

use crate::config::{ModelConfig, ModelProviderConfig, ModelProviderContent};
use crate::provider::{ModelProvider, ServiceTargetOpt};
use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{ModelIden, ModelName};
use std::collections::HashMap;
use tracing::debug;

#[derive(thiserror::Error, Debug)]
pub enum ModelBuilderError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
}

pub(crate) struct ProviderBuilder {
    models: Vec<ModelConfig>,
    provider: HashMap<String, ModelProviderConfig>,
}

impl ProviderBuilder {
    pub(crate) fn new(providers: Vec<ModelProviderConfig>, models: Vec<ModelConfig>) -> Self {
        Self {
            models,
            provider: providers
                .into_iter()
                .map(|config| (config.id.clone(), config))
                .collect(),
        }
    }

    fn build_for_model(
        model: ModelConfig,
        provider: &HashMap<String, ModelProviderConfig>,
    ) -> Result<ServiceTargetOpt, ModelBuilderError> {
        let provider = provider
            .get(model.provider.as_str())
            .ok_or(ModelBuilderError::ProviderNotFound(model.provider))?;

        let (endpoint, adapter_kind, auth) = match &provider.content {
            ModelProviderContent::OpenAI { key_env, base_url } => (
                base_url.clone().map(Endpoint::from_owned),
                AdapterKind::OpenAI,
                AuthData::FromEnv(key_env.clone()),
            ),
            ModelProviderContent::Anthropic { key_env, base_url } => (
                base_url.clone().map(Endpoint::from_owned),
                AdapterKind::Anthropic,
                AuthData::FromEnv(key_env.clone()),
            ),
            ModelProviderContent::GitHub { key_env } => (
                Some(Endpoint::from_static("https://models.github.ai/inference/")),
                AdapterKind::GithubCopilot,
                AuthData::FromEnv(
                    key_env
                        .clone()
                        .unwrap_or_else(|| "GITHUB_TOKEN".to_string()),
                ),
            ),
            ModelProviderContent::Bedrock {
                access_key_env,
                secret_access_key_env,
                region,
            } => (
                None,
                AdapterKind::BedrockSigv4,
                AuthData::MultiKeys(HashMap::from([
                    (
                        "aws_access_key_id".to_string(),
                        std::env::var(access_key_env.as_deref().unwrap_or("AWS_ACCESS_KEY_ID"))
                            .unwrap_or_default(),
                    ),
                    (
                        "aws_secret_access_key".to_string(),
                        std::env::var(
                            secret_access_key_env
                                .as_deref()
                                .unwrap_or("AWS_SECRET_ACCESS_KEY"),
                        )
                        .unwrap_or_default(),
                    ),
                    ("aws_region".to_string(), region.clone()),
                ])),
            ),
        };
        debug!(
            "Selected service target for '{}' with adapter: {:?}",
            model.model, adapter_kind
        );
        Ok(ServiceTargetOpt {
            endpoint,
            auth: Some(auth),
            model: Some(ModelIden::new(adapter_kind, &model.model)),
        })
    }

    pub(crate) fn build(self) -> Result<ServiceTargetResolver, ModelBuilderError> {
        let mut models = HashMap::with_capacity(self.models.len());
        for model in self.models {
            models.insert(
                ModelName::new(model.id.clone()),
                Self::build_for_model(model, &self.provider)?,
            );
        }
        Ok(ModelProvider::create_resolver(models))
    }
}
