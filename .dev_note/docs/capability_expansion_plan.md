# Voxi Capability Expansion Plan

Planning-stage document. **No code changes** are made by this plan; it defines scope, gap analysis, and a staged roadmap for review before any implementation begins.

---

## Stage 0: Prerequisite Scenarios
Before any codebase modifications begin, the following integration scenarios will be written under `tests/system/`:
1. `tests/system/hooks_pre_tool_deny.json` — verify that a `pre_tool` hook configured with `deny` halts tool execution.
2. `tests/system/hooks_external_disabled_by_default.json` — verify that external shell hooks are rejected if `external_enabled` is false.
3. `tests/system/skills_list_and_invoke.json` — verify that external skills are parsed, listed, and run as tools.

---

## WS-A — Hooks Subsystem

### 1. Configuration (`data/config/hooks.json`)
Allows config updates via the dashboard config editor:
```json
{
  "external_enabled": false,
  "hooks_dir": ".voxi/hooks",
  "timeout_ms": 5000,
  "rules": [
    {
      "event": "pre_tool",
      "matcher": "run_command",
      "action": "ask"
    }
  ]
}
```
> [!IMPORTANT]
> **Dashboard Allowlist Sync**: The file `hooks.json` must be added to the `ALLOWED_CONFIGS` array inside [main.rs](file:///Users/sdhalia/Developer/githubRepo/shoppingAgent/src/voxi-web-dashboard/src/main.rs#L95) to allow read/write operations from the configuration editor UI.

### 2. Security for External Hooks
To prevent command injection and unauthorized filesystem access:
- **Path Resolution**: Resolve all script paths to absolute paths; reject any symbolic links resolving outside the configured `hooks_dir`.
- **Environment Allowlist**: Sanitize execution environment; pass a strict allowlist (e.g. `PATH`, `HOME`) and exclude local credentials.
- **Stdin Event Passing**: Avoid passing event parameters via command-line arguments (`argv`) to prevent shell injection. Instead, serialize the event payload as a JSON string and pipe it directly to the script's `stdin`.

### 3. Ask Decision Transport
Since client connections block the daemon's request thread during prompt execution:
- **Daemon-to-Client Notification**: When a tool requires approval (`ask`), `tool_dispatcher.rs` sends an `approval_request` notification containing an `approval_id` over the active streaming IPC connection.
- **Secondary Connection**: The client displays the confirmation prompt to the user. Once the user replies (allow/deny), the client opens a **new socket connection** to the daemon and calls `submit_approval { approval_id, allowed: bool }`.
- **Default-Deny Timeout**: The dispatcher thread blocks on a channel associated with the `approval_id` for up to `timeout_ms` (default 30s) and defaults to `deny` on timeout.

### 4. Code Structure Changes
- **`src/voxi/src/core/hooks.rs` [NEW]**: Logic to parse `hooks.json` and run Rust-in-process hooks or sandboxed external scripts (guarded by OS check — macOS and Linux only).
- **`src/voxi/src/core/tool_dispatcher.rs` [MODIFY]**: Wire pre/post hooks to wrap execution.
- **`src/voxi/src/core/tool_policy.rs` [MODIFY]**: Support policy mappings for hook evaluation.

---

## WS-B — Skills Section Formalization

- **Scanning**: Extends `src/voxi/src/core/textual_skill_scanner.rs` to load skill files from `~/.voxi/skills/` and repo-level `.agents/skills/`.
- **opt-in Skill Review Gate**: When `AgentCore` completes a loop successfully, it checks if a draft skill is constructible. It prompts the user via the active channel. If approved, it serializes a clean `SKILL.md` to disk.
- **Secrets Redaction**: The draft skill generator runs a redaction step that strips any substrings matching patterns for keys/tokens or references to loaded environment variables.
- **Endpoints**: Adds `/api/skills` for listing/enabling/disabling.
- **Persistence Layer**: Since `SKILL.md` files are read-only catalog entries, the enable/disable state is persisted in `skills_state.json` inside the config folder. The `skills_state.json` file must also be added to `ALLOWED_CONFIGS` in the web dashboard's `main.rs`.

---

## WS-F & WS-E — SSE Dashboard & CLI TUI

### 1. Dual-Process SSE Event Hop
Neither `web_dashboard` (on `9091`) nor `tv_channel` (on `9092`) are the processes where subagent traces or hooks actually execute; they both run as child processes spawned by the main `voxi` daemon. To route events correctly:
- **Daemon Event Broadcaster**: The `voxi` daemon maintains a centralized, in-memory event broadcast stream (e.g. `tokio::sync::broadcast::Sender`).
- **IPC Event subscription**: Add a new JSON-RPC IPC method `subscribe_events` to `ipc_server.rs`. When invoked, it keeps the socket connection open and writes every emitted event payload formatted as a newline-delimited JSON line to the socket.
- **Dashboard Axum SSE Forwarder**:
  - The dashboard Axum server exposes a query-token authenticated `/api/events` route.
  - When a browser connects to `/api/events`, Axum spawns a task that connects to the daemon's Unix socket, calls `subscribe_events`, and reads the stream.
  - It pushes each event payload directly to the browser client via an SSE Event Stream.

### 2. Token Auth
Requires query-based token verification (e.g. `/api/events?token=<auth-token>`) to protect the event stream.

### 3. Endpoints
Adds `/api/agents` to retrieve live subagent traces.

---

## WS-G — UI/UX Dashboard Improvements (Premium Enhancements)
Applying the **ui-ux-pro-max** skill guidelines to elevate the visual and interactive design of the dashboard:

### 1. Premium Visual Theme
- **Color Palette**: Desaturated sleek dark mode. Background: `#0e0e12` (deep charcoal), surface cards: `#161622` (mid-gray), borders: `#ffffff0f` (fine translucent separators), details/accents: `#6366f1` (indigo) and `#14b8a6` (teal).
- **Typography**: Paired fonts. Headings and interactive labels use `Inter` (sans-serif) for clean modern readability, while raw output, code blocks, and event traces use `JetBrains Mono` for a technical, developer-centric feel.
- **Micro-Animations**: All button presses use a scale transition (`transform 150ms cubic-bezier(0.4, 0, 0.2, 1)`) with `scale(0.97)` on click and `scale(1.02)` on hover.

### 2. Live Interactive Hook Approval Modal
- When a `pre_tool` hook blocks on an `ask` action:
  - An overlay modal pops up with a blur background (`backdrop-filter: blur(8px)`) and a subtle dark scrim.
  - The modal displays the blocked tool name, the command/parameters, and a highlighted danger badge if it involves system execution (e.g., `run_command`).
  - Shows a visible countdown ring representing the timeout period (e.g. 30 seconds) that runs down in real-time.
  - Action buttons: A primary CTA "Approve Action" in green-teal, and a secondary "Deny Action" in semantic red.

### 3. Subagent & Hook Live Execution Timeline
- Inside the Chat/Sessions view, add a collapsible side panel named "Execution Logs & Traces".
- Uses a vertical line timeline with nodes that light up based on SSE events:
  - **Gray Node**: Pending.
  - **Pulse Teal Node**: Currently executing (thinking / tool calling / hook running).
  - **Green Node**: Success.
  - **Red Node**: Failure/Denial.
- Displays nested subagent steps (e.g., `Parent Agent -> spawn: research subagent -> hook: pre_tool -> tool: view_file -> success`).

### 4. Interactive Skills Catalog
- A new page tab "Skills Console" that reads/writes via `/api/skills`:
  - Displays loaded skills in a bento-grid layout with cards representing each skill (name, description, path).
  - A custom slider toggle switch on each card to enable or disable the skill (updating `skills_state.json`).
  - An "Auto-Generated Drafts" queue at the top of the grid. When a draft skill is generated:
    - Renders a draft review card showing the markdown diff side-by-side.
    - Provides button CTAs: "Approve & Save" (writes to target skill directory) and "Discard Draft".

### 5. Loading Skeletons
- Replace generic "Loading..." text blocks with animated CSS skeleton shimmer cards (`@keyframes shimmer` overlaying a gray gradient sweep) to reduce perceived wait times.

---

## WS-C & WS-D (Deferred Items)
For the current iteration, the following features are deferred:
- Subagent isolation flag (WS-C)
- Provider fallback chain (WS-D)
- Scheduled automations/cron manager (WS-D)

---

## Commit & Deployment Policy
- No local `cargo build` or `cargo check` commands.
- Build and run validation only via `./deploy.sh` for host/target targets.
- Stage-gate transitions recorded on `.dev_note/DASHBOARD.md`.
- Commits must use `.tmp/commit_msg.txt` with `git commit -F`.
