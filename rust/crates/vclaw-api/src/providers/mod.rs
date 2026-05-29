mod anthropic;
mod openai_compat;

use std::collections::BTreeMap;

pub use anthropic::AnthropicClient;
pub use openai_compat::OpenAiCompatClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub api_base: String,
    pub api_key: Option<String>,
    pub default_headers: BTreeMap<String, String>,
}

impl ProviderConfig {
    pub fn new(api_base: impl Into<String>) -> Self {
        Self {
            api_base: api_base.into(),
            api_key: None,
            default_headers: BTreeMap::new(),
        }
    }
}
