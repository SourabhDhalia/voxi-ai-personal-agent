# Implementation Plan - Stop Request & Zepto MCP Routing Fix

This plan details the implementation of a stable `request_id` tracking system, request serialization per session, stop request endpoints (JSON-RPC, Web, Telegram, and A2A), and a hardened Zepto MCP flow with protocol validation.

## User Review Required

> [!IMPORTANT]
> **WSL Execution Environment**: All compilation and deployment validation will run using host-side dry-run checks and the existing `./deploy_host.sh` scripts.
> Local `cargo build` or `cargo test` is prohibited on the macOS host per `AGENTS.md`.

## Open Questions
None. The design is fully specified by `.dev_note/plan1`, `.dev_note/plan2`, and `.dev_note/plan3`.

## Proposed Changes

---

### Component: Daemon Core & API

#### [MODIFY] [agent_core.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/agent_core.rs)
- Add a thread-safe registry of active requests: `active_requests: Arc<Mutex<HashMap<String, RequestState>>>`.
  - `RequestState` holds `session_id`, `request_id`, start time, `cancelled: Arc<AtomicBool>`, and optional abort handle.
- Modify `process_prompt` to accept an optional `request_id`. If not provided, generate a unique one.
- Implement per-session serialization:
  - Add `session_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>` to ensure only one prompt execution runs concurrently per `session_id`.
  - Prior to acquiring the session lock, check if the request has already been cancelled.
- Inject cancellation check gates before context loading, before/after LLM backend requests, before tool dispatch, between tool rounds, and before writing transcripts.
- If cancelled, terminate early, record a bounded message (`Request stopped by user.`) in the session context, and cleanup the active requests registry.
- Add `cancel_request(session_id, request_id)` to cancel matching request.
- Suppress ChatGPT widget tools (specifically `mcp_zepto_zepto_shop`) in local Ollama tool declarations.
- Enforce the canonical Zepto usage sequence (user details -> list addresses -> select address -> past order items -> search products) for grocery/food queries, returning a corrective prompt if address/store selection is skipped.

#### [MODIFY] [ipc_server.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/core/ipc_server.rs)
- Add `cancel_request` method to JSON-RPC dispatcher.
- Modify `prompt` JSON-RPC method to accept `request_id` and return `request_id`.
- Update streaming chunks to pass `request_id`.

#### [MODIFY] [api.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/libvoxi/src/api.rs)
- Add `process_prompt_with_request` and `process_prompt_streaming_with_request`.
- Add `cancel_request` call wrapping the IPC client protocol.

---

### Component: MCP Connection & Protocol Validation

#### [MODIFY] [mcp_client.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/channel/mcp_client.rs)
- Harden the HTTP client connection payload to include `capabilities: {"tools": {}}` in the initialization payload.
- Send the `notifications/initialized` with `"params": {}`.
- Support alias mapping correctly (e.g. `search_products` resolves to `mcp_zepto_search_products` when only Zepto offers it).
- Expose diagnostic checks for `/mcp status <server>`, `/mcp tools <server>`, and `/mcp test <server>`.

---

### Component: Web Dashboard & Channel

#### [MODIFY] [main.rs (Web Dashboard)](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi-web-dashboard/src/main.rs)
- Update `/api/chat` POST endpoint to accept/return `request_id`.
- Add `/api/chat/stop` POST endpoint to accept `{ session_id, request_id }` and call IPC `cancel_request`.
- Update `/api/outbound/messages` to allow optional `session_id` and `request_id` filters.

#### [MODIFY] [app.js](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/data/web/app.js)
- Generate a unique client-side `request_id` for every prompt run.
- Keep track of pending placeholders and render a "Stop" button on them.
- Wire the Stop button to call `POST /api/chat/stop` with `{ session_id, request_id }`.
- Ensure completed responses only update matching active chat placeholder, never crossing sessions.

---

### Component: Telegram Channel & A2A Handler

#### [MODIFY] [telegram_client.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/channel/telegram_client.rs)
- Track active `request_id` per Telegram `chat_id`.
- Support `/stop` command:
  - In Chat Mode, call `AgentCore::cancel_request` for that chat's active `request_id`.
  - In Coding Mode, kill the active CLI child process and report `Request stopped.`.

#### [MODIFY] [a2a_handler.rs](file:///Users/sdhalia/Developer/githubRepo/voxi-rust/src/voxi/src/channel/a2a_handler.rs)
- Use `task_id` as the `request_id`.
- Map cancel task request to cancel matching `a2a_<task_id>` session.

---

## Verification Plan

### Automated Tests
- Run `deploy_host.sh --test` to compile and run existing/new unit tests.

### Manual Verification
1. Run daemon and Web dashboard. Send overlapping prompts in separate chats, verify no crosstalk.
2. Send a prompt, click Stop button on the UI, and verify the model execution terminates cleanly.
3. Run `/mcp status zepto` and check tool outputs.
4. Try Telegram `/stop` command and check response.
