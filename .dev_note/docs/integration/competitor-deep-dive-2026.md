# Deep-Dive: Hermes · OpenClaw · QwenPaw · Skill Lists → Voxi Integration

Produced by **4 parallel sub-agents** (the explicit "parallel sub-agents" ask),
each analyzing one source via WebFetch. Research-only; no upstream code copied —
patterns ported to idiomatic async Rust within Voxi's subsystems.

Sources: [hermes-agent](https://github.com/nousresearch/hermes-agent) ·
[openclaw](https://github.com/openclaw/openclaw) ·
[QwenPaw](https://github.com/agentscope-ai/QwenPaw) (on AgentScope 2.0) ·
[awesome-agent-skills](https://github.com/heilcheng/awesome-agent-skills) ·
[awesome-ai-agents-2026](https://github.com/ARUNAGIRINATHAN-K/awesome-ai-agents-2026)

## The striking convergence
All three agent frameworks independently put the **same three** items at the top:
parallel sub-agents, durable checkpointing, tiered memory. That convergence is
the strongest possible signal for Voxi's roadmap.

## Ranked integration plan

### 1. Parallel sub-agent execution  🔴 HIGH (your named gap)
- **Hermes:** RPC subagents — each child has isolated conversation + terminal; a
  script runs *inside* the child and calls tools via RPC, collapsing N tool turns
  into "zero-context-cost"; batch mode runs workers concurrently (`max_concurrent_children`).
- **QwenPaw:** `spawn_subagent` tool — ephemeral, workspace-scoped child with its
  own reason/act loop; AgentScope `FanoutPipeline(enable_gather=True)` / `parallel_tool_calls`.
- **OpenClaw:** Lane Queue — per-session FIFO lane (serial by default), explicit
  parallel via sibling `subagent`/`cron` lanes; deterministic + replayable.
- **Voxi integration:** in `swarm_manager`/`agent_core`, add a `spawn_subagent`
  tool that builds a child `AgentCore` with a filtered toolset + path-jailed
  workspace, runs it on a `tokio::task`, joins via `tokio::task::JoinSet`,
  concurrency-capped by a `tokio::sync::Semaphore`, each child bounded by timeout
  + token budget. Add a `LaneRegistry` (per-session mpsc/single-consumer) so
  default is serial and parallelism is explicit. `Send + Sync` per project rules.

### 2. Durable checkpointing  🔴 HIGH (foundational)
- **QwenPaw:** `StateModule` trait (`state_dict`/`load_state_dict`) on every
  stateful component; nested state serializes the whole agent tree; `JSONSession`
  save/restore by id.
- **OpenClaw:** append-only **JSONL transcript** (branchable) = replayable
  checkpoint; each tool result is an "evidence" artifact; Context-Window Guard.
- **Hermes:** SQLite state across restarts, `parent_session_id` lineage,
  trajectory checkpointing; bounded `max_iterations` + iteration-budget + interrupt.
- **Voxi integration:** define a `StateModule` trait (serde) on `AgentCore`,
  `memory_store`, tool registry, active sub-agents; SQLite-backed `SessionStore`
  snapshots after each step → crash-resume + time-travel. Adopt the bounded-loop
  + interrupt guard immediately (cheap runaway protection). See ADR-0004.

### 3. Tiered memory + compaction  🟠 MED-HIGH (builds on Voxi's ONNX search)
- Working → episodic(daily) → long-term tiers; auto-compaction at a token
  threshold (Hermes 50% + `protect_last_n`; QwenPaw `COMPRESSED` marks; OpenClaw
  memory-flush promotes durable facts *before* truncating); **hybrid retrieval**
  (vector + BM25/FTS5 with reciprocal-rank fusion).
- **Voxi integration:** add a tier enum + `compact_session()` (LLM-summarize old
  turns, flush durable facts to the Markdown tier, index with existing ONNX
  embeddings) + an FTS5 BM25 lane fused (RRF) with the new `search_semantic`.
  See ADR-0005.

### 4. Self-verification loop  🟠 MED-HIGH (your named gap)
- **Hermes:** per-turn file-mutation verifier + write-time LSP diagnostics fed
  back as synthetic tool results → model self-corrects next turn.
- **QwenPaw:** `.learnings/` error log + auto-reflection middleware.
- **Voxi integration:** post-tool hook in `agent_core` — after write/patch tools,
  run a validator (rustc `--error-format=json`, JSON/YAML/TOML parse, schema) and
  inject results back; optionally let `SafetyGuard` block on failure. See ADR-0006.

### 5. Browser tool (internet capability)  🔴 HIGH net-new
- **OpenClaw:** Semantic Snapshot — derive a compact **accessibility/ARIA tree**
  (`button "Sign In" [ref=1]`, ~<50KB) instead of 5MB screenshots; act on stable
  `ref` ids via `click(ref)`/`type(ref,text)`/`navigate(url)`.
- **Voxi integration:** add a browser tool in `voxi-tool-executor` backed by CDP
  (`chromiumoxide`/`headless_chrome`); emit ARIA snapshots; run inside the sandbox
  (#6); gate behind SafetyGuard allowlist. Pair with a simpler `web_fetch`
  (fetch+extract readable text) as the non-interactive baseline — Voxi already
  has `web_search` (Brave) but **no `web_fetch`/browser**.

### 6. Execution sandbox + skill scanner  🟠 MED
- Per-session Docker isolation (OpenClaw trust-tiers; QwenPaw local/Docker/E2B;
  Hermes hardened Docker: read-only root, `--cap-drop=ALL`, `--pids-limit`).
  QwenPaw also scans skills **before install** for prompt/command injection,
  hardcoded keys, exfil patterns.
- **Voxi integration:** `SandboxBackend` trait (`Local` default, `Container` via
  `bollard`), selected per session trust-tier; route untrusted execution through
  `voxi-tool-executor`. Add a `skill_scanner` run at skill-load (regex/AST
  heuristics). Feature-gate Docker; degrade to deny-untrusted where unavailable
  (cross-platform rule). See ADR-0002.

## Skills to author (agentskills.io SKILL.md, under `data/skills/`)
Priority order (agent D): **summarize-and-extract** (no new tooling, ship first) →
**web-research** (needs `web_fetch`/browser first) → **document-ops** (PDF/DOCX/XLSX)
→ **local-data-analysis** (DuckDB-style SQL over files) → **memory-and-recall**
(rides the new tiered memory + semantic search).

## Recommended sequencing
1. **Now (tractable):** `web_fetch` tool; author `summarize-and-extract` +
   `web-research` skills; bounded-loop interrupt guard.
2. **Foundational:** `StateModule`+`SessionStore` checkpointing (#2) → unlocks safe
   parallel fan-out and verify-and-retry.
3. **Headline:** parallel `spawn_subagent` (#1) on top of #2.
4. Tiered memory (#3), self-verification (#4), browser tool (#5), sandbox (#6).

> Caveat (from the agents): Hermes/OpenClaw marketing overstates "durability"
> (loops are largely synchronous); we port the *patterns*, not the hype.
