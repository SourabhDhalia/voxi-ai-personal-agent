//! Fallback Parser — Extracts tool calls from plain text.
//!
//! Handles cases where the LLM fails to use the structured tool calling API
//! but produces valid tool call patterns in its response text.

use crate::llm::backend::LlmToolCall;
use regex::Regex;
use serde_json::{json, Value};

pub struct FallbackParser;

impl FallbackParser {
    fn call_id(prefix: &str) -> String {
        format!("{}_{}", prefix, &uuid::Uuid::new_v4().to_string()[..8])
    }

    fn args_from_value(value: Option<&Value>) -> Value {
        value.cloned().unwrap_or_else(|| json!({}))
    }

    fn push_tool_call(tool_calls: &mut Vec<LlmToolCall>, prefix: &str, name: &str, args: Value) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        tool_calls.push(LlmToolCall {
            id: Self::call_id(prefix),
            name: name.to_string(),
            args,
        });
    }

    fn parse_tool_value(value: &Value, tool_calls: &mut Vec<LlmToolCall>, prefix: &str) {
        if let Some(items) = value.as_array() {
            for item in items {
                Self::parse_tool_value(item, tool_calls, prefix);
            }
            return;
        }

        let Some(obj) = value.as_object() else {
            return;
        };

        if let Some(items) = obj.get("tool_calls").and_then(|v| v.as_array()) {
            for item in items {
                Self::parse_tool_value(item, tool_calls, prefix);
            }
            return;
        }

        if let Some(function) = obj.get("function") {
            if let Some(function_obj) = function.as_object() {
                if let Some(name) = function_obj.get("name").and_then(|v| v.as_str()) {
                    let args = function_obj
                        .get("arguments")
                        .or_else(|| function_obj.get("args"));
                    Self::push_tool_call(tool_calls, prefix, name, Self::args_from_value(args));
                    return;
                }
            } else if let Some(name) = function.as_str() {
                let args = obj
                    .get("arguments")
                    .or_else(|| obj.get("args"))
                    .or_else(|| obj.get("parameters"));
                Self::push_tool_call(tool_calls, prefix, name, Self::args_from_value(args));
                return;
            }
        }

        if let Some(name) = obj
            .get("tool")
            .or_else(|| obj.get("name"))
            .and_then(|v| v.as_str())
        {
            let args = obj
                .get("arguments")
                .or_else(|| obj.get("args"))
                .or_else(|| obj.get("parameters"));
            Self::push_tool_call(tool_calls, prefix, name, Self::args_from_value(args));
        }
    }

    fn parse_json_candidate(candidate: &str, tool_calls: &mut Vec<LlmToolCall>, prefix: &str) {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            return;
        }

        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            Self::parse_tool_value(&value, tool_calls, prefix);
        }
    }

    /// Parse tool calls from the given text.
    /// Supports patterns like:
    /// 1. <tool_call>name({"arg": "val"})</tool_call>
    /// 2. ```json {"tool": "name", "arguments": {...}} ```
    pub fn parse(text: &str) -> Vec<LlmToolCall> {
        let mut tool_calls = Vec::new();

        // 1. XML-style tag parser: <tool_call>name(json_args)</tool_call>
        let xml_re =
            Regex::new(r"(?s)<tool_call>\s*([A-Za-z0-9_.:-]+)\s*\((.*?)\)\s*</tool_call>").unwrap();
        for cap in xml_re.captures_iter(text) {
            let name = cap[1].to_string();
            let args_raw = &cap[2];
            let args: Value =
                serde_json::from_str(args_raw).unwrap_or_else(|_| Value::String(args_raw.into()));
            Self::push_tool_call(&mut tool_calls, "call_fb", &name, args);
        }

        // 1.5. Pure XML Model-Agnostic parser: <CallTool name="..." args="{...}" />
        let calltool_re = Regex::new(r#"(?s)<CallTool\s+name="([^"]+)"\s+args='([^']*)'\s*/>|<CallTool\s+name="([^"]+)"\s+args="([^"]*)"\s*/>"#).unwrap();
        for cap in calltool_re.captures_iter(text) {
            let (name, args_raw) = if let Some(n) = cap.get(1) {
                (
                    n.as_str().to_string(),
                    cap.get(2).map_or("", |m| m.as_str()),
                )
            } else {
                (
                    cap.get(3).unwrap().as_str().to_string(),
                    cap.get(4).map_or("", |m| m.as_str()),
                )
            };

            // Clean up escaped quotes if any
            let clean_args = args_raw.replace("\\\"", "\"");
            let args: Value =
                serde_json::from_str(&clean_args).unwrap_or_else(|_| Value::String(clean_args));
            Self::push_tool_call(&mut tool_calls, "call_xml", &name, args);
        }

        // 2. JSON block parser: ```json {"tool": "name", "arguments": {...}} ```
        if tool_calls.is_empty() {
            let json_re = Regex::new(r"(?s)```(?:json)?\s*(.*?)\s*```").unwrap();
            for cap in json_re.captures_iter(text) {
                Self::parse_json_candidate(&cap[1], &mut tool_calls, "call_fb_j");
            }
        }

        if tool_calls.is_empty() {
            Self::parse_json_candidate(text, &mut tool_calls, "call_fb_raw");
        }

        tool_calls
    }

    /// Extract <NewSummary>...</NewSummary> from the text for Fact-based Compaction
    pub fn extract_summary(text: &str) -> Option<String> {
        let re = Regex::new(r"(?s)<NewSummary>(.*?)</NewSummary>").unwrap();
        re.captures(text).map(|cap| cap[1].trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_style_parsing() {
        let text = "I will call the tool now: <tool_call>ls({\"path\": \"/tmp\"})</tool_call>";
        let calls = FallbackParser::parse(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "ls");
        assert_eq!(calls[0].args["path"], "/tmp");
    }

    #[test]
    fn test_json_block_parsing() {
        let text = "Use this: \n```json\n{\"tool\": \"read_file\", \"arguments\": {\"path\": \"test.txt\"}}\n```";
        let calls = FallbackParser::parse(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].args["path"], "test.txt");
    }

    #[test]
    fn test_plain_function_json_parsing() {
        let text = r#"{"function":{"name":"list_saved_addresses","arguments":{"city":"blr"}}}"#;
        let calls = FallbackParser::parse(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "list_saved_addresses");
        assert_eq!(calls[0].args["city"], "blr");
    }

    #[test]
    fn test_tool_calls_array_parsing() {
        let text = r#"{"tool_calls":[{"function":{"name":"mcp_swiggy-instamart_list_saved_addresses","arguments":"{\"limit\":2}"}}]}"#;
        let calls = FallbackParser::parse(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "mcp_swiggy-instamart_list_saved_addresses");
        assert_eq!(calls[0].args, Value::String("{\"limit\":2}".into()));
    }
}
