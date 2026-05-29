//! OpenAI-compatible LLM backend — uses serde_json + ureq.

#![allow(clippy::all)]

use super::backend::*;
use crate::infra::http_client;
use serde_json::{json, Value};

pub struct OpenAiBackend {
    api_key: String,
    model: String,
    endpoint: String,
    provider_name: String,
}

impl OpenAiBackend {
    pub fn new(provider: &str) -> Self {
        let (endpoint, model) = match provider {
            "xai" => ("https://api.x.ai/v1", "grok-3-mini-fast"),
            _ => ("https://api.openai.com/v1", "gpt-4o"),
        };
        OpenAiBackend {
            api_key: String::new(),
            model: model.into(),
            endpoint: endpoint.into(),
            provider_name: provider.into(),
        }
    }

    fn trimmed_text(text: &str) -> String {
        text.trim().to_string()
    }
}

#[async_trait::async_trait]
impl LlmBackend for OpenAiBackend {
    fn initialize(&mut self, config: &Value) -> bool {
        if let Some(k) = config["api_key"].as_str() {
            self.api_key = k.into();
        }
        if let Some(m) = config["model"].as_str() {
            self.model = m.into();
        }
        if let Some(e) = config["endpoint"].as_str() {
            self.endpoint = e.into();
        }
        !self.api_key.is_empty()
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
            let text = Self::trimmed_text(&msg.text);
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
                            "function": {"name": tc.name, "arguments": tc.args.to_string()}
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
        let mut req = json!({"model": self.model, "messages": msgs});
        if let Some(tokens) = max_tokens {
            req["max_tokens"] = json!(tokens);
        } else {
            req["max_tokens"] = json!(4096);
        }
        if !tools.is_empty() {
            let tool_arr: Vec<Value> = tools.iter().map(|t| json!({
                "type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.parameters}
            })).collect();
            req["tools"] = Value::Array(tool_arr);
        }

        let url = format!("{}/chat/completions", self.endpoint);
        let auth = format!("Bearer {}", self.api_key);
        let headers = [("Authorization", auth.as_str())];
        let http_resp = http_client::http_post(&url, &headers, &req.to_string(), 1, 60).await;

        let mut resp = LlmResponse::default();
        resp.http_status = http_resp.status_code;
        if !http_resp.success {
            resp.error_message = http_resp.error;
            return resp;
        }

        if let Ok(json) = serde_json::from_str::<Value>(&http_resp.body) {
            if let Some(msg) = json.pointer("/choices/0/message") {
                resp.text = msg["content"].as_str().unwrap_or("").into();
                if let Some(tcs) = msg["tool_calls"].as_array() {
                    for tc in tcs {
                        let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                        resp.tool_calls.push(LlmToolCall {
                            id: tc["id"].as_str().unwrap_or("").into(),
                            name: tc["function"]["name"].as_str().unwrap_or("").into(),
                            args: serde_json::from_str(args_str).unwrap_or(json!({})),
                        });
                    }
                }
            }
            if let Some(u) = json.get("usage") {
                resp.prompt_tokens = u["prompt_tokens"].as_i64().unwrap_or(0) as i32;
                resp.completion_tokens = u["completion_tokens"].as_i64().unwrap_or(0) as i32;
                resp.total_tokens = u["total_tokens"].as_i64().unwrap_or(0) as i32;
            }
            resp.success = true;
        }
        resp
    }

    fn get_name(&self) -> &str {
        &self.provider_name
    }
}
