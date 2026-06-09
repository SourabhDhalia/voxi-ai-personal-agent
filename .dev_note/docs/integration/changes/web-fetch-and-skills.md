# Internet capability (`web_fetch`) + new agent skills

**Status:** Implemented; build-verifying
**Files:** `src/voxi/src/core/feature_tools.rs`,
`src/voxi/src/core/tool_declaration_builder.rs`,
`src/voxi/src/core/agent_core/process_prompt.rs`,
`data/skills/web-research/SKILL.md`, `data/skills/summarize-and-extract/SKILL.md`
**Skill lens:** secure-code-guardian, rust-engineer

## Problem
Voxi already had `web_search` (Brave/Google/Gemini) but **no way to read a
URL's content** — the agent could find links but not open them. The
competitor deep-dive flagged autonomous web access as the #1 missing capability.

## Change — `web_fetch` tool
- `feature_tools::web_fetch(url, max_chars)` — HTTP(S) GET via the existing
  `http_client::default_client()`, HTML stripped to readable text, capped.
- **SSRF guard** (`is_blocked_fetch_host`): refuses non-HTTP schemes and
  loopback / private / link-local / unspecified / cloud-metadata hosts
  (`169.254.169.254`, `metadata.google.internal`, `localhost`, RFC1918, `::1`).
- `html_to_text`: regex tag/script/style strip + entity decode + whitespace
  collapse, with graceful passthrough if a pattern fails to compile (no panic).
- Declared in `push_research_tools` (alongside `web_search`) and dispatched in
  `process_prompt.rs`.
- **4 unit tests**: blocks internal hosts, allows public hosts, host extraction
  (incl. userinfo/port/IPv6), HTML stripping.

## Change — new skills (`data/skills/*/SKILL.md`)
- **`web-research`** — orchestrates `web_search` + `web_fetch` into cited answers.
- **`summarize-and-extract`** — long-content summary / structured extraction
  (no new tooling).

Both follow the existing agentskills.io-style frontmatter + workflow format.
`document-ops` / `local-data-analysis` were intentionally **not** authored yet —
their underlying tools (PDF/DuckDB) don't exist, so the skills would mislead.

## Note
This is the *non-interactive* baseline. The richer **ARIA-snapshot browser tool**
(OpenClaw pattern, via CDP/`chromiumoxide`) remains designed in the deep-dive —
it needs a sandbox first (ADR-0002).
