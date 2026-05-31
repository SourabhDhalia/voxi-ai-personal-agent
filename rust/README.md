# VClaw: Canonical Rust Agent Workspace

This directory contains **VClaw**, a standalone, high-performance CLI coding agent runtime (inspired by Claude Code/Claw Code workflows) that operates directly on the host repository workspace. It acts as the modern, modular CLI workflow for Voxi developers.

## Workspace Layout

- **`crates/rusty-claude-cli`**: The CLI front-end, containing the interactive console prompt (REPL), arguments parser, and human/JSON output formatters.
- **`crates/vclaw-runtime`**: The core orchestration engine. Controls turn-based planning, tool execution logic, prompt template construction, vector storage, and safety rules.
- **`crates/vclaw-tools`**: The built-in and modular tool implementation registry.
- **`crates/vclaw-commands`**: Operational helper commands and interactive console slash commands.
- **`crates/vclaw-api`** & **`crates/vclaw-plugins`**: Common types and dynamic loading integration.

---

## Features

### 1. Interactive REPL Prompt Mode
Simply run the CLI binary without arguments in a terminal environment to boot into the interactive shell:
```bash
$ cargo run -p rusty-claude-cli
VClaw Interactive Console (type 'exit' or 'quit' to exit)
vclaw > Explain how the doctor command works
...
```

### 2. Slash Commands
In interactive mode, you can type special commands to query or configure the runtime:
- `/doctor` (or `/diagnose`): Runs the environment health check.
- `/resume <session-id>`: Reconnects to and resumes a prior active conversation session.
- `/exit` (or `/quit`): Exits the shell cleanly.

### 3. Diagnostics Command (`--doctor` or `/doctor`)
Run `cargo run -p rusty-claude-cli -- --doctor` to perform a comprehensive audit of the execution host:
- **Paths**: Verifies read/write access to runtime directories (`root_dir`, `session_dir`, `plugin_dir`, `log_dir`).
- **Tooling**: Audits for essential shell utilities (`git`, `curl`, `python3`).
- **Environment**: Checks if API keys are set (`GEMINI_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`).
- **MCP Servers**: Automatically tests reachability and executable paths for configured Model Context Protocol (MCP) servers.

### 4. Loop Guards & API Budget Control
To prevent runaway LLM execution loops (and infinite API spend), VClaw implements a multi-stage loop guard inside the turn execution:
- **3rd Consecutive Turn**: If the agent attempts to call the exact same tool with matching arguments for 3 consecutive loops, VClaw injects a `System` warning message advising the model that it is looping.
- **4th Consecutive Turn**: If the loop persists, the engine immediately aborts the current conversation turn with a `LoopDetected` runtime error.

### 5. Built-in Discovery Tools
Equipped with low-permission, high-speed tools for codebase search and filesystem exploration:
- `fs.list_directory`: Lists directory items, sizes, and file type metadata.
- `fs.glob`: Recursively scans directories to locate files matching glob patterns (e.g. `src/**/*.rs`), featuring a self-contained wildcard matching algorithm.
- `fs.read_text`, `fs.write_text`, `fs.search_text`: Standard codebase reading and searching toolsets.

### 6. Semantic Skill Discovery
VClaw uses an on-device ORT embedding engine to compute cosine similarity scores on user prompts. If an exact tool mapping is not found, the agent automatically falls back to semantic search to find and prefetch matching workflow templates and skills.

### 7. Sanitized Credential Logging
All session logs and prompt history records are scanned by regex-based sanitizers. Credentials such as Model Context Protocol (MCP) authorization headers, bearer tokens, or custom API keys are automatically masked (e.g. `<HIDDEN_TOKEN>`) before being persisted to disk.

---

## Testing & Verification

Run tests inside the canonical Rust workspace using standard Cargo command sets:
```bash
cargo test --manifest-path rust/Cargo.toml --offline
```
*Note: The locked dependency specifications in `Cargo.lock` are configured to point directly to local offline cache packages for consistent builds in sandbox targets.*
