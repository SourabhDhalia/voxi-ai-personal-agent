# Code Changes Explanation: Multi-MCP Integration & User Clarification

This document details the modifications made to the Voxi codebase to enable the daemon to run as an autonomous shopping assistant using multiple external MCP servers.

---

## 1. Upgrade to the MCP Client Manager Config Parser
**Modified File**: [mcp_client.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/channel/mcp_client.rs)

### Problem
- The legacy `load_config_and_connect` expected a legacy `{"servers": [...]}` array layout, whereas standard configs (`mcp_servers.json`) use the map-based `{"mcpServers": { ... }}` layout.
- The client only supported stdio-based processes and could not connect to HTTP-based MCP servers (like Swiggy).

### Solution
- Upgraded the parser to support the standard `mcpServers` map format with a backward-compatible fallback to the legacy `servers` array.
- Added transparent support for `"type": "http"` servers. When the parser encounters an HTTP server (e.g. `https://mcp.swiggy.com/im`), it automatically wraps it in an `npx -y mcp-remote <url>` command:
```rust
let mcp_type = s["type"].as_str().unwrap_or("stdio");
if mcp_type == "http" {
    if let Some(url) = s["url"].as_str() {
        command = "npx".to_string();
        args = vec!["-y", "mcp-remote", url].into_iter().map(String::from).collect();
    }
}
```
This enables stdio-based `McpClient` to interact with HTTP/SSE servers seamlessly.

---

## 2. Integrating MCP Clients into AgentCore
**Modified File**: [agent_core.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/agent_core.rs)

### Add Field & Initialization
- Added `mcp_client_manager: tokio::sync::RwLock<McpClientManager>` to `AgentCore`.
- Instantiated it in `AgentCore::new()` and called `load_config_and_connect` in `AgentCore::initialize()`.

### Indexing Tool Declarations
- Modified `AgentCore::get_bridge_tool_declarations` to dynamically query and append all discovered remote MCP tools:
```rust
{
    let mcp = self.mcp_client_manager.read().await;
    tools.extend(mcp.get_all_tools());
}
```
- In the session chat loop, filtered and injected relevant MCP tools into the LLM context based on matching intent keywords (e.g., query for "zepto" or "swiggy").

### Execution Routing
- Added routing logic for tool calls starting with `mcp_` inside `AgentCore::execute_tool` and the session chat loop's asynchronous tool executor futures, delegating calls directly to `mcp_client_manager.call_tool()`.

---

## 3. Upgrading Ollama Backend for Tool-Calling
**Modified File**: [ollama.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/llm/ollama.rs)

### Problem
- The local Ollama backend completely ignored the `tools` slice, making it impossible for local models to native-call shopping tools.

### Solution
- Upgraded the `/api/chat` request schema to pass function declarations to Ollama.
- Formatted historical chat logs in the request using correct `tool` and `assistant` payload schemas (matching the OpenAI implementation in `openai.rs`).
- Parsed native `tool_calls` out of Ollama's response object, handling both stringified and object-structured function arguments:
```rust
let args = match tc["function"]["arguments"].clone() {
    Value::String(s) => serde_json::from_str(&s).unwrap_or(json!({})),
    obj @ Value::Object(_) => obj,
    _ => json!({}),
};
```

---

## 4. Multi-turn User Clarification Tool
**Modified Files**: [tool_declaration_builder.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/tool_declaration_builder.rs), [agent_core.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/agent_core.rs)

### Solution
- Registered `request_user_clarification` as a builtin meta-tool.
- When called, the tool posts the question payload to the `web_dashboard` channel.
- Added a parser check right after parallel tools execute in `process_prompt`: if `request_user_clarification` was triggered, the daemon logs the question to history, transitions the loop phase to `Complete`, and **abruptly returns the question** to pause execution until the next user turn.
