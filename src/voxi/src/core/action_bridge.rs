use crate::llm::backend::LlmToolDecl;
use serde_json::Value;

pub struct ActionBridge;

impl ActionBridge {
    pub fn new() -> Self {
        ActionBridge
    }

    pub fn start(&mut self) -> bool {
        log::info!("ActionBridge: Started mock action bridge");
        true
    }

    pub fn get_action_declarations(&self) -> Vec<LlmToolDecl> {
        vec![]
    }

    pub fn execute_action(&self, action_id: &str, _args: &Value) -> Value {
        serde_json::json!({
            "error": format!("Actions are not supported on this platform. Action ID: {}", action_id)
        })
    }
}
