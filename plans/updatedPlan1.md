# Autonomous Shopping Agent Integration via Multiple MCPs

This implementation plan details how to connect the Voxi daemon to multiple external Model Context Protocol (MCP) servers, expose their tools to the LLM, route tool execution dynamically, and support multi-turn shopping conversations with user clarification.

## User Review Required

> [!IMPORTANT]
> **Ollama Tool-Calling Support**: The current `OllamaBackend` ignores tool declarations. We propose upgrading the backend to support Ollama's native tool-calling JSON format. This requires an Ollama model that natively supports function calling (e.g., Qwen 2.5, Gemma 4, Llama 3.1).
> If a model does not support native function calling, the daemon will fall back to extracting XML/JSON tool calls from the plain text response via `FallbackParser`.

> [!WARNING]
> **Safety Policies for Purchases**: Shopping tools (like checking out or payments) have real-world side effects. These must be registered under `ToolPolicy` as **High-Risk** or require explicit user confirmation via an interactive Slack/Telegram/Dashboard bridge before execution.

## Confirmed Design Decisions

> [!NOTE]
> **Shopping MCP Servers**: We will configure both **Zepto** and **Swiggy** (Instamart, Food, Dineout) MCP servers.
> - **Zepto** is configured using `"command": "npx"` with `"args": ["-y", "mcp-remote", "https://mcp.zepto.co.in/mcp"]`.
> - **Swiggy** is configured as `"type": "http"`. The `McpClientManager` parser will dynamically translate `"type": "http"` entries into spawned `npx -y mcp-remote <url>` commands.
> - This allows us to use standard `stdio` transport for all MCP integrations without rewriting a custom async HTTP/SSE client in Rust.

> [!NOTE]
> **User Interaction Channel**: We will use the **Web Dashboard** as the active channel for now (and hook it into the custom DTV TV popup UI channel later).

---

## Proposed Changes

### Core Daemon & MCP Integration

#### [MODIFY] [mcp_servers.json](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/data/config/mcp_servers.json)
- Add Zepto and Swiggy configurations:
  ```json
  {
    "mcpServers": {
      "sqlite": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-sqlite", "/tmp/sqlite.db"]
      },
      "zepto": {
        "command": "npx",
        "args": ["-y", "mcp-remote", "https://mcp.zepto.co.in/mcp"]
      },
      "swiggy-instamart": {
        "type": "http",
        "url": "https://mcp.swiggy.com/im"
      },
      "swiggy-food": {
        "type": "http",
        "url": "https://mcp.swiggy.com/food"
      },
      "swiggy-dineout": {
        "type": "http",
        "url": "https://mcp.swiggy.com/dineout"
      }
    }
  }
  ```

#### [MODIFY] [mcp_client.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/channel/mcp_client.rs)
- Update `McpClientManager::load_config_and_connect` to parse the standard map-based `mcpServers` configuration format.
- If a server entry specifies `"type": "http"`, map it to `command = "npx"` and `args = ["-y", "mcp-remote", url]`.
- Clean up any unused legacy code expecting the `"servers"` array structure.

#### [MODIFY] [agent_core.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/agent_core.rs)
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


---

### LLM Backend & Tool-Calling Support

#### [MODIFY] [ollama.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/llm/ollama.rs)
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

#### [NEW] [user_clarification.md](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/user_clarification.rs)
- Implement a built-in `request_user_clarification` tool:
  - **Parameters**: `question` (string)
  - **Effect**: Publishes the question to the current session's active channel (web dashboard, Slack, Discord, Telegram) and pauses the agent loop by transitioning to a new state `AgentPhase::AwaitingUserInput`.
  - **Resumption**: Once the user replies in the channel, the reply text is captured, mapped as the tool result for `request_user_clarification`, and the loop is resumed.

---

## Verification Plan

### Automated Tests
- Since local tests are prohibited by `RULE[AGENTS.md]`, all verification must be run on the QEMU/Voxi target using `./deploy.sh`.
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
3. Start a chat session using `voxi-cli` or Telegram.
4. Input a shopping request like: *"Check the shopping list database for milk and let me know if it's there."*
5. Verify that the agent uses the `mcp_sqlite_query` tool, queries `/tmp/sqlite.db`, retrieves the item, and returns it to the chat.
