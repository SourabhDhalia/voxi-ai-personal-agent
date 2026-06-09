# ADR-0006 — Self-verification / self-critique loop

**Status:** Proposed
**Gap:** G-A — no self-verification (Voxi only has `verify_executable` path checks)
**Peer evidence:** Claude Code (the #1 agent) — "self-verification loops" is its
listed signature feature. 2026 thesis: *scaffolding ≈ model quality*.

## Context
After the model produces a result (especially after tool use), Voxi finalizes
immediately. Top agents add a verification pass that re-checks the result
against the goal before returning — a scaffolding improvement that lifts
real-world success independent of the model.

## Decision
Add an optional, bounded verification step in the agent loop
(`agent_core/process_prompt`):
1. After the model signals completion, run a `verify` pass: a structured
   self-critique ("does this satisfy the request? what's missing/uncertain?").
2. If the critique finds a fixable gap, allow up to **K** (default 1) corrective
   iterations, then finalize regardless (bounded — no infinite loops).
3. Config-gated and per-request overridable; off by default for latency-sensitive
   channels.

Because this changes daemon-visible behavior, add the `tests/system` scenario
**first** (repo rule), then implement — ideally as part of the F4 split of the
`process_prompt` god-file so the loop is testable in isolation.

## Consequences
- (+) Higher task success; embodies the "scaffolding matters" thesis.
- (−) Added latency + tokens per verified turn; must be bounded and gated.

## Validation
`tests/system/self_verification_runtime_contract.json`: a deliberately-incomplete
first answer triggers exactly one corrective iteration, then finalizes.
