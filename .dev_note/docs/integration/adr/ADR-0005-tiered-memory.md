# ADR-0005 — Tiered self-editing memory (core / archival / recall)

**Status:** Proposed
**Gap:** F7 — flat memory store, no tiering or token-budget eviction
**Peer evidence:** Letta/MemGPT 3-tier model; OpenClaw Markdown-file memory.

## Context
Voxi's `storage/memory_store.rs` is already a **hybrid SQLite index + Markdown
sync** (ahead of OpenClaw's pure-Markdown). What it lacks is *tiering* and
*eviction*: long-lived sessions grow the context injected into prompts without
bound.

## Decision
Layer a tier model over the existing store (do **not** replace it):
- **core** — always injected, small, agent-editable (identity, active goals).
- **archival** — large, durable, retrieved on demand by search.
- **recall** — recent conversation window, FIFO with token budget.

Expose agent-callable tools `memory_insert` / `memory_search` / `memory_evict`
so the model manages its own memory (Letta pattern). Gate LLM-driven paging
behind a heuristic to avoid Letta's known per-turn latency/token overhead.

## Consequences
- (+) Bounded prompt growth; agent-curated long-term memory; reuses SQLite index.
- (−) Eviction policy needs tuning; paging adds some latency when triggered.

## Validation
`tests/system/tiered_memory_runtime_contract.json`: insert across tiers, assert
core is always injected, archival is retrieved only on search, recall evicts at
budget.
