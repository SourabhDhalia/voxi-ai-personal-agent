# Autonomous Shopping Agent Integration via Multiple MCPs

This implementation plan details how to connect the TizenClaw daemon to multiple external Model Context Protocol (MCP) servers, expose their tools to the LLM, route tool execution dynamically, and support multi-turn shopping conversations with user clarification.

## User Review Required

> [!IMPORTANT]
> **Ollama Tool-Calling Support**: The current `OllamaBackend` ignores tool declarations. We propose upgrading the backend to support Ollama's native tool-calling JSON format. This requires an Ollama model that natively supports function calling (e.g., Qwen 2.5, Gemma 4, Llama 3.1).
> If a model does not support native function calling, the daemon will fall back to extracting XML/JSON tool calls from the plain text response via `FallbackParser`.

> [!WARNING]
> **Safety Policies for Purchases**: Shopping tools (like checking out or payments) have real-world side effects. These must be registered under `ToolPolicy` as **High-Risk** or require explicit user confirmation via an interactive Slack/Telegram/Dashboard bridge before execution.

## Open Questions

> [!IMPORTANT]
> - **Shopping MCP Servers**: Which specific shopping MCP servers do you plan to use? (e.g., custom SQLite/Postgres database server, web search/scraping MCP, Amazon/eBay APIs, or a custom retail ERP client).
> - **Authentication/Secrets**: Do any of your target MCP servers require environment variables or API keys (e.g., Amazon API keys)? If so, they must be safely injected into the server process command arguments or environment during spawning.

---

## Proposed Changes

### Core Daemon & MCP Integration

#### [MODIFY] [agent_core.rs](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/src/core/agent_core.rs)
- Add `mcp_client_manager: tokio::sync::RwLock<McpClientManager>` field to `AgentCore` to hold active connections to all shopping and utility MCP servers.
- In `AgentCore::initialize`, load `mcp_servers.json` (from `platform.paths.config_dir`) and call `load_config_and_connect`.
- In `AgentCore::get_bridge_tool_declarations`, query `self.mcp_client_manager.read().await.get_all_tools()` and merge the returned `LlmToolDecl` list.
- In `AgentCore::execute_tool`, check if `tool_name` starts with `mcp_`. If so, route the execution:
  ```rust
  if tool_name.starts_with("mcp_") {
      let mut mcp = self.mcp_client_manager.write().await;
      match mcp.call_tool(tool_name, args) {
          Some(res) => res,
          None => serde_json::json!({"error": format!("MCP server for tool {} not found or disconnected", tool_name)}),
      }
  }
  ```
- Similarly, in the chat loop observation collector, handle asynchronous `mcp_` execution.

#### [MODIFY] [mcp_client.rs](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/src/channel/mcp_client.rs)
- Ensure that `McpClientManager` correctly propagates environment variables to spawned child processes if they are specified in `mcp_servers.json` (e.g., adding an optional `"env": {"API_KEY": "..."}` mapping to the server config).

---

### LLM Backend & Tool-Calling Support

#### [MODIFY] [ollama.rs](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/src/llm/ollama.rs)
- Update the `/api/chat` request payload to include `tools` when present.
- Parse the `/api/chat` response to extract Ollama's native `tool_calls` structure:
  ```json
  "message": {
    "role": "assistant",
    "content": "",
    "tool_calls": [
      {
        "function": {
          "name": "mcp_sqlite_query",
          "arguments": { "query": "..." }
        }
      }
    ]
  }
  ```
- Map these to `LlmResponse::tool_calls`.

---

### Multi-turn Conversation & Clarifications

#### [NEW] [user_clarification.md](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/src/core/user_clarification.rs)
- Implement a built-in `request_user_clarification` tool:
  - **Parameters**: `question` (string)
  - **Effect**: Publishes the question to the current session's active channel (web dashboard, Slack, Discord, Telegram) and pauses the agent loop by transitioning to a new state `AgentPhase::AwaitingUserInput`.
  - **Resumption**: Once the user replies in the channel, the reply text is captured, mapped as the tool result for `request_user_clarification`, and the loop is resumed.

---

## Verification Plan

### Automated Tests
- Since local tests are prohibited by `RULE[AGENTS.md]`, all verification must be run on the QEMU/Tizen target using `./deploy.sh`.
- Deploy the updated daemon:
  ```bash
  ./deploy.sh
  ```
- Run integration tests validating standard MCP handshakes and execution:
  ```bash
  ./deploy.sh --test
  ```

### Manual Verification
1. Place a mock `mcp_servers.json` in the config directory.
2. Start the daemon and check startup logs to verify connection and successful handshake.
3. Start a chat session using `tizenclaw-cli` or Telegram.
4. Input a shopping request like: *"Check the shopping list database for milk and let me know if it's there."*
5. Verify that the agent uses the `mcp_sqlite_query` tool, queries `/tmp/sqlite.db`, retrieves the item, and returns it to the chat.
