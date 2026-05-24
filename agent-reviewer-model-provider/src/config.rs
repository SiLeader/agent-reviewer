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
