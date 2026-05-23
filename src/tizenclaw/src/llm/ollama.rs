//! Ollama local LLM backend — uses serde_json + ureq.

#![allow(clippy::all)]

use super::backend::*;
use crate::infra::http_client;
use serde_json::{json, Value};

pub struct OllamaBackend {
    model: String,
    endpoint: String,
}

impl Default for OllamaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaBackend {
    pub fn new() -> Self {
        OllamaBackend {
            model: "llama3".into(),
            endpoint: "http://localhost:11434".into(),
        }
    }
}

#[async_trait::async_trait]
impl LlmBackend for OllamaBackend {
    fn initialize(&mut self, config: &Value) -> bool {
        if let Some(m) = config["model"].as_str() {
            self.model = m.into();
        }
        if let Some(e) = config["endpoint"].as_str() {
            self.endpoint = e.into();
        }
        true
    }

    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: &[LlmToolDecl],
        _on_chunk: Option<&(dyn Fn(&str) + Send + Sync)>,
        system_prompt: &str,
        max_tokens: Option<u32>,
    ) -> LlmResponse {
        let mut valid_tools = std::collections::HashSet::new();
        for t in tools {
            valid_tools.insert(t.name.as_str());
        }

        let mut msgs = vec![];
        if !system_prompt.is_empty() {
            msgs.push(json!({"role": "system", "content": system_prompt}));
        }
        for msg in messages {
            let text = msg.text.trim().to_string();
            let mut is_downgraded = false;
            if msg.role == "tool" && !valid_tools.contains(msg.tool_name.as_str()) {
                is_downgraded = true;
            }
            if !msg.tool_calls.is_empty()
                && msg
                    .tool_calls
                    .iter()
                    .any(|tc| !valid_tools.contains(tc.name.as_str()))
            {
                is_downgraded = true;
            }

            if is_downgraded {
                if msg.role == "tool" {
                    msgs.push(json!({"role": "user", "content": format!("[Historical Tool Result for '{}']: {}", msg.tool_name, msg.tool_result)}));
                } else if !msg.tool_calls.is_empty() {
                    let calls_text = msg
                        .tool_calls
                        .iter()
                        .map(|tc| format!("Called tool '{}' with args '{}'", tc.name, tc.args))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let full_text = if text.is_empty() {
                        calls_text
                    } else {
                        format!("{}\n\n{}", text, calls_text)
                    };
                    msgs.push(json!({"role": "assistant", "content": full_text}));
                } else if !text.is_empty() {
                    msgs.push(json!({"role": msg.role, "content": text}));
                }
            } else if msg.role == "tool" {
                msgs.push(json!({"role": "tool", "content": msg.tool_result.to_string(), "tool_call_id": msg.tool_call_id}));
            } else if !msg.tool_calls.is_empty() {
                let tcs: Vec<Value> = msg
                    .tool_calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id, "type": "function",
                            "function": {"name": tc.name, "arguments": tc.args}
                        })
                    })
                    .collect();
                let mut m = json!({"role": "assistant", "tool_calls": tcs});
                if !text.is_empty() {
                    m["content"] = json!(text);
                }
                msgs.push(m);
            } else if !text.is_empty() {
                msgs.push(json!({"role": msg.role, "content": text}));
            }
        }

        let mut req = json!({
            "model": self.model,
            "messages": msgs,
            "stream": false,
            "options": {
                "num_predict": max_tokens.unwrap_or(4096)
            }
        });

        if !tools.is_empty() {
            let tool_arr: Vec<Value> = tools.iter().map(|t| json!({
                "type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.parameters}
            })).collect();
            req["tools"] = Value::Array(tool_arr);
        }

        let url = format!("{}/api/chat", self.endpoint);
        let http_resp = http_client::http_post(&url, &[], &req.to_string(), 1, 300).await;

        let mut resp = LlmResponse::default();
        resp.http_status = http_resp.status_code;
        if !http_resp.success {
            resp.error_message = http_resp.error;
            return resp;
        }

        if let Ok(json) = serde_json::from_str::<Value>(&http_resp.body) {
            if let Some(msg) = json.get("message") {
                resp.text = msg["content"].as_str().unwrap_or("").into();
                if let Some(tcs) = msg["tool_calls"].as_array() {
                    for tc in tcs {
                        let args = match tc["function"]["arguments"].clone() {
                            Value::String(s) => {
                                serde_json::from_str(&s).unwrap_or(Value::String(s))
                            }
                            value => value,
                        };
                        resp.tool_calls.push(LlmToolCall {
                            id: tc["id"].as_str().map(|s| s.to_string()).unwrap_or_else(|| {
                                format!("call_ol_{}", &uuid::Uuid::new_v4().to_string()[..8])
                            }),
                            name: tc["function"]["name"].as_str().unwrap_or("").trim().into(),
                            args,
                        });
                    }
                }
            }
            if let Some(prompt_tokens) = json.get("prompt_eval_count").and_then(|v| v.as_i64()) {
                resp.prompt_tokens = prompt_tokens as i32;
            }
            if let Some(completion_tokens) = json.get("eval_count").and_then(|v| v.as_i64()) {
                resp.completion_tokens = completion_tokens as i32;
            }
            resp.total_tokens = resp.prompt_tokens + resp.completion_tokens;
            resp.success = true;
        }
        resp
    }

    fn get_name(&self) -> &str {
        "ollama"
    }

    fn cache_identity(&self) -> String {
        format!("ollama:model={}:endpoint={}", self.model, self.endpoint)
    }
}
