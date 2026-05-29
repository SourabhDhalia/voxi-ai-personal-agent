# Stop Request and Chat Response Routing Fix

## Summary
- Add a stable `request_id` for every chat/prompt run, separate from `session_id`.
- Stop only the specific in-flight request from the same chat/session.
- Prevent race-condition response mixing by routing every final response, stream chunk, outbound update, Telegram reply, and A2A artifact by `session_id + request_id`.
- Serialize execution per `session_id` so two requests in the same chat cannot corrupt transcript order, while different chats/users can still run concurrently.

## Public APIs and Types
- Add daemon IPC support:
  - `prompt` accepts optional `params.request_id`.
  - `prompt` returns `{ text, session_id, request_id, status }`.
  - New JSON-RPC method `cancel_request` with `{ request_id, session_id }`.
  - Streaming chunks include `{ id, session_id, request_id, chunk }`.
- Add Rust client APIs without breaking existing `process_prompt`:
  - `PromptRunResponse { session_id, request_id, text, status, stream_received }`
  - `CancelRequestResponse { request_id, session_id, status, message }`
  - `process_prompt_with_request(...)`
  - `process_prompt_streaming_with_request(...)`
  - `cancel_request(session_id, request_id)`
- Add web APIs:
  - `POST /api/chat` accepts/returns `request_id`.
  - `POST /api/chat/stop` cancels `{ session_id, request_id }`.
  - `/api/outbound/messages` supports optional `session_id` and `request_id` filters.

## Implementation Changes
- In `AgentCore`, add an active request registry keyed by `request_id`:
  - Store `session_id`, `request_id`, start time, cancellation flag, and abort handle.
  - Reject duplicate active request ids.
  - Cancel only when the caller’s `session_id` matches the active request.
  - Remove entries on completion, failure, or cancellation.
- Move the current prompt loop into a tracked internal execution path:
  - Existing `process_prompt(...) -> String` remains as a compatibility wrapper.
  - New tracked path returns structured status: `completed`, `cancelled`, or `error`.
  - Check cancellation before context load, before/after LLM calls, before tool dispatch, between tool rounds, and before final transcript writes.
  - On cancellation, stop further tool/LLM work and record a bounded assistant message like `Request stopped by user.` in that same session only.
- Add per-session async locking:
  - One active execution at a time per `session_id`.
  - Requests from different chat ids/sessions remain concurrent.
  - If a queued request is cancelled before acquiring the lock, it exits without mutating chat history.
- Web dashboard:
  - Generate `session_id` client-side for a new chat before sending the first prompt.
  - Generate `request_id` client-side for each send.
  - Keep a pending map `{ request_id, session_id, placeholder_element }`.
  - Add a Stop button to each pending assistant placeholder.
  - Never append a completed response to the currently visible chat unless its `session_id` matches; otherwise only refresh the session list.
  - Replace the matching placeholder by `request_id`, not by arrival order.
- Telegram:
  - Track the active request id per `chat_id`.
  - Add `/stop` command to cancel only that chat’s current request.
  - Chat mode calls the tracked AgentCore path with `request_id`.
  - Coding mode gets a cancellation flag around its child process; `/stop` kills only that chat’s active CLI child and reports `Request stopped.`
- A2A:
  - Use `task_id` as `request_id`.
  - `tasks/cancel` calls AgentCore cancellation for the matching `a2a_<task_id>` session.
  - The spawned task must not overwrite a cancelled task with `Completed` after cancellation wins the race.
- Outbound messages:
  - Add `request_id` to dashboard outbound JSONL records.
  - `send_outbound_message` supplies current request id by default.
  - Dashboard polling filters/labels updates by originating `session_id + request_id`.

## Test Plan
- Add tests for:
  - IPC prompt response echoes the supplied `request_id`.
  - Auto-generated request ids are unique and included in responses.
  - `cancel_request` cancels matching `session_id + request_id`.
  - Cancellation with the wrong session id is rejected.
  - A2A cancelled tasks are not later overwritten as completed.
  - Dashboard response handling updates the matching placeholder, not the active chat by arrival time.
  - Telegram `/stop` only cancels the active request for that `chat_id`.
- Do not run local `cargo build`, `cargo check`, `cargo test`, or `cargo clippy`.
- Validation commands for the target machines:
  - Ubuntu x86_64: `./deploy_host.sh --test`
  - VoxiOS armv7l: `./deploy.sh`
- Manual verification:
  - Send two web chat prompts rapidly in different sessions; each answer appears only in its originating chat.
  - Send two prompts in the same chat; transcript order remains deterministic.
  - Stop a slow Ollama request; UI shows stopped state and no late answer appears.
  - Cancel A2A task; final status remains `cancelled`.
  - Telegram `/stop` from one chat does not affect another chat.

## Assumptions
- “Stop request” means cancel one specific in-flight request, not the whole daemon or all requests in a chat.
- `session_id` remains the durable chat id; `request_id` is per prompt/run.
- Same-session requests should be serialized for correctness on lower/local LLM models.
- Existing simple `process_prompt` APIs stay available for compatibility, while new tracked APIs expose request ids and cancellation.
