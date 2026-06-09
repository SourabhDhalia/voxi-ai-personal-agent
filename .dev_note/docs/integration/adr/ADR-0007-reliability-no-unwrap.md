# ADR-0007 — Reliability: eliminate hot-path `unwrap`, supervise tasks

**Status:** Proposed
**Gap:** F1 — `panic = "abort"` + ~446 `.unwrap()` in an "always-on" daemon

## Context
`Cargo.toml` sets `panic = "abort"`, yet production code has ~446 `.unwrap()`
(hot spots: `clawhub_client` 88, `skill_capability_manager` 45,
`storage/memory_store` 38, `storage/session_store` 29, `tool_dispatcher` 22).
A corrupt session/memory file → panic → the whole persistent daemon dies. This
contradicts both the product promise and the repo's own "no `.unwrap()` in
production paths" rule.

## Decision
1. **Lint gate:** enable `clippy::unwrap_used` / `expect_used` as `deny` for
   `src/voxi/src` non-test code (allow in `#[cfg(test)]`). Introduce gradually
   via `#[allow]` opt-outs that are burned down per module.
2. **Sweep priority order:** storage → clawhub → tool_dispatcher → rest. Convert
   to `Result` propagation with `thiserror` error enums.
3. **Supervision:** run each channel (Telegram/Discord/Slack/MCP/dashboard) and
   the agent loop as supervised tasks so a panic restarts only that subtree,
   not the process. Revisit `panic = "abort"` for the daemon profile once the
   hot paths are clean (keep abort for size on constrained device builds if the
   supervision tree makes process death rare).

## Consequences
- (+) A bad input degrades one subsystem, not the whole agent; matches the
  always-on promise.
- (−) Large mechanical sweep; error-type design; do it module-by-module behind
  the lint gate so it's reviewable.

## Validation
Per-module: feed corrupt/empty store files in unit tests and assert a graceful
`Err`/skip instead of a panic. CI fails on any new `unwrap` in gated modules.
