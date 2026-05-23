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
