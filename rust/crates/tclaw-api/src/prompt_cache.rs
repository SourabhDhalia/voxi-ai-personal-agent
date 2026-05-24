use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptCacheMode {
    Disabled,
    Ephemeral,
    Persistent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptCacheConfig {
    pub mode: PromptCacheMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breakpoint: Option<String>,
}

impl PromptCacheConfig {
    pub fn disabled() -> Self {
        Self {
            mode: PromptCacheMode::Disabled,
            ttl_seconds: None,
            breakpoint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PromptCacheUsage {
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    pub hit: bool,
}
