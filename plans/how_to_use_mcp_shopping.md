# How to Use Voxi as an Autonomous Shopping Agent

This guide explains how to configure, run, and test Voxi with multiple shopping MCP servers (Zepto and Swiggy) and handle back-and-forth user clarifications.

---

## 1. Configure MCP Servers
The MCP server configurations are loaded from `data/config/mcp_servers.json`.

Ensure your `mcp_servers.json` contains the following structure:
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

---

## 2. Deploy and Run the Daemon
Since you are testing on a target machine, run:
```bash
# Clean, compile, and deploy to Voxi (emulator/device)
./deploy.sh
```
When the daemon boots up, it will check the `mcp_servers.json` file, auto-wrap the HTTP servers in `npx mcp-remote` processes, perform initialize handshakes, and discover all shopping tools.

Verify in the logs (`voxi.log`) that the connection is successful:
```
[INFO] MCP Client Manager loaded configuration and connected to servers
[DEBUG] MCP Client: 'zepto' connected (4 tools)
[DEBUG] MCP Client: 'swiggy-instamart' connected (3 tools)
```

---

## 3. Choose the Right LLM Model
Ensure your active model in `llm_config.json` supports native function calling:
- **Recommended Local Models**: `gemma4:9b`, `qwen2.5:7b`, or `qwen2.5-coder:7b`.
- Run Ollama locally on the target machine and ensure the model is pulled:
```bash
ollama pull qwen2.5:7b
```

---

## 4. Testing Shopping Workflows

### Scenario A: Groceries Search (Zepto)
1. Open the Web Dashboard or send a message to the daemon:
   > *"I want to search for 1L whole milk and 12 organic eggs on Zepto."*
2. The agent will format a tool call: `mcp_zepto_search_products({"query": "whole milk 1L"})`.
3. The daemon routes this to the spawned Zepto client, executes it, and feeds the results back to the LLM.

### Scenario B: Multi-turn Clarification
1. Send a complex request:
   > *"I want to cook Paneer Tikka tonight. Order the ingredients from Instamart."*
2. The agent searches for paneer, bell peppers, and spices. It finds multiple options (e.g., 200g vs 500g paneer pack).
3. The LLM invokes the built-in `request_user_clarification` tool:
   ```json
   {
     "name": "request_user_clarification",
     "arguments": {
       "question": "I found Amul Paneer 200g (₹85) and Mother Dairy Paneer 500g (₹210). Which brand/size would you like to add?"
     }
   }
   ```
4. The loop pauses and immediately returns this question to the Web Dashboard.
5. You reply:
   > *"Get the Amul Paneer 200g."*
6. The agent registers this reply, resumes the loop, adds the specific pack to your cart, and summarizes the final order details.
