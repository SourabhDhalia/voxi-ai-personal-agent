# Design Architecture - Stop Request & Zepto MCP Routing Fix

This document outlines the detailed architectural design for introducing `request_id` tracking, serialization of session prompt executions, request cancellation handling, and Zepto MCP hardening.

## 1. Thread-Safe Request Registry & Session Serialization

### Active Request Registry
We will store in-flight request states in `AgentCore` inside a synchronized map:
```rust
pub struct RequestState {
    pub session_id: String,
    pub request_id: String,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
}
```
In `AgentCore` struct:
```rust
active_requests: Arc<std::sync::Mutex<HashMap<String, RequestState>>>
```

### Per-Session Serialization
To prevent multiple requests in the same session from corrupting the chat history or executing concurrently (which can cause out-of-order execution on local models), prompt execution will be serialized using a per-session lock map:
```rust
session_locks: Arc<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>
```
For any incoming prompt:
1. Retrieve or create the `Arc<tokio::sync::Mutex<()>>` lock for that `session_id`.
2. Acquire the lock asynchronously.
3. Once the lock is acquired, register the request state in `active_requests` and check if it has already been cancelled.
4. Execute the loop, releasing the lock upon completion or early cancellation termination.

## 2. Cancellation Checkpoints
We will perform cancellation checks using a helper method that returns a cancellation error text if `cancelled` is true:
```rust
fn check_cancelled(&self, request_id: &str) -> bool
```
Cancellation gates will be checked:
- At the start of `process_prompt`.
- Before loading context.
- Before calling `chat_with_fallback`.
- After calling `chat_with_fallback`.
- Before and after tool dispatch.
- Between tool rounds.
- Before final transcript write to disk.

On cancellation, a message `Request stopped by user.` is recorded in the session store and returned to the caller.

## 3. IPC and Web Dashboard API Cancel Command

### JSON-RPC Commands
- **prompt**: Accepts optional `request_id` parameters and returns `request_id` in result.
- **cancel_request**: Accepts `{ session_id, request_id }` and returns `{ "status": "ok" }`.
- **stream_chunk**: Includes `request_id` in chunk event parameters.

### Web Dashboard Endpoints
- `POST /api/chat`: accepts `request_id`, generates if empty, forwards to IPC, returns `request_id`.
- `POST /api/chat/stop`: accepts `{ session_id, request_id }` and executes `cancel_request` via IPC.
- `GET /api/outbound/messages`: supports optional `session_id` and `request_id` query parameters.

## 4. Zepto MCP Protocol Hardening & Flow Enforcer

### Handshake Payload Matching
Ensure connection payload matches exactly:
- `initialize` payload: `params.capabilities = {"tools": {}}` instead of `{}`.
- `notifications/initialized` payload: `"params": {}`.

### Suppress Widget Tools
In local Ollama / Daemon execution mode, filter out `mcp_zepto_zepto_shop` tool declaration to prevent Ollama from selecting it.

### Routing Alias & Ambiguity Guard
Before falling back to local tools, resolve names using connected MCP servers.
- Match `search_products` to `mcp_zepto_search_products` when only Zepto registers it.
- If multiple servers register the same alias, fail closed, list the options, and ask for confirmation.

### Canonical Zepto Sequential Flow
Intercept prompts requesting Zepto search/order:
- Enforce calling sequence:
  1. `mcp_zepto_get_user_details`
  2. `mcp_zepto_list_saved_addresses`
  3. `mcp_zepto_select_saved_address`
  4. `mcp_zepto_get_past_order_items`
  5. `mcp_zepto_search_products`
- If a tool is executed out of sequence (e.g. search before address selection), return a corrective result telling the model to run `mcp_zepto_list_saved_addresses` first.
