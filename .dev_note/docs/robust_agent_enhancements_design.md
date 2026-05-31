# Design: Robust Agent Workspace Enhancements

This design details the Rust architecture and implementation approach for adding loop guards, discovery filesystem tools, environment diagnostics (`/doctor`), and the multi-turn interactive REPL console loop.

## 1. Loop Guard and Recovery

### Objective
Prevent AI agents from executing infinite loops (e.g. calling the same failed command or editing the same file repeatedly) without user intervention.

### Design
Inside `vclaw-runtime/src/conversation/engine.rs`, the main execution loop streams the assistant response and executes any returned tool calls. We will maintain loop state across turns:

```rust
struct LoopGuard {
    consecutive_identical_count: usize,
    last_tool_calls: Option<Vec<ToolCallRequest>>,
}
```

For each step in the turn:
1. Compare the requested list of `tool_calls` with `last_tool_calls`.
2. If they are identical (same tool name and arguments/input):
   - Increment `consecutive_identical_count`.
3. If not, reset `consecutive_identical_count` to 1 and update `last_tool_calls`.
4. Trigger transitions based on `consecutive_identical_count`:
   - **Count = 3**: Inject a `System` role message into `session.messages` describing the loop and prompting the model to pivot.
   - **Count >= 4**: Return a `ConversationRuntimeError::Invariant` aborting the run with a descriptive loop message.

---

## 2. Filesystem Discovery Tools

### Objective
Allow the agent to perform directory listing and pattern matching via dedicated low-risk tools rather than invoking raw shell commands.

### Tools Design
We will add two new tool definitions inside `vclaw-tools/src/builtins.rs`:

#### `fs.list_directory`
- Input Schema:
  ```json
  {
    "type": "object",
    "required": ["path"],
    "properties": {
      "path": { "type": "string" }
    }
  }
  ```
- Implementation: Use `std::fs::read_dir` to read files/directories, returning a list of entry objects:
  ```json
  {
    "entries": [
      {
        "name": "src",
        "is_directory": true,
        "is_file": false,
        "size": 4096
      }
    ]
  }
  ```
- Risk level: `Low`.

#### `fs.glob`
- Input Schema:
  ```json
  {
    "type": "object",
    "required": ["pattern"],
    "properties": {
      "pattern": { "type": "string" }
    }
  }
  ```
- Implementation: Use the `glob` crate to match files recursively and return a list of matching absolute/relative paths.
- Risk level: `Low`.

---

## 3. Environment Doctor (`/doctor`)

### Objective
Validate setup configuration, tool dependencies in the `PATH`, environment secret keys, and configured MCP servers.

### Interface & Structure
Inside `vclaw-runtime/src/doctor.rs`, we define the diagnostic data structures:
- `PathCheckResult`: Checks if `config.root_dir`, `session_dir`, `plugin_dir`, and `log_dir` are readable/writable.
- `ToolCheckResult`: Audits if `git`, `curl`, and `python3` can be invoked in the current shell execution path.
- `EnvCheckResult`: Audits presence of required API keys (e.g. `GEMINI_API_KEY`).
- `McpCheckResult`: Verifies that configuration properties of configured MCP servers are loaded.

```rust
pub fn run_diagnostics(config: &RuntimeConfig) -> DoctorSummary { ... }
```

---

## 4. Interactive REPL Loop

### Objective
Provide a multi-turn terminal command line interface (REPL) when running `rusty-claude-cli` without arguments.

### Logic Flow
1. Check if the input session is interactive (terminal mode and no piped stdin).
2. If interactive mode is triggered, start a read-eval-print-loop:
   - Print prompt `vclaw > `.
   - Read line from standard input.
   - Parse command or slash command (e.g. `/doctor`, `/exit`, or plain text).
   - Dispatch the input to the conversation engine or command registry.
   - Render the response.
   - Loop until exit signal.
