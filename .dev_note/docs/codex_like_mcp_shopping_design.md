# Codex-Like MCP Shopping Design

## Intent

Make shopping MCP use deterministic tool metadata and session state instead
of relying only on LLM memory. The daemon should discover provider tools,
summarize how each tool behaves, preserve provider-specific identifiers, and
keep user-facing replies short.

## Design

- Build `McpToolBehavior` records from live `tools/list` metadata.
- Index behavior docs into the existing SQLite embedding store at startup,
  using ONNX embeddings when available and keyword search as fallback.
- Normalize MCP results into success, business error, auth required, user
  action required, ambiguous, or fatal outcomes.
- Track shopping options per session so `1`, `1st`, or `3d` maps to the
  last numbered options shown to the user.
- Preserve raw MCP results for execution and compact only display copies.
- Treat provider-unspecified shopping as compare-first across configured
  shopping MCP providers.

## Verification

- Use deploy scripts only; no local cargo build, check, test, or clippy.
- Validate JSON/prompt syntax with lightweight non-cargo commands.
- If real GBS or device tooling is missing, record the pre-flight blocker.
