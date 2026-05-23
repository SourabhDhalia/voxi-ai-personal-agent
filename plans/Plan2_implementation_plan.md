# Workspace Optimization and Build-Time Reduction for Shopping Agent

This plan outlines how to clean up the TizenClaw codebase to act as a dedicated shopping agent running on Tizen DTV or Ubuntu. It strips out developer FFI validation plugins and unused built-in tools, and fixes the 30-minute build-time bottleneck.

## User Review Required

> [!IMPORTANT]
> **Build-Time Bottleneck Root Cause**: The 30-minute build time is caused by compiling the OpenSSL C source library from scratch (`native-tls-vendored` feature in `reqwest`) and compiling SQLite from scratch (`bundled` feature in `rusqlite`) inside a QEMU-emulated `gbs build` environment.
> We propose:
> 1. Replacing `native-tls-vendored` with pure Rust `rustls-tls-native-roots`, which compiles 10x faster and doesn't require any C compiler emulation.
> 2. Removing `bundled` from `rusqlite`, linking dynamically against the target system's native `sqlite3` library (which is pre-installed on both Tizen DTV and Ubuntu).

> [!WARNING]
> **Code Removal**: This change will remove the 4 metadata plugin packages and the FFI validation shared libraries (`libtizenclaw-metadata-plugin`, etc.). These are only used for dynamic third-party Tizen RPM package injection and are not needed for a dedicated shopping daemon.

---

## Proposed Changes

### 1. Build-Time Optimization & Cargo Workspace Cleanup

#### [MODIFY] [Cargo.toml](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/Cargo.toml)
- Remove the following members from the workspace list to stop compiling them:
  - `"src/libtizenclaw"`
  - `"src/tizenclaw-metadata-plugin"`
  - `"src/tizenclaw-metadata-llm-backend-plugin"`
  - `"src/tizenclaw-metadata-skill-plugin"`
  - `"src/tizenclaw-metadata-cli-plugin"`

#### [MODIFY] [Cargo.toml](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/Cargo.toml)
- Update `reqwest` dependency: replace `native-tls-vendored` with `rustls-tls-native-roots`.
- Update `rusqlite` dependency: remove `features = ["bundled"]` to dynamically link against system SQLite.
- Remove the `openssl` crate dependency as it is no longer required.

#### [MODIFY] [Cargo.toml](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/libtizenclaw-core/Cargo.toml)
- Update `reqwest` dependency: replace `native-tls-vendored` with `rustls-tls-native-roots` and `rustls-tls`.

---

### 2. Stripping Unnecessary Built-In Tools

#### [MODIFY] [tool_declaration_builder.rs](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/src/core/tool_declaration_builder.rs)
- Keep only core shopping/interaction/search tools:
  - `remember`, `recall`, `forget` (Memory)
  - `send_outbound_message` (Interaction/Telemetry)
  - `request_user_clarification` (Interaction/Pausing)
  - `web_search`, `validate_web_search` (Web Discovery)
- Remove all other developer, code execution, supervisor, image, and document tool definitions:
  - Remove: `get_agent_status`, `list_agents`, `lookup_web_api`, `run_generated_code`, `manage_generated_code`
  - Remove: `create_task`, `list_tasks`, `cancel_task`
  - Remove: `ingest_document`, `search_knowledge`
  - Remove: `create_session`, `list_sessions`, `send_to_session`, `switch_user`
  - Remove: `create_pipeline`, `list_pipelines`, `run_pipeline`, `create_workflow`, `list_workflows`, `run_workflow`, `create_skill`, `read_skill`, `list_skill_references`, `read_skill_reference`
  - Remove: `run_supervisor`, `list_agent_roles`, `spawn_agent`
  - Remove: `extract_document_text`, `inspect_tabular_data`, `generate_image`

#### [MODIFY] [agent_core.rs](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/src/core/agent_core.rs)
- Remove the matching execution branches in `AgentCore::execute_tool` and the chat loops for the deleted tools.
- Keep execution logic only for `remember`, `recall`, `forget`, `send_outbound_message`, `request_user_clarification`, `web_search`, and `validate_web_search`.

---

## Verification Plan

### Automated Tests
- Build the optimized daemon:
  ```bash
  gbs build --arch x86_64
  # or local cargo compile for Ubuntu target:
  cargo check --bin tizenclaw
  ```
- Measure compilation time to confirm it has been reduced from ~30 minutes to under 3 minutes.

### Manual Verification
1. Run the optimized daemon.
2. Verify that `get_bridge_tool_declarations` returns only the stripped-down, focused set of shopping tools + configured MCP tools (`zepto`, `swiggy-*`).
3. Send a test query to confirm tool calls function correctly and compile size is reduced.
