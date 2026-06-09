# Voxi — Competitive Analysis, Flaws & OSS Integration Plan

> Self-paced analysis produced via `/loop`. Grounds Voxi against the 5 leading
> open-source agents (June 2026), lists concrete flaws with `file:line`
> evidence, proposes fixes, and gives a phased integration plan.
> Every item is tagged with the relevant skill: `architecture-designer`,
> `rust-engineer`, `unsafe-checker`, `secure-code-guardian`, `sre-engineer`,
> `code-documenter`, `embedded-systems`, `review`.

---

## 0. Executive summary

Voxi is a genuinely ambitious **persistent Rust agent daemon** (4 LLM backends,
Telegram + MCP channels, swarm manager, perception/context/intent engines,
session + memory stores, web dashboard, voice). It is *architecturally ahead*
of most OSS agents in being a long-running daemon with a stable IPC surface and
real system-scenario tests (`tests/system/*.json`).

But it has three classes of problems that directly contradict its own promises:

1. **Reliability** — `panic = "abort"` + ~446 `.unwrap()` in a daemon sold as
   "always-on." One bad unwrap kills everything. (→ `sre-engineer`)
2. **Security** — the `SafetyGuard` is a tiny case-sensitive substring denylist
   that is trivially bypassed, and its prompt-injection check is dead code.
   (→ `secure-code-guardian`)
3. **Maintainability** — god-files (`process_prompt.rs` = 4,735 LOC), two
   competing runtimes (`src/voxi` legacy vs `rust/crates/vclaw-*` canonical)
   with no migration seam. (→ `architecture-designer`)

The 5 OSS leaders each solve one thing Voxi is currently weak at: **sandboxed
execution** (OpenHands), **structured tool feedback** (SWE-agent's ACI),
**repo-aware context** (Aider), **self-editing tiered memory** (Letta/MemGPT),
and **durable checkpointed state** (LangGraph). The integration plan below
borrows one signature idea from each.

---

## 1. What Voxi is today (grounded snapshot)

| Tree | LOC | Role | Status |
|------|-----|------|--------|
| `src/voxi*` | ~77k | Main daemon: `agent_core`, 4 LLM backends, Telegram/MCP channels, swarm, perception/context/intent engines, storage, dashboard, voice | **Active** |
| `rust/crates/vclaw-*` | ~15k | Canonical reconstruction: `runtime`, `policy_engine`, `permission_enforcer`, `mcp_stdio`, conversation engine | Forward-looking |
| `src/*.py`, `mcphelper/` | ~22 files | Python parity + MCP HTTP bridge | Support |

Strengths worth protecting: multi-provider LLM abstraction
(`src/voxi/src/llm/`), MCP client (`channel/mcp_client.rs`, 3,104 LOC),
JSON-RPC IPC contract with scenario tests, and an existing hook deny path
(`tests/system/hooks_pre_tool_deny.json`).

---

## 2. The 5 leading OSS agents and their signature feature

| Agent | The one thing it does best | Why Voxi should care |
|-------|---------------------------|----------------------|
| **OpenHands** (ex-OpenDevin) | **Sandboxed Docker/K8s runtime** with full RW filesystem + an agent SDK + Planning Mode (v1.6, Mar 2026). Used by AMD/Apple/Google. | Voxi runs tools with a *denylist*. OpenHands isolates the blast radius instead. Voxi needs an isolation boundary, not a string filter. |
| **SWE-agent** (Princeton/Stanford) | **Agent-Computer Interface (ACI)** — structured, compact tool commands + guard-railed feedback. `mini-SWE-agent` (~100 LOC) scores >74% SWE-bench. Has a dedicated security/CTF mode. | Voxi's tool results are ad-hoc. A typed ACI layer would shrink token cost and improve tool reliability. |
| **Aider** | **Repo map via tree-sitter** — ranks and feeds only the relevant slice of a large codebase + tight git commit loop. | Voxi has no code-repo grounding; `context_engine` works but isn't repo/structure-aware. Big win for the Telegram coding workflow. |
| **Letta (MemGPT)** | **3-tier self-editing memory** (core / archival / recall) the agent pages via function calls. Persistence-by-default. | Voxi has `memory_store` + `session_store` but no agent-controlled tiering/eviction. This is the highest-leverage upgrade for a *persistent* agent. |
| **LangGraph** | **Durable checkpointing** — every node emits a serializable state snapshot to Postgres/SQLite/Redis; resumable, time-travel debuggable. | Voxi's `agent_loop_state` + `workflow_engine` aren't checkpointed. A crash (see panic=abort) loses in-flight work. Pairs directly with the reliability fix. |

Sources:
[awesome-ai-agents-2026](https://github.com/Zijian-Ni/awesome-ai-agents-2026) ·
[OpenHands](https://github.com/OpenHands/OpenHands) ·
[OpenHands SDK](https://docs.openhands.dev/sdk) ·
[Devin vs OpenHands vs SWE-agent](https://toolhalla.ai/blog/devin-vs-openhands-vs-swe-agent-2026) ·
[Top 8 OSS coding agents 2026](https://www.opensourceaireview.com/blog/top-8-open-source-coding-agents-in-2026) ·
[Letta/MemGPT walkthrough](https://sureprompts.com/blog/letta-memgpt-walkthrough) ·
[Letta vs LangGraph memory](https://vectorize.io/articles/letta-vs-langchain-memory) ·
[Agent memory architectures](https://zylos.ai/research/2026-04-05-ai-agent-memory-architectures-persistent-knowledge/)

---

## 3. Feature-gap matrix

| Capability | Voxi | OpenHands | SWE-agent | Aider | Letta | LangGraph |
|-----------|:----:|:---------:|:---------:|:-----:|:-----:|:---------:|
| Persistent daemon | ✅ strong | ⚠️ session | ❌ | ❌ | ✅ | ⚠️ |
| Multi-provider LLM | ✅ (4) | ✅ | ✅ | ✅ | ✅ | ✅ |
| MCP support | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ |
| Multi-agent / swarm | ✅ basic | ✅ | ❌ | ❌ | ⚠️ | ✅ |
| **Sandboxed execution** | ❌ denylist only | ✅✅ | ⚠️ | ❌ | ❌ | ⚠️ |
| **Structured tool interface (ACI)** | ⚠️ ad-hoc | ✅ | ✅✅ | ✅ | ⚠️ | ✅ |
| **Repo-aware code context** | ❌ | ✅ | ✅ | ✅✅ | ❌ | ❌ |
| **Tiered self-editing memory** | ⚠️ flat store | ⚠️ | ❌ | ❌ | ✅✅ | ⚠️ |
| **Durable checkpoint/resume** | ❌ | ⚠️ | ❌ | ⚠️ git | ✅ | ✅✅ |
| Hardened safety/policy | ⚠️ weak | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ |

The four `❌`/weak columns for Voxi (sandbox, repo-context, tiered memory,
durable checkpoint) are exactly the integration targets in §6.

---

## 4. Flaws in the current code (ranked, with evidence)

### F1 — `panic = "abort"` + ~446 `.unwrap()` in an "always-on" daemon  🔴 Critical
- `Cargo.toml`: `panic = "abort"`. Any panic terminates the whole process.
- ~446 `.unwrap()` in `src/voxi/src` (excluding tests, hotspots:
  `clawhub_client.rs` 88, `skill_capability_manager.rs` 45,
  `storage/memory_store.rs` 38, `storage/session_store.rs` 29,
  `tool_dispatcher.rs` 22). The storage ones are the scariest — a corrupt
  session file panics → daemon dies → "persistent agent" isn't.
- Directly violates `.claude/CLAUDE.md`: *"No `.unwrap()` in production paths."*
- **Skill:** `sre-engineer`, `rust-engineer`

### F2 — `SafetyGuard` is a bypassable substring denylist  ✅ FIXED & VERIFIED
> Hardened in `src/voxi/src/core/safety_guard.rs`: normalization
> (whitespace-collapse + case-fold), 13 regex dangerous-command patterns
> (flag-reorder resistant), structured-arg blob scanning for split commands,
> and an allowlist mode. 10 new regression tests added; full workspace test run
> green (513 passed, 0 failed). `check_prompt_injection` wiring remains a
> follow-up (see fix item 4 below). Original finding kept for the record:
- `src/voxi/src/core/safety_guard.rs:129` — `args.contains(blocked)`, case-
  sensitive, on a 5-entry list (`"rm -rf /"`, `mkfs`, `dd if=`, …).
- Trivially bypassed: `rm  -rf /` (double space), `rm -fr /`, `RM -RF /`,
  `find / -delete`, `$(printf 'rm')`, base64/env indirection.
- `check_prompt_injection` (`safety_guard.rs:184`) is **defined but never
  called** anywhere in the codebase — dead safety control giving false comfort.
- No allowlist model, no shell tokenization, no path canonicalization.
- **Skill:** `secure-code-guardian`, `security-reviewer`, `review`

### F3 — FFI plugin loader uses raw `dlopen`/`transmute_copy`, violating the `libloading` rule  🟠 High
- `src/voxi/src/llm/plugin_llm_backend.rs`: raw `libc::dlopen`/`dlsym`
  (`:69`, `:105`) + `std::mem::transmute_copy(&sym)` (`:109`) +
  hand-written `unsafe impl Send/Sync` (`:46-47`).
- `CLAUDE.md` rule: *"Voxi `.so` symbols must be loaded via `libloading`. The
  daemon must never panic if a native library is absent."* `transmute_copy` of
  a `dlsym` result is a classic UB footgun; the manual `Send/Sync` asserts
  thread-safety on a type holding raw C handles with no justification comment.
- **Skill:** `unsafe-checker`, `rust-engineer`

### F4 — God-files / weak module boundaries  🟠 High
- `agent_core/process_prompt.rs` = **4,735 LOC, 41 fns**; `tool_dispatcher.rs`
  49 fns; `mcp_client.rs` 3,104 LOC; `telegram_client.rs` 2,717 LOC.
- Hard to review, unit-test, or reason about ownership/locking. The agent's hot
  loop lives in one giant file.
- **Skill:** `architecture-designer`, `rust-engineer`

### F5 — Two competing runtimes with no migration seam  🟡 Medium
- `src/voxi` (legacy, active) and `rust/crates/vclaw-*` (canonical) both
  implement policy/permission/MCP concepts. No documented adapter or cutover
  plan → silent drift, duplicated bugs, unclear source of truth.
- **Skill:** `architecture-designer`

### F6 — No execution isolation / blast-radius control  🟡 Medium
- Tools (shell, file, system_cli_adapter) run in the daemon's own process
  space, gated only by F2's denylist. No seccomp, namespace, container, or
  even a fs/cmd allowlist root.
- **Skill:** `secure-code-guardian`, `sre-engineer`, `embedded-systems`
  (on-device this matters even more)

### F7 — Flat memory store, no tiering/eviction policy  🟡 Medium
- `storage/memory_store.rs` + `session_store.rs` persist but have no
  core/archival/recall tiering or token-budget eviction. Long-lived sessions
  will bloat context. (This is exactly Letta's solved problem.)
- **Skill:** `architecture-designer`

### F8 — No durable checkpoint of the agent loop  🟡 Medium
- `agent_loop_state.rs` + `workflow_engine.rs` hold state in memory; a restart
  (or F1 abort) loses in-flight multi-step work. No resume/replay.
- **Skill:** `sre-engineer`

---

## 5. Fixes (concrete, mapped to flaws)

| ID | Fix | Effort |
|----|-----|--------|
| **F1** | (a) Add a CI lint gate: `clippy::unwrap_used`/`expect_used` deny on `src/voxi/src` non-test. (b) Sweep storage + clawhub + dispatcher unwraps → `Result` propagation with `thiserror` error enums. (c) Reconsider `panic = "abort"` for the daemon profile, or wrap the agent loop in `catch_unwind`-isolated task supervision so one task panic doesn't abort the process. | M–L |
| **F2** | Replace denylist with: (1) **allowlist** of permitted tools/commands; (2) shell tokenization (`shell-words`) + per-arg canonicalization for path/command checks; (3) case-insensitive + normalized matching as defense-in-depth only; (4) **wire `check_prompt_injection` into the prompt intake path** or delete it; (5) add `tests/system/safety_guard_bypass_runtime_contract.json` covering the bypass vectors. | M |
| **F3** | Migrate `plugin_llm_backend.rs` to `libloading::Library` + typed `Symbol<fn…>` (no `transmute_copy`); graceful fallback when `.so` absent; document the `Send/Sync` safety invariant or remove the manual impls. Run `unsafe-checker` over the result. | M |
| **F4** | Split `process_prompt.rs` into `intake`, `tool_loop`, `grounding`, `finalize` submodules behind a thin orchestrator; same for `tool_dispatcher`. No behavior change — pure structural refactor validated by existing scenarios + `review`. | M |
| **F5** | Write an ADR (`architecture-designer`) declaring `vclaw-*` the target, define an adapter trait boundary, and a per-subsystem cutover checklist. Freeze new features in legacy where canonical exists. | S (doc) + ongoing |
| **F6** | Introduce a `ToolSandbox` trait with backends: `none` (host, dev), `subprocess+rlimits/namespace` (Linux), and a documented on-device profile. Default the daemon to least-privilege. | L |
| **F7** | Add memory tiering (core/archival/recall) + token-budget eviction to `memory_store`, exposed as agent-callable tools (Letta pattern). | M–L |
| **F8** | Add a checkpoint trait on `agent_loop_state` (serialize after each tool step) with SQLite backend; resume on startup (LangGraph pattern). | M |

---

## 6. Integration plan — borrow one signature idea from each (phased)

### Phase 0 — Stabilize (must precede any feature work)
- **F1** reliability sweep + clippy gate, **F2** safety hardening, **F3**
  `libloading` migration. These are the foundation; the daemon must not abort
  and must not be trivially exploitable before adding capability.
- Add `tests/system/*` scenarios *first* per repo rule, validate with
  `./deploy_host.sh --test` + `voxi-tests`.

### Phase 1 — Durable state (from **LangGraph**)
- Checkpoint the agent loop (F8) to SQLite. Unlocks crash-resume and
  time-travel debugging. Smallest surface, biggest reliability payoff, and it
  composes with Phase 0.

### Phase 2 — Tiered memory (from **Letta/MemGPT**)
- Implement core/archival/recall tiering (F7) with agent-callable
  `memory_insert` / `memory_search` / `memory_evict` tools. Highest-leverage
  upgrade for a *persistent* agent. (Note Letta's caveat: LLM-managed paging
  adds latency/token cost — keep paging heuristically gated.)

### Phase 3 — Structured tool interface (from **SWE-agent ACI**)
- Define a typed `ToolResult` envelope (status, stdout window, truncation,
  next-action hints) and compact command grammar. Reduces tokens, improves
  tool-call success. Reuse SWE-agent's "security/CTF mode" idea as an opt-in
  Voxi tool profile.

### Phase 4 — Repo-aware context (from **Aider**)
- Add a tree-sitter repo map to `context_engine` so the Telegram coding
  workflow (codex/gemini/claude CLIs) feeds only the relevant code slice.

### Phase 5 — Execution sandbox (from **OpenHands**)
- Implement the `ToolSandbox` trait (F6). On host: subprocess + rlimits +
  namespaces; design an on-device least-privilege profile (`embedded-systems`).
  This is the largest effort and depends on Phase 0's allowlist model.

### Cross-cutting — Documentation (`code-documenter`)
- After Phases 0–1, generate module-level rustdoc for the split `agent_core`
  submodules and an updated architecture overview under `docs/`.

---

## 7. Recommended first action

**Harden `SafetyGuard` (F2)** is the best first concrete fix: small, isolated,
high security value, testable via a new `tests/system` scenario, and it doesn't
require touching the two-runtime question. It is the implementation target for
the next loop iteration.

---

## 8. Original ideas beyond the brief ("use your mind")

- **Capability tokens, not denylists.** Replace ad-hoc gating with a single
  `policy_engine` (already exists in `vclaw-*`!) that issues scoped capability
  grants per session. This *unifies* F2/F5/F6 and gives the two-runtime
  migration a concrete first beachhead: route legacy safety checks through the
  canonical `policy_engine`.
- **Panic supervision tree.** Borrow Erlang/Tokio supervisor semantics: each
  channel (Telegram, MCP, dashboard) runs as a supervised task; a crash
  restarts only that subtree, not the daemon. Directly rescues the
  `panic = "abort"` problem for an always-on device agent.
- **Scenario-driven safety regression suite.** The repo already has a strong
  `tests/system/*.json` harness — extend it into a standing "red-team" suite of
  bypass attempts so safety can't silently regress.

---

## 9. 2026 Refresh (latest benchmark-driven field)

> Re-validated against live mid-2026 data. (`billboard.li/agents` was
> unreachable — 403 to the fetcher, no connected browser — so this uses
> indexed leaderboards instead.) The §2 list was *framework*-centric; the
> current field is *capability/benchmark*-led and surfaces new gaps.

### Current ranked field (software-dev agents, mid-2026)

| Agent | Benchmarks | Distinguishing feature |
|-------|-----------|------------------------|
| Claude Code (Opus 4.x) | 87.6% SWE-V, 69.4% Terminal | **Multi-agent coordination + self-verification loops** |
| OpenAI Codex (GPT-5.5) | 82.7% Terminal #1 | Terminal-native DevOps |
| Cursor | scales w/ model | AI-native IDE, Plan/Act mode |
| Gemini CLI | 80.6% SWE-V | Free tier, cloud integration |
| Devin 2.0 | — | **Sandboxed cloud VM + autonomous PR submission** |
| OpenHands | 72% SWE-V | Open MIT, **100+ LLM backends** |
| Augment Code | 70.6% | **Full repository indexing**, MCP-interoperable |
| Aider / Cline | model-dep | Git-native terminal / VS Code, open-source |
| Manus | — | Plain-English → full-stack app generation |

### The decisive 2026 insight: *scaffolding ≈ model quality*
Three frameworks running the **same** model showed large variance across 731
tasks. Real-world agent performance is gated by *scaffolding* (tool interface,
verification, context, memory) as much as by the model. **This validates the
entire thrust of this plan** — invest in Voxi's scaffolding, don't chase models.
Corollary: ignore raw SWE-bench numbers (Feb-2026 audit found 59.4% of cases
flawed / contaminated); measure real task success instead.

### New gaps surfaced by the refresh

| ID | Gap | Evidence in Voxi | Borrow from |
|----|-----|------------------|-------------|
| **G-A** | **Self-verification / self-critique loop** | none (`verify_executable` only path-checks binaries) | Claude Code (the #1 signature feature) |
| **G-B** | **Batch / headless execution mode** (`/batch`) | none (no non-interactive bulk task runner) | Claude Code headless, Devin autonomous runs |
| **G-C** | **Repo/code indexing** (`/init`-style code map) | `tool_indexer` indexes tools, not code; `init.rs` exists only in canonical CLI, not wired to the daemon | Aider repo-map, Augment full-repo index |
| (reaffirmed) | **Execution sandbox now = table stakes** | F6 still open | Devin 2.0 cloud VM, OpenHands Docker |

### What Voxi already has that the 2026 field prizes
- **MCP is now the interoperability standard** — Voxi's `mcp_client.rs`
  (3,104 LOC) is a strategic asset. Position Voxi as an *MCP-native persistent
  orchestrator*, not another one-shot coder.
- **`/update`** via ClawHub registry already exists (most OSS agents lack a
  self-update/skill-distribution channel).
- **Persistent daemon + swarm** aligns with the "layered multi-agent stack"
  trend (~70% of devs now run several agents at once).

### Re-prioritized roadmap (supersedes §6 ordering)
1. **Phase 0 — Stabilize** (unchanged): F1 unwrap/panic, **F2 ✅ done**, F3 libloading.
2. **NEW high-leverage: G-A self-verification loop** — small scaffolding change,
   directly mirrors the #1 agent and the "scaffolding matters" thesis. Promote
   above the larger memory/sandbox initiatives.
3. **Quick wins:** expose `/init` in the daemon CLI; add `/batch` headless mode (G-B).
4. **Phase 1–2:** durable checkpoint (LangGraph) + tiered memory (Letta) — still
   the highest-leverage *persistence* upgrades.
5. **Phase 3–5:** ACI tool interface (SWE-agent), repo index (G-C / Aider),
   execution sandbox (F6 — now table-stakes, raise priority).

---

## 10. Grounded analysis vs the REAL top-usage peers (decisive)

> Source: live token-usage leaderboard (OpenRouter "Top Apps"). The agents that
> actually matter for Voxi are its *category peers* — persistent, local-first,
> multi-channel personal agent daemons — not generic coding agents. By real
> usage these are **#1 Hermes Agent (747B tok/day)** and **#3 OpenClaw (153B)**,
> with **#2 Kilo Code (186B)** as the multi-agent coding reference. Each gap
> below is grounded against Voxi's actual source.

### The peers
- **Hermes Agent** (Nous Research): persistent cross-session memory; **self-writing
  reusable skills** on the `agentskills.io` open standard; 6 channels via one
  gateway; **isolated subagents with their own terminals**; **5 execution
  backends (local/Docker/SSH/Singularity/Modal) with namespace isolation**;
  any-model switching (`hermes model`, 200+ via OpenRouter); single-curl install.
- **OpenClaw** (MIT, "fastest-growing OSS project ever" — Jensen Huang): ~25 chat
  channels; 100+ portable skills; always-on/proactive; **memory as Markdown files
  on disk**; voice wake; NVIDIA NemoClaw secure-runtime integration.
- **Kilo Code** (Cline fork): Plan/Code/Debug **modes + Orchestrator** multi-agent
  decomposition; **Memory Bank**; MCP marketplace; 500+ models.

### Where Voxi is already competitive or AHEAD (grounded — protect these)
| Capability | Voxi reality | vs peers |
|-----------|--------------|----------|
| Multi-channel | **9 channels**: Telegram, Discord, Slack, Voice, TV, Webhook, Web dashboard, MCP client+server, **A2A** (`src/voxi/src/channel/`) | ≈ Hermes (6); behind OpenClaw (~25) |
| Memory store | **Hybrid SQLite index + Markdown sync** for LTM (`storage/memory_store.rs`) | Ahead of OpenClaw's pure-Markdown |
| Multi-agent | **`orchestrator` role** + `swarm_manager` + `workflow_engine` | Has Kilo-Orchestrator primitives |
| Interop | MCP **client + server** + A2A handler | Strong; matches the MCP-standard trend |
| Skills + update | ClawHub registry, `skill_capability_manager`, voice | Few peers have a self-update channel |

**Takeaway:** Voxi is genuinely in the same league as the #1/#3 apps; it does not
need a rebuild, it needs targeted gap-closing.

### Real gaps vs these peers (grounded)
| ID | Gap | Voxi reality | Peer with it | Effort |
|----|-----|--------------|--------------|--------|
| **P1** | **No execution sandbox / isolated subagents** | tools run in-process; F6 still open | Hermes (5 backends, namespaces), OpenClaw (NemoClaw) | L — but now **table-stakes** |
| **P2** | **No OpenRouter backend** | backends = anthropic/openai/gemini/ollama/plugin; **but `openai.rs` already does OpenAI-compatible base-URL switching (xai/grok)** | Hermes 200+, Kilo 500+ | **S — likely a config add on the existing OpenAI-compat path** |
| **P3** | **Skill self-authoring loop** | has skill *management* (`auto_skill_agent`, ClawHub) but no clear "solve→write reusable skill" loop; not on `agentskills.io` standard | Hermes (signature) | M |
| **P4** | Channel breadth | missing WhatsApp/Signal/iMessage/Matrix | OpenClaw | S each, additive |
| **P5** | One-command onboarding | `deploy.sh`/`install.sh` are heavy | Hermes/OpenClaw single curl | S — DX |

### Revised top recommendations (grounded, supersedes §9 ordering)
1. **Quick win — P2 OpenRouter backend.** Highest value-to-effort: instant access
   to 200+ models, likely a small addition to the existing OpenAI-compatible
   `openai.rs` (which already swaps base URLs). Matches what every top peer
   advertises.
2. **Strategic table-stakes — P1 execution sandbox (F6).** Both #1 and #3 peers
   ship isolation; this is no longer optional for an always-on agent that runs
   shell/file tools. Pair with the SafetyGuard work already done (F2).
3. **Differentiator — P3 skill self-authoring + `agentskills.io` compatibility.**
   Turns Voxi's existing ClawHub/skill system into Hermes-class self-improvement.
4. Keep the scaffolding bets: G-A self-verification, F8 durable checkpoint,
   F7 tiered memory.
5. Stabilize first regardless: F1 (unwrap/panic), **F2 ✅**, F3 (libloading).
