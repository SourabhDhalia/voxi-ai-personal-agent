# Voxi — Remaining Work: Detailed Execution Plan

Self-contained playbook for the work **not yet done**. Written so a fresh agent
can execute each task cold. Pair each task with its ADR in `adr/` for rationale.

Branch: `claude/eloquent-murdock-6d3fa3` · PR **#8** (base `main`).

---

## 0. Environment & conventions (READ FIRST)

- **Host:** macOS. **CI:** Linux → beware `#[cfg(target_os=...)]` dead_code that
  only appears on Linux (a macOS-only fn unused on Linux warns in CI). Gate such
  fns with the same `cfg`.
- **Build/test (NEVER run `cargo` directly):**
  - `./deploy.sh --test` → `cargo test --workspace --offline`. Cold build ~25 min;
    **run it backgrounded** and wait for completion.
  - `./deploy.sh` → build + install + **restart daemon live at `http://localhost:9091`**.
    The daemon serves `~/.voxi/data/web` (the INSTALLED copy), so **frontend edits
    in `data/web/` only go live after `./deploy.sh`**.
  - UI-test with `curl http://localhost:9091/...` (GET only). NEVER point the
    Claude *preview* tool at 9091 — it kills the daemon. The sandbox blocks
    localhost; run curl with the sandbox disabled.
- **Commits:** write message to `.tmp/commit_msg.txt`, then
  `git commit -F .tmp/commit_msg.txt`. Title ≤50 chars, body lines ≤80, English,
  no `feat:`/`fix:` prefix. End body with:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`. Push:
  `git push origin claude/eloquent-murdock-6d3fa3` (updates PR #8). `gh` is NOT
  installed; create PRs via the GitHub REST API using the git credential helper.
- **Rules:** zero build warnings; no `.unwrap()` in production paths; cross-
  platform (no Linux-only `/proc`); `Send+Sync` on async types; add/adjust a
  `tests/system/*.json` scenario BEFORE changing daemon-visible behavior, then
  validate with `voxi-tests`.

## 1. Codebase map (key files & wiring points)

| Concern | Location |
|---|---|
| Agent hot loop | `src/voxi/src/core/agent_core/process_prompt.rs` (4,735 LOC). Loop at `loop {` ~L1720 (cancellation check at top). `loop_state` created L662. **Tool dispatch chain** of `else if tc_name == "X" {…}` starts ~L3018 (web_search L3021, web_fetch added after it, remember/recall follow). SafetyGuard tool gate `check_tool_call` ~L2104; `self.safety_guard.lock()` pattern ~L2117. |
| Loop state | `src/voxi/src/core/agent_loop_state.rs`. `snapshot()`/`restore_from()`, `DEFAULT_MAX_TOOL_ROUNDS=10`, `ABSOLUTE_MAX_TOOL_ROUNDS=100`, `needs_compaction()`, `token_budget=256k`, `compact_threshold=0.90`, `recent_outputs` (idle window 3). |
| Snapshot persist/load | `src/voxi/src/core/agent_core/runtime_admin_impl.rs`: `persist_loop_snapshot` (atomic JSON write to `runtime_topology().loop_state_dir`), `load_loop_snapshot`. Also `search_memory`, `run_subagents`, `get_session_store`, `get_llm_config` live here. |
| Memory | `src/voxi/src/storage/memory_store.rs`: SQLite + Markdown, `search_semantic`, `rank_by_cosine`, `decode_embedding_blob`, `MEMORY_EMBEDDING_MODEL="all-MiniLM-L6-v2"`, `MEMORY_VECTOR_SCAN_LIMIT=256`. Embeddings: `src/voxi/src/core/on_device_embedding.rs` (ONNX, 384-dim, raw `dlopen` + manual `unsafe impl Send/Sync` — an F3-style cleanup candidate). |
| Tools | impl in `src/voxi/src/core/feature_tools.rs` (`web_search`, `web_fetch`+SSRF guard, `extract_document_text`); declarations in `src/voxi/src/core/tool_declaration_builder.rs` (`push_research_tools`, `push_document_tools`); dispatch in `process_prompt.rs`. HTTP client: `crate::generic::infra::http_client::default_client()` (reqwest). |
| Secure tool runner | `src/voxi-tool-executor/src/main.rs` (separate daemon; where OS-level sandboxing belongs). |
| Parallel fan-out | `src/voxi/src/core/parallel.rs` (`run_parallel_bounded`). `run_subagents` (runtime_admin_impl). IPC `spawn_subagents`. |
| IPC | `src/voxi/src/core/ipc_server.rs`: dispatch `match method {` ~L376. `rt_handle.block_on` + `tokio::task::block_in_place` for async methods; `Self::resolve_session_id`. |
| Dashboard | separate binary `src/voxi-web-dashboard/src/main.rs`. Routes ~L307 (`Router::new().route(...)`). `ipc_call(method, params)` proxies to daemon. `validate_token` for auth. Frontend: `data/web/{index.html,app.js,style.css}`; `escHtml` (XSS-safe), `apiFetch`, cache-bust `?v=YYYYMMDDx` (bump on every FE change). |
| Canonical runtime | `rust/crates/vclaw-*` (`policy_engine`, `permission_enforcer`, `mcp_*`). Target of the two-runtime migration. |

---

## TASK A — Tiered memory + compaction (ADR-0005)  · L · highest leverage
**Goal:** working→episodic→long-term tiers with auto-compaction + hybrid retrieval.
Voxi already has the vector half (`search_semantic`) and a token guard
(`needs_compaction()`); add the *lifecycle*.

**Steps:**
1. `tests/system/tiered_memory_runtime_contract.json` first.
2. In `agent_loop_state.rs` confirm `needs_compaction()` (token_used/budget ≥
   compact_threshold). In `process_prompt.rs` main loop, when `loop_state.needs_compaction()`,
   call a new `compact_session()` BEFORE the next LLM call.
3. `compact_session()` (new, in `agent_core`): take the oldest N messages (keep
   last ~20 — mirror Hermes `protect_last_n`), summarize them via a cheap LLM
   backend call, and **flush durable facts to the Markdown memory tier**
   (`MemoryStore::set(key,value,category)`) BEFORE truncating the transcript, so
   nothing is lost (OpenClaw "memory-flush"). Index the flushed facts (they get
   embedded by the existing backfill path).
4. Hybrid retrieval: add an FTS5/BM25 query in `memory_store.rs` next to
   `search_semantic`, fuse with Reciprocal-Rank-Fusion (RRF: score = Σ 1/(k+rank),
   k≈60). New `search_hybrid(query, top_k)`.
5. Tiers: add a `tier` column (working/episodic/longterm) to the `memories`
   table (`ensure_column`), default `longterm`; `core` facts always injected.
**Done when:** long sessions stay under budget; durable facts survive compaction;
`search_hybrid` beats keyword-only on a fixture. **Validate:** `./deploy.sh --test`.

## TASK B — Self-verification loop (ADR-0006)  · M · hot-path, gate it
**Goal:** after the model signals completion, run a bounded self-critique pass.
**Steps:**
1. `tests/system/self_verification_runtime_contract.json` first (a deliberately
   incomplete first answer triggers exactly ONE corrective iteration).
2. In `process_prompt.rs`, at the loop's completion branch (where it transitions
   to `ResultReporting`/`Complete`), if a config flag `verify_enabled` is on and
   `verify_iterations < K` (default K=1): issue a structured critique prompt
   ("does this satisfy the request? what's missing?"); if it finds a fixable gap,
   continue the loop one more round; else finalize. MUST be bounded (reuse
   `max_tool_rounds`/`ABSOLUTE_MAX_TOOL_ROUNDS`) and config-gated (off by default
   for latency-sensitive channels).
3. Optional: a post-write validator (Hermes pattern) — after a file-write tool,
   parse JSON/YAML/TOML or run `rustc --error-format=json`, inject results back as
   a synthetic tool-result message.
**Done when:** verification adds ≤K iterations, is off by default, scenario passes.

## TASK C — Execution sandbox + skill scanner (ADR-0002)  · L · security
**Goal:** OS-level isolation for untrusted tool execution; pre-install skill scan.
**Steps:**
1. Define `trait ToolSandbox: Send + Sync { async fn run(&self, spec) -> Result; }`
   with `Isolation { None, Subprocess, Namespace, Container }`. Put it in
   `voxi-tool-executor` (the runner already centralizes execution).
2. `Subprocess` backend first: child process + `rlimit` (CPU/mem/fds via `libc`
   or `rlimit` crate) + scrubbed env + cwd jail. Then `Container` (Docker via
   `bollard` crate) feature-gated; **degrade to deny-untrusted where Docker
   absent** (cross-platform rule).
3. Per-session trust tier → backend selection, sourced from `SafetyGuard`/
   `policy_engine` (ties to Task G). Untrusted channel sessions → sandboxed.
4. Skill scanner: new `core/skill_scanner.rs` run at skill-load — regex/AST
   heuristics for prompt-injection, command-injection, hardcoded secrets, exfil
   URLs; quarantine failing skills. Hook into `skill_capability_manager`.
5. `tests/system/sandbox_*_runtime_contract.json`: an escaped command cannot
   touch host FS outside the jail.

## TASK D — ARIA browser tool  · L · depends on Task C
**Goal:** interactive web (forms/JS) via accessibility-tree snapshots, not pixels.
**Steps:**
1. Add a CDP driver (`chromiumoxide` or `headless_chrome` crate) in
   `voxi-tool-executor`. Emit a compact ARIA snapshot (`button "Sign In" [ref=1]`,
   ~<50KB) as the observation; actions `navigate(url)`, `click(ref)`,
   `type(ref,text)`.
2. Declare a `browser` tool in `tool_declaration_builder.rs`; dispatch in
   `process_prompt.rs`; run INSIDE the sandbox (Task C) and gate via SafetyGuard
   allowlist + the `web_fetch` SSRF host rules (reuse `is_blocked_fetch_host`).
**Note:** pairs with the existing `web_fetch` (non-interactive baseline, done).

## TASK E — Reliability: remove hot-path `unwrap`, supervise tasks (ADR-0007) · L
**Goal:** a corrupt file must not panic-abort the daemon (`panic="abort"` in
root `Cargo.toml`). ~446 `.unwrap()` in prod paths.
**Steps (do per-module, smallest blast radius first):**
1. Storage first: `storage/session_store.rs` (29), `storage/memory_store.rs` (38)
   — convert `.lock().unwrap()`/parse `.unwrap()` to `Result`/graceful skip;
   feed corrupt/empty files in unit tests and assert `Err`/skip, not panic.
2. Then `clawhub_client.rs` (88), `skill_capability_manager.rs` (45),
   `tool_dispatcher.rs` (22). Use `thiserror` enums.
3. Add a clippy gate: `#![deny(clippy::unwrap_used, clippy::expect_used)]` for
   `src/voxi/src` non-test (introduce gradually behind per-module `#[allow]`
   burned down as you sweep).
4. Supervision: run each channel + the agent loop as supervised `tokio` tasks so
   a panic restarts only that subtree (then revisit `panic="abort"`).
**Done when:** the gated modules have zero `unwrap` and tests prove graceful
failure on bad input.

## TASK F — Split the `process_prompt` god-file (F4)  · L · pure refactor
**Goal:** break `process_prompt.rs` (4,735 LOC) into submodules: `intake`,
`tool_loop`, `grounding`, `finalize`, behind a thin orchestrator. **No behavior
change** — validate with the existing `tests/system/*.json` + full `--test`.
Do it AFTER Task B (so the verification loop lands in a testable `tool_loop`).

## TASK G — Two-runtime migration (ADR-0001)  · L · ongoing
**Goal:** make `rust/crates/vclaw-*` the source of truth; unify safety via its
`policy_engine`. **First beachhead:** define `trait ToolAuthorizer` in
`vclaw-api`, implement in `policy_engine`, and route legacy
`SafetyGuard::check_tool_call` through it (legacy `SafetyGuard` becomes one impl).
Then per-subsystem cutover checklist in `.dev_note/DASHBOARD.md`.

---

## Smaller / quick items
- **Model panel write path:** extend the dashboard `/api/llm` route with a POST
  to `set_llm_config` + a provider/model `<select>` in `data/web/` (read path +
  GET already shipped). Confirm/guard switching the active backend.
- **`document-ops` / `local-data-analysis` skills:** only author the SKILL.md
  AFTER the underlying tools exist (PDF: `extract_document_text` already covers
  read; add a DuckDB/SQL-over-files tool first). Do NOT author skills for missing
  tools.
- **ONNX loader cleanup:** migrate `on_device_embedding.rs` raw `dlopen` +
  `unsafe impl Send/Sync` to `libloading` (same pattern as the F3 plugin fix,
  commit `695415a`).
- **Make backend features live:** run `./deploy.sh` so `web_fetch`,
  checkpoint-resume, `spawn_subagents`, prompt-injection logging, and the model
  panel are active in the running daemon (dashboard UI already redeployed once).

## Suggested order
0. `./deploy.sh` (make shipped backend live) →
1. **E** storage-unwrap slice (cheap reliability) →
2. **A** tiered memory (highest leverage, reuses semantic search) →
3. **B** self-verification →
4. **C** sandbox → **D** browser →
5. **F** god-file split → **G** two-runtime migration.

Each task = its own branch/PR, `./deploy.sh --test` green, scenario added,
committed per the rules in §0.
