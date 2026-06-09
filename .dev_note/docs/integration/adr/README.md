# Architecture Decision Records

ADRs for the larger architectural gaps identified in the competitive analysis.
These are **designed, not yet implemented** — each is scoped so it can be built
deliberately and reviewed before landing.

| ADR | Decision | Drives gap | Effort |
|-----|----------|-----------|--------|
| [0001](ADR-0001-canonical-runtime-and-policy.md) | Canonicalize on `vclaw-*`; route safety through `policy_engine` | F5, F2, F6 | M (incremental) |
| [0002](ADR-0002-execution-sandbox.md) | `ToolSandbox` trait + tiered isolation backends | P1/F6 (table-stakes) | L |
| [0003](ADR-0003-skill-self-authoring.md) | Solve→write reusable skill loop; `agentskills.io` format | P3 | M |
| [0004](ADR-0004-durable-checkpoint.md) | Checkpoint agent loop to SQLite; resume on restart | F8 | M |
| [0005](ADR-0005-tiered-memory.md) | core/archival/recall tiers + eviction | F7 | M–L |
| [0006](ADR-0006-self-verification-loop.md) | Self-critique/verification pass before finalizing | G-A | M |
| [0007](ADR-0007-reliability-no-unwrap.md) | Remove unwraps on hot paths; supervised tasks | F1 | M–L |

Status legend: Proposed → Accepted → Implemented. All below are **Proposed**.
