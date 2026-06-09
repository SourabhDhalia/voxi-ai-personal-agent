# ADR-0004 — Durable checkpoint of the agent loop

**Status:** Proposed
**Gap:** F8 — in-flight multi-step work is lost on restart/crash
**Peer evidence:** LangGraph's checkpointing (serializable state snapshot per
node, resumable, time-travel debuggable).

## Context
`agent_loop_state.rs` + `workflow_engine.rs` hold execution state in memory. A
restart — or, until F1 lands, a `panic = "abort"` — discards a multi-step task
midway. For a persistent daemon this breaks the core promise.

## Decision
Add a `Checkpoint` trait persisted to SQLite (Voxi already uses `rusqlite`):

```rust
trait Checkpoint: Send + Sync {
    fn save(&self, run_id: &str, state: &AgentLoopState) -> Result<()>;
    fn load(&self, run_id: &str) -> Result<Option<AgentLoopState>>;
}
```

- Snapshot after each tool step (step index, pending tool calls, partial result).
- On startup, scan for incomplete runs and resume or cleanly abandon with a log.
- Reuse the existing storage SQLite connection/pool.

## Consequences
- (+) Crash-resume; replay/debugging; pairs with the F1 reliability work.
- (−) Serialization surface for loop state; must version the snapshot schema.

## Validation
`tests/system/agent_loop_checkpoint_runtime_contract.json`: start a multi-step
run, simulate restart, assert resume from the last completed step.
