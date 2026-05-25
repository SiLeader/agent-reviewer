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

mod builder;
mod config;
mod provider;

use crate::builder::{ModelBuilderError, ProviderBuilder};
pub use crate::config::{ModelConfig, ModelProviderConfig};
use genai::ClientBuilder;

pub trait WithProviderConfig: Sized {
    fn with_provider_config(
        self,
        models: Vec<ModelConfig>,
        providers: Vec<ModelProviderConfig>,
    ) -> Result<Self, ModelBuilderError>;
}

impl WithProviderConfig for ClientBuilder {
    fn with_provider_config(
        self,
        models: Vec<ModelConfig>,
        providers: Vec<ModelProviderConfig>,
    ) -> Result<Self, ModelBuilderError> {
        Ok(self.with_service_target_resolver(ProviderBuilder::new(providers, models).build()?))
    }
}
