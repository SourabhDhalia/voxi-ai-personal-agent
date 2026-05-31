# VClaw Features & Usage Guide

This guide describes the core capabilities of **VClaw** (the canonical Rust agent CLI located in `rust/`) and explains how to run, configure, and inspect its features.

---

## 1. Running the CLI Agent

The front-end interface is provided by the `rusty-claude-cli` crate.

### A. Interactive REPL Mode
To run VClaw in an interactive terminal prompt loop (similar to standard Claude Code prompts):
```bash
cargo run -p rusty-claude-cli
```
This boots you into the VClaw shell:
```text
VClaw Interactive Console (type 'exit' or 'quit' to exit)
vclaw > 
```
Type any prompt directly or run slash commands.

### B. Standard Stdio Command Mode
You can execute single prompts directly from your shell:
```bash
cargo run -p rusty-claude-cli -- "Find and list all files in the src directory"
```

### C. Piped Stdin Mode
You can pipe text content directly into the agent for processing:
```bash
cat src/main.rs | cargo run -p rusty-claude-cli -- "Explain this entrypoint"
```

---

## 2. Interactive Slash Commands

In the interactive REPL shell (`vclaw > `), you can use the following commands:
- `/doctor` (or `/diagnose`): Runs the environment health check.
- `/resume <session-id>`: Loads and continues a previous conversation session.
- `/exit` (or `/quit`): Exits the interactive shell.

---

## 3. Environment Diagnostics (`--doctor` / `/doctor`)

The diagnostics tool audits the execution host to identify missing dependencies, keys, or directories.

### Usage
- **CLI Flag**: `cargo run -p rusty-claude-cli -- --doctor`
- **Interactive Command**: `/doctor`

### Performed Audits
1. **Directory Paths**: Validates read & write access to `root_dir`, `session_dir`, `plugin_dir`, and `log_dir`.
2. **Terminal Tools**: Checks if common command-line utility tools (`git`, `curl`, `python3`) are installed on the host path.
3. **Secrets / Environment Keys**: Checks for the presence of environment variables:
   - `GEMINI_API_KEY`
   - `ANTHROPIC_API_KEY`
   - `OPENAI_API_KEY`
4. **MCP Server Reachability**: Reads server commands from the Model Context Protocol (`mcp_servers.json`) configuration and checks if the executable binaries are reachable on the system.

---

## 4. Loop Guards & Execution Budget Control

To protect your API budgets from run-away agent loops (e.g. when an LLM repeats the same tool call recursively due to errors), VClaw uses a strict stateful loop tracking guard:

- **Stage 1 (System Warning)**: If the agent executes the exact same tool with the same arguments for **3 consecutive turns**, the engine automatically appends a `System` message warning to the conversation prompt:
  > *"System Warning: You have invoked the tool '<tool_name>' with matching arguments 3 consecutive times. Please verify if your approach is stuck in a loop."*
- **Stage 2 (Abort Execution)**: If the agent attempts a **4th consecutive** identical execution, the engine terminates the turn and throws a `LoopDetected` runtime error.

---

## 5. Built-in Filesystem Discovery Tools

VClaw is equipped with filesystem discovery tools that run under a safe `Low` permission scope to prevent unauthorized command execution:

### A. List Directory (`fs.list_directory`)
Lists files and folders under a target directory with item types and size details.
- **Parameters**: `{"path": "/absolute/or/relative/path"}`
- **Permissions**: Scope: `Read`, Level: `Low`.

### B. Glob Search (`fs.glob`)
Searches recursively for files matching a wildcard pattern (e.g. `src/**/*.rs` or `docs/*.md`).
- **Parameters**: `{"pattern": "src/**/*.rs"}`
- **Permissions**: Scope: `Read`, Level: `Low`.
- **Syntax**: Supports double wildcards `/**/` for recursive directory searching.

### C. Standard File Operations
- `fs.read_text`: Reads UTF-8 file contents into memory.
- `fs.write_text`: Safely writes modified texts back to files (runs under `Standard` write permissions).
- `fs.search_text`: Scans lines in target files for exact query matches.

---

## 6. Semantic Discovery & Cosine Similarity

VClaw uses an on-device ORT (ONNX Runtime) embedding library to retrieve and select matching skills:
- When a user submits a prompt, VClaw generates a prompt embedding.
- If no exact command or tool mapping is matched, the engine calculates **cosine similarity** between the prompt embedding and indexed developer skills.
- The highest-scoring skills are pre-fetched and injected into the planning context dynamically.

---

## 7. Sanitized Credential Logging

To protect sensitive API tokens and cookies from leaking into local logs or session stores:
- Credentials in Model Context Protocol (MCP) headers (e.g. `Mcp-Session-Id`, `Bearer` tokens, OAuth authorization secrets) are intercepted by regex filters.
- All intercepted credential blocks are rewritten to `<HIDDEN_TOKEN>` before being written to disk logs, session history files, or dashboard caches.
