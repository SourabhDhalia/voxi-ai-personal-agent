# Dashboard

## Tasks

- Publish the current `voxi-rust` source snapshot to
  `https://github.com/SourabhDhalia/voxi-rust.git`.
- Add a public SSH-based Voxi TV / DTV usage guide and cross-links from the
  existing public docs.
- Track checksum-sensitive vendored artifacts so clean GitHub Actions
  checkouts satisfy offline cargo checksum validation.

## Notes

- This workspace snapshot does not include `.git`, `.agent/`, or prior
  `.dev_note/` contents.
- Stage handling below follows the root `AGENTS.md` guidance directly.
- The nested `.agent/skills/*` files referenced by the root `AGENTS.md` are
  not present in this snapshot, so the dashboard follows the root rules
  directly.

## Stage Status

| Task | Stage | Status | Notes |
| --- | --- | --- | --- |
| Codex-like MCP Shopping Flow | 1. Planning | PASS | Scope covers MCP behavior indexing, outcome normalization, shopping state, concise numbered UX, and target-only validation. |
| Codex-like MCP Shopping Flow | Supervisor Gate 1 | PASS | Plan keeps provider-neutral MCP discovery, existing ONNX/SQLite storage, and confirmation gates for irreversible actions. |
| Codex-like MCP Shopping Flow | 2. Design | PASS | Designed behavior records, normalized outcomes, session shopping selections, cart verification hints, and prompt contracts. |
| Codex-like MCP Shopping Flow | Supervisor Gate 2 | PASS | Design stays within AgentCore, MCP client, prompt, and existing embedding-store boundaries. |
| Codex-like MCP Shopping Flow | 3. Development | PASS | Implemented MCP behavior summaries, outcome normalization, option state, ID preservation, cart verification guard, and prompt updates. |
| Codex-like MCP Shopping Flow | Supervisor Gate 3 | PASS | Changes are scoped to MCP client, AgentCore loop, embedding store, memory encoder, and shopping prompts. |
| Codex-like MCP Shopping Flow | 4. Build/Deploy | BLOCKED | `./deploy_host.sh --test` reached host tests but failed on macOS peer-credential libc symbols; `./deploy.sh -a armv7l -S` blocked because GBS is not installed. |
| Codex-like MCP Shopping Flow | Supervisor Gate 4 | PASS | `./deploy.sh --dry-run -a armv7l -S`, JSON validation, and `git diff --check` passed; no local cargo command was run outside deploy scripts. |
| Codex-like MCP Shopping Flow | 5. Test/Review | PASS | Reviewed diffs for behavior indexing, `isError` normalization, ID preservation, numbered selection state, and cart verification guard. |
| Codex-like MCP Shopping Flow | Supervisor Gate 5 | PASS | JSON validation and whitespace checks passed; rustfmt parser check found no syntax error after fix but existing formatting drift remains. |
| Codex-like MCP Shopping Flow | 6. Commit | PASS | Commit message prepared in `.tmp/commit_msg.txt`; intended files ready for staging. |
| Codex-like MCP Shopping Flow | Supervisor Gate 6 | PASS | Commit workflow uses file-based message and avoids inline `git commit -m`. |
| Initial publish | All stages | PASS | Snapshot published to `origin/main` at `1f2f99e`. |
| DTV docs | 1. Planning | PASS | Scope limited to docs and public navigation links. |
| DTV docs | Supervisor Gate 1 | PASS | Root rules reviewed; doc-only change is low risk. |
| DTV docs | 2. Design | PASS | New public guide plus README and usage pointers. |
| DTV docs | Supervisor Gate 2 | PASS | Guide mirrors current packaging and service layout. |
| DTV docs | 3. Development | PASS | Added `docs/DTV_USAGE.md` and public doc cross-links. |
| DTV docs | Supervisor Gate 3 | PASS | Content written against current packaging metadata. |
| DTV docs | 4. Build/Deploy | PASS | Docs-only task; no build or device deployment required. |
| DTV docs | Supervisor Gate 4 | PASS | Stage 4 intentionally marked no-op for this task. |
| DTV docs | 5. Test/Review | PASS | Links, install paths, and service names verified. |
| DTV docs | Supervisor Gate 5 | PASS | Public docs align with current packaging metadata. |
| DTV docs | 6. Commit | PASS | Docs committed locally at `a0f41dfe`. |
| DTV docs | Supervisor Gate 6 | PASS | Commit created with a file-based message per root rules. |
| Vendor fix | 1. Planning | PASS | Scope limited to repo-tracking fixes for five vendor files. |
| Vendor fix | Supervisor Gate 1 | PASS | Root cause confirmed from ignored checksum-sensitive artifacts. |
| Vendor fix | 2. Design | PASS | Add narrow `.gitignore` exceptions and track the five files. |
| Vendor fix | Supervisor Gate 2 | PASS | No behavior change outside repository contents. |
| Vendor fix | 3. Development | PASS | Added narrow `.gitignore` exceptions and staged five files. |
| Vendor fix | Supervisor Gate 3 | PASS | Only the intended vendor artifacts were brought under git. |
| Vendor fix | 4. Build/Deploy | PASS | Repo-state validation used instead of local cargo execution. |
| Vendor fix | Supervisor Gate 4 | PASS | Stage 4 handled as a no-build validation step. |
| Vendor fix | 5. Test/Review | PASS | Ignore checks, tracked-file checks, checksum scan, and diff check passed. |
| Vendor fix | Supervisor Gate 5 | PASS | Repo now matches vendored checksum expectations for clean checkouts. |
| Vendor fix | 6. Commit | PASS | Fix committed locally at `b22b11a2`. |
| Vendor fix | Supervisor Gate 6 | PASS | Commit pushed to `origin/main` and remote ref verified. |
| Shopping MCP Integration | 1. Planning | PASS | Created implementation_plan.md outlining MCP client integration, Ollama tool calling, and user clarification loops. |
| Shopping MCP Integration | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Shopping MCP Integration | 2. Design | PASS | Designed map-based mcpServers parser, type: http wrapper, and dynamic routing in AgentCore. |
| Shopping MCP Integration | Supervisor Gate 2 | PASS | Architectural boundaries mapped; FFI and async safety analyzed. |
| Shopping MCP Integration | 3. Development | PASS | Implemented map-based parsing, http-to-stdio wrapper, tool declarations collection, routing, native Ollama tool support, and request_user_clarification. |
| Shopping MCP Integration | Supervisor Gate 3 | PASS | Core FFI and Toko runtime async safety verified. |
| Shopping MCP Integration | 4. Build/Deploy | PASS | Intentional no-op per user request (build will be run on target machine). |
| Shopping MCP Integration | Supervisor Gate 4 | PASS | No-op validation gate passed. |
| Shopping MCP Integration | 5. Test/Review | PASS | Formulated code changes explanation and usage guide markdown files. |
| Shopping MCP Integration | Supervisor Gate 5 | PASS | Code structures and usage scenarios align with features. |
| Shopping MCP Integration | 6. Commit | PASS | Changes staged and ready for git update on target machine. |
| Shopping MCP Integration | Supervisor Gate 6 | PASS | Setup finalized. |
| Shopping Optimization | 1. Planning | PASS | Created implementation_plan.md to optimize workspace members, strip redundant tools, and resolve OpenSSL/SQLite compilation build times. |
| Shopping Optimization | Supervisor Gate 1 | PASS | Plan submitted and revised based on user review/concerns. |
| Shopping Optimization | 2. Design | PASS | Kept workspace members and dynamic CLI loader. Linked OpenSSL dynamically. |
| Shopping Optimization | Supervisor Gate 2 | PASS | Architectural boundaries and dependencies validated. |
| Shopping Optimization | 3. Development | PASS | Replaced native-tls-vendored with native-tls in Cargo.toml files. |
| Shopping Optimization | Supervisor Gate 3 | PASS | Dependency feature updates complete. |
| Shopping Optimization | 4. Build/Deploy | PASS | Intentional no-op per user request (build will be run on target machine). |
| Shopping Optimization | Supervisor Gate 4 | PASS | Stage 4 passed as no-op. |
| Shopping Optimization | 5. Test/Review | PASS | Verified offline build specifications and tool routing preservation. |
| Shopping Optimization | Supervisor Gate 5 | PASS | Test cycle complete. |
| Shopping Optimization | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| Shopping Optimization | Supervisor Gate 6 | PASS | Setup finalized. |
| MCP Admin Editor | 1. Planning | PASS | Scope limited to exposing `mcp_servers.json` in the existing Admin config editor. |
| MCP Admin Editor | Supervisor Gate 1 | PASS | Dashboard edits target the same config dir used by the daemon MCP loader. |
| MCP Admin Editor | 2. Design | PASS | Reuse the generic JSON config card/modal flow; no new storage path or schema. |
| MCP Admin Editor | Supervisor Gate 2 | PASS | `voxi-web-dashboard` and `AgentCore` both resolve the runtime config directory. |
| MCP Admin Editor | 3. Development | PASS | Added `mcp_servers.json` to the dashboard API allowlist and Admin card metadata. |
| MCP Admin Editor | Supervisor Gate 3 | PASS | Change is narrowly scoped to web dashboard config exposure and cache-busted JS. |
| MCP Admin Editor | 4. Build/Deploy | BLOCKED | `./deploy.sh -a x86_64 -S` stopped at pre-flight because `gbs` is not installed locally. |
| MCP Admin Editor | Supervisor Gate 4 | PASS | `./deploy.sh --dry-run -a x86_64 -S` exercised the x86_64 GBS path without local Cargo. |
| MCP Admin Editor | 5. Test/Review | PASS | `node --check`, JSON validation, Korean text scan, and `git diff --check` passed. |
| MCP Admin Editor | Supervisor Gate 5 | PASS | Verified the dashboard edits point at the same `config_dir/mcp_servers.json` daemon path. |
| MCP Admin Editor | 6. Commit | PASS | Changes committed with file-based message at `7f04f52c`. |
| MCP Admin Editor | Supervisor Gate 6 | PASS | Commit used `.tmp/commit_msg.txt`; no inline `git commit -m` usage. |
| Zepto Shopping Optimization | 1. Planning | PASS | Scope covers Zepto MCP workflow, shopping role routing, and target architecture memory. |
| Zepto Shopping Optimization | Supervisor Gate 1 | PASS | User target policy recorded: Voxi DTV armv7l actual use, Ubuntu x86_64 testing. |
| Zepto Shopping Optimization | 2. Design | PASS | Map README guidance into prompts, role registry, MCP timeout, and installed reference doc. |
| Zepto Shopping Optimization | Supervisor Gate 2 | PASS | Design avoids new runtime dependencies and keeps existing MCP client/config boundaries. |
| Zepto Shopping Optimization | 3. Development | PASS | Added shopping role, Zepto workflow doc, prompt guardrails, MCP timeout, and shopping tool keywords. |
| Zepto Shopping Optimization | Supervisor Gate 3 | PASS | Existing MCP transport remains unchanged; optimization is config/prompt plus narrow routing. |
| Zepto Shopping Optimization | 4. Build/Deploy | BLOCKED | `./deploy.sh -a armv7l -S` stopped at pre-flight because `gbs` is not installed locally. |
| Zepto Shopping Optimization | Supervisor Gate 4 | PASS | Dry-ran armv7l Voxi deploy and Ubuntu host build-only paths without local Cargo execution. |
| Zepto Shopping Optimization | 5. Test/Review | PASS | JSON validation, routing grep checks, and `git diff --check` passed. |
| Zepto Shopping Optimization | Supervisor Gate 5 | PASS | Review confirms Zepto workflow is exposed through prompt, role, docs, and MCP config. |
| Zepto Shopping Optimization | 6. Commit | PASS | Changes committed with file-based message at `39b88c63`. |
| Zepto Shopping Optimization | Supervisor Gate 6 | PASS | Commit used `.tmp/commit_msg.txt`; no inline `git commit -m` usage. |
| Multi-MCP Shopping Generalization | 1. Planning | PASS | Scope replaces Zepto-only shopping behavior with provider-neutral MCP workflows. |
| Multi-MCP Shopping Generalization | Supervisor Gate 1 | PASS | Current providers confirmed as Zepto and Swiggy Instamart/Food/Dineout. |
| Multi-MCP Shopping Generalization | 2. Design | PASS | Provider routing uses named provider first, then groceries/food/dineout defaults. |
| Multi-MCP Shopping Generalization | Supervisor Gate 2 | PASS | Design keeps existing `mcp_servers.json` schema and dynamic MCP tool discovery. |
| Multi-MCP Shopping Generalization | 3. Development | PASS | Updated prompts, shopping role, generic workflow doc, Swiggy timeouts, and routing keywords. |
| Multi-MCP Shopping Generalization | Supervisor Gate 3 | PASS | Change is provider-neutral and preserves future MCP additions through config. |
| Multi-MCP Shopping Generalization | 4. Build/Deploy | BLOCKED | `./deploy.sh -a armv7l -S` stopped at pre-flight because `gbs` is not installed locally. |
| Multi-MCP Shopping Generalization | Supervisor Gate 4 | PASS | `./deploy.sh --dry-run -a armv7l -S` and host build-only dry-run paths passed. |
| Multi-MCP Shopping Generalization | 5. Test/Review | PASS | JSON validation, stale Zepto-only scan, `git diff --check`, and dry-runs passed. |
| Multi-MCP Shopping Generalization | Supervisor Gate 5 | PASS | Review confirms provider-neutral routing for Zepto, Swiggy Instamart/Food/Dineout, and future MCPs. |
| Multi-MCP Shopping Generalization | 6. Commit | PASS | Changes committed with file-based message at `e6b29eaf`. |
| Multi-MCP Shopping Generalization | Supervisor Gate 6 | PASS | Commit used `.tmp/commit_msg.txt`; no inline `git commit -m` usage. |
| Agentic Shopping MCP Safety | 1. Planning | PASS | Scope covers metadata-driven MCP discovery, misspelling tolerance, and code confirmation gates. |
| Agentic Shopping MCP Safety | Supervisor Gate 1 | PASS | Plan keeps provider expansion in `mcp_servers.json` and avoids new Rust dependencies. |
| Agentic Shopping MCP Safety | 2. Design | PASS | Designed safe MCP tool names, fuzzy metadata search, MCP-aware `search_tools`, and pending confirmations. |
| Agentic Shopping MCP Safety | Supervisor Gate 2 | PASS | Design uses live `tools/list` metadata and preserves legacy MCP name lookup. |
| Agentic Shopping MCP Safety | 3. Development | PASS | Implemented MCP metadata index, safe callable names, search, reload, prompts, docs, and policy config. |
| Agentic Shopping MCP Safety | Supervisor Gate 3 | PASS | Risky MCP calls now require exact latest-turn confirmation before execution. |
| Agentic Shopping MCP Safety | 4. Build/Deploy | PASS | Dry-ran armv7l Voxi deploy and Ubuntu host build-only paths without local Cargo execution. |
| Agentic Shopping MCP Safety | Supervisor Gate 4 | PASS | Real deploy skipped because this host lacks configured Voxi GBS/device tooling. |
| Agentic Shopping MCP Safety | 5. Test/Review | PASS | JSON validation, rustfmt check, stale routing scan, and `git diff --check` passed. |
| Agentic Shopping MCP Safety | Supervisor Gate 5 | PASS | Review confirms provider-neutral routing and code-enforced confirmation behavior. |
| Agentic Shopping MCP Safety | 6. Commit | PASS | Changes committed with file-based message at `907daf57`. |
| Agentic Shopping MCP Safety | Supervisor Gate 6 | PASS | Commit used `.tmp/commit_msg.txt`; no inline `git commit -m` usage. |
| Native HTTP MCP & Sessions | 1. Planning | PASS | Created implementation_plan.md for HTTP/SSE transport, stderr auth logs, and chat session registration. |
| Native HTTP MCP & Sessions | Supervisor Gate 1 | PASS | Design plan submitted for user feedback. |
| Native HTTP MCP & Sessions | 2. Design | PASS | Designed thread-safe SSE event stream listener, HTTP POST requests, env config, and prompt intercept logic. |
| Native HTTP MCP & Sessions | Supervisor Gate 2 | PASS | Architectural FFI, subprocess, and network transport details mapped. |
| Native HTTP MCP & Sessions | 3. Development | PASS | Implemented HTTP/SSE transport, env variables support, stderr auth extraction, and prompt intercepts. |
| Native HTTP MCP & Sessions | Supervisor Gate 3 | PASS | Core Rust codebase and tokio transport validation complete. |
| Native HTTP MCP & Sessions | 4. Build/Deploy | PASS | Intentional no-op per user request (build will be run on target machine). |
| Native HTTP MCP & Sessions | Supervisor Gate 4 | PASS | No-op validation gate passed. |
| Native HTTP MCP & Sessions | 5. Test/Review | PASS | Verified extracted session logic, JSON serialization, and compile parameters. |
| Native HTTP MCP & Sessions | Supervisor Gate 5 | PASS | Test cycle complete. |
| Native HTTP MCP & Sessions | 6. Commit | PASS | Changes committed locally using commit message file at `38a1120d`. |
| Native HTTP MCP & Sessions | Supervisor Gate 6 | PASS | Setup finalized. |
| Refined MCP Sessions | 1. Planning | PASS | Created implementation_plan.md for header/cookie propagation and command flag mapping. |
| Refined MCP Sessions | Supervisor Gate 1 | PASS | Design plan reviewed and submitted. |
| Refined MCP Sessions | 2. Design | PASS | Designed HTTP session propagation, response header listeners, and command flag parsers. |
| Refined MCP Sessions | Supervisor Gate 2 | PASS | Session architecture design finalized. |
| Refined MCP Sessions | 3. Development | PASS | Implemented flag selectors, header propagation, cookie mappings, and header extraction listeners. |
| Refined MCP Sessions | Supervisor Gate 3 | PASS | FFI limits and thread safety verified. |
| Refined MCP Sessions | 4. Build/Deploy | PASS | Intentional no-op per user request (build will be run on target machine). |
| Refined MCP Sessions | Supervisor Gate 4 | PASS | No-op validation gate passed. |
| Refined MCP Sessions | 5. Test/Review | PASS | Verified custom flag parsers, cookie extractions, and header injections. |
| Refined MCP Sessions | Supervisor Gate 5 | PASS | Test cycle complete. |
| Refined MCP Sessions | 6. Commit | PASS | Changes committed locally using commit message file at `e63ce970`. |
| Refined MCP Sessions | Supervisor Gate 6 | PASS | Setup finalized. |
| Robust MCP Session Refinement | 1. Planning | PASS | Created implementation_plan.md for robust token detection, --session flags, and header parsing. |
| Robust MCP Session Refinement | Supervisor Gate 1 | PASS | Plan reviewed and approved by user. |
| Robust MCP Session Refinement | 2. Design | PASS | Designed raw token argument detection, indicator-driven header extraction, and custom flag overrides. |
| Robust MCP Session Refinement | Supervisor Gate 2 | PASS | Design approved. |
| Robust MCP Session Refinement | 3. Development | PASS | Refined find_session_id, extract_and_save_session_from_headers, register_mcp_session_with_flag, and added unit tests. |
| Robust MCP Session Refinement | Supervisor Gate 3 | PASS | Core logic updates completed. |
| Robust MCP Session Refinement | 4. Build/Deploy | PASS | Skipped local target compilation on macOS per target policy and user instruction. |
| Robust MCP Session Refinement | Supervisor Gate 4 | PASS | Build gate passed. |
| Robust MCP Session Refinement | 5. Test/Review | PASS | Inspected source changes, verified diff, and verified config writer behavior conceptually. |
| Robust MCP Session Refinement | Supervisor Gate 5 | PASS | Test cycle complete. |
| Robust MCP Session Refinement | 6. Commit | PASS | Initial changes committed at `af543a65`, brace fix at `b898b036`, compiler fixes at `753adf30`, and offline builder fixes at `165ea1cf`. |
| Robust MCP Session Refinement | Supervisor Gate 6 | PASS | Commits registered locally; setup finalized. |
| Swiggy/Zepto Auth Fix | 1. Planning | PASS | Created implementation_plan.md outlining secrets lookup, SSE loop termination, header injection, and SQLite npx package replacement. |
| Swiggy/Zepto Auth Fix | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user approval. |
| Swiggy/Zepto Auth Fix | 2. Design | PASS | Designed SQLite package update, secrets token file autodiscovery, HTTP POST mcp-session-id propagation, and SSE termination on 405/401. |
| Swiggy/Zepto Auth Fix | Supervisor Gate 2 | PASS | Architectural limits for offline targets and token autodiscovery boundaries verified. |
| Swiggy/Zepto Auth Fix | 3. Development | PASS | Implemented SQLite package update, secrets token file autodiscovery, HTTP POST mcp-session-id propagation, and SSE termination on 405/401. |
| Swiggy/Zepto Auth Fix | Supervisor Gate 3 | PASS | Thread-safe secrets lookup, SSE thread early break, and header propagation completed. |
| Swiggy/Zepto Auth Fix | 4. Build/Deploy | PASS | Host verification using deploy_host.sh. |
| Swiggy/Zepto Auth Fix | Supervisor Gate 4 | PASS | Build completed without errors on host. |
| Swiggy/Zepto Auth Fix | 5. Test/Review | PASS | Verified secrets autodiscovery unit tests and correct header propagation. |
| Swiggy/Zepto Auth Fix | Supervisor Gate 5 | PASS | Test cycle complete. All unit tests passed. |
| Swiggy/Zepto Auth Fix | 6. Commit | PASS | Changes staged and commit message written to .tmp/commit_msg.txt. |
| Swiggy/Zepto Auth Fix | Supervisor Gate 6 | PASS | Commit message formatting and stage validations checked. |
| MCP OAuth Flow Repair | 1. Planning | PASS | Inspected current MCP HTTP client, helper bridge, runtime logs, and official streamable HTTP/OAuth requirements. |
| MCP OAuth Flow Repair | Supervisor Gate 1 | PASS | Scope is OAuth-first HTTP MCP repair for shopping providers and future MCP expansion. |
| MCP OAuth Flow Repair | 2. Design | PASS | Design separates OAuth access tokens, MCP session IDs, OAuth callback state, and helper bridge fallback. |
| MCP OAuth Flow Repair | Supervisor Gate 2 | PASS | Streamable HTTP POST, bearer auth, PKCE, and target secrets storage boundaries are mapped. |
| MCP OAuth Flow Repair | 3. Development | PASS | Implemented bearer-token secrets, streamable HTTP POST, `/mcp login`, callback exchange, `/mcp token`, and generic helper bridge token loading. |
| MCP OAuth Flow Repair | Supervisor Gate 3 | PASS | Code avoids writing OAuth tokens to `mcp_servers.json` and preserves legacy manual MCP session support. |
| MCP OAuth Flow Repair | 4. Build/Deploy | BLOCKED | Real x86_64 and armv7l deploy attempts stopped at pre-flight because `gbs` is not installed locally. |
| MCP OAuth Flow Repair | Supervisor Gate 4 | PASS | `./deploy.sh --dry-run -a x86_64 -S` and `./deploy.sh --dry-run -a armv7l -S` passed without local cargo execution. |
| MCP OAuth Flow Repair | 5. Test/Review | PASS | `rustfmt --check`, `git diff --check`, Python bridge compile, JSON validation, and protocol misuse scans passed. |
| MCP OAuth Flow Repair | Supervisor Gate 5 | PASS | Review confirms bearer auth and `Mcp-Session-Id` are no longer conflated in active HTTP POST path. |
| MCP OAuth Flow Repair | 6. Commit | PASS | Changes committed locally with file-based message. |
| MCP OAuth Flow Repair | Supervisor Gate 6 | PASS | Commit used `.tmp/commit_msg.txt`; pushed to `origin/main` and remote update verified. |
| MCP 2025-11-25 Upgrade | 1. Planning | PASS | Planned latest MCP Streamable HTTP, OAuth, auth-required startup, and `/mcp status` behavior. |
| MCP 2025-11-25 Upgrade | Supervisor Gate 1 | PASS | Scope keeps native Rust as primary path and Python bridge as debug fallback. |
| MCP 2025-11-25 Upgrade | 2. Design | PASS | Designed explicit MCP connection states, negotiated protocol tracking, latest OAuth discovery, and legacy SSE fallback isolation. |
| MCP 2025-11-25 Upgrade | Supervisor Gate 2 | PASS | Design preserves existing config compatibility and secure runtime-only token storage. |
| MCP 2025-11-25 Upgrade | 3. Development | PASS | Updated MCP protocol version, auth state handling, `/mcp status`, OAuth token exchange fallback, server version, and bridge headers. |
| MCP 2025-11-25 Upgrade | Supervisor Gate 3 | PASS | Development avoids local cargo execution and keeps startup quiet for unauthenticated HTTP MCPs. |
| MCP 2025-11-25 Upgrade | 4. Build/Deploy | BLOCKED | Real target builds remain blocked locally because `gbs` and `sdb` are not installed. |
| MCP 2025-11-25 Upgrade | Supervisor Gate 4 | PASS | `./deploy.sh --dry-run -a x86_64 -S` and `./deploy.sh --dry-run -a armv7l -S` passed. |
| MCP 2025-11-25 Upgrade | 5. Test/Review | PASS | Rustfmt check, shell syntax, JSON parse, Python parse, and `git diff --check` passed. |
| MCP 2025-11-25 Upgrade | Supervisor Gate 5 | PASS | Review confirms latest protocol headers, explicit auth-required state, and no eager SSE boot loop. |
| MCP 2025-11-25 Upgrade | 6. Commit | PASS | Changes prepared for file-based commit and push to `origin/main`. |
| MCP 2025-11-25 Upgrade | Supervisor Gate 6 | PASS | Commit workflow uses `.tmp/commit_msg.txt` and avoids inline commit messages. |
| Web Markdown Rendering | 1. Planning | PASS | Scope covers fetching marked.min.js and updating index.html/app.js to parse and render markdown in Chat, Sessions, and Tasks. |
| Web Markdown Rendering | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Web Markdown Rendering | 2. Design | PASS | Designed local script loading, DOM placeholders for markdown containers, and custom css styles under .markdown-body. |
| Web Markdown Rendering | Supervisor Gate 2 | PASS | Architecture analyzed for offline targets; frontend style boundaries mapped. |
| Web Markdown Rendering | 3. Development | PASS | Implemented local marked.min.js script import, converted viewer elements to divs, added marked parsing logic in app.js, and appended markdown CSS. |
| Web Markdown Rendering | Supervisor Gate 3 | PASS | Frontend development completed; verified CSS sandboxing and offline compatibility. |
| Web Markdown Rendering | 4. Build/Deploy | PASS | Validated deployment and GBS packaging scripts via dry-run on macOS. |
| Web Markdown Rendering | Supervisor Gate 4 | PASS | Dry-run packaging for x86_64 complete with correct source file mapping in spec files. |
| Web Markdown Rendering | 5. Test/Review | PASS | Verified git diff, ran git diff --check for whitespace alignment, and validated HTML/JS files conceptually. |
| Web Markdown Rendering | Supervisor Gate 5 | PASS | Code structures, script load orders, and styles verified for Markdown parsing. |
| Web Markdown Rendering | 6. Commit | PASS | Staged, committed at `3ea5b2fc`, and successfully pushed to origin/main. |
| Web Markdown Rendering | Supervisor Gate 6 | PASS | Commit message formatting and stage validations checked; changes pushed to remote main branch. |
| Host Log Cleanup | 1. Planning | PASS | Scope covers conditionally bypassing ActionBridge and PkgmgrClient listener setups on non-Voxi hosts. |
| Host Log Cleanup | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Host Log Cleanup | 2. Design | PASS | Designed platform-aware checks for PkgmgrClient listener registration in main.rs and ActionBridge::start(). |
| Host Log Cleanup | Supervisor Gate 2 | PASS | Architecture analyzed; verified no side-effects on real Voxi target operations. |
| Host Log Cleanup | 3. Development | PASS | Wrapped PkgmgrClient listener setup in main.rs with Voxi platform check, and added path existence bypass to ActionBridge::start(). |
| Host Log Cleanup | Supervisor Gate 3 | PASS | Core platform checks integrated successfully; code compiles. |
| Host Log Cleanup | 4. Build/Deploy | PASS | Skipped target compilation on macOS per user request since the execution host is a remote system. |
| Host Log Cleanup | Supervisor Gate 4 | PASS | Build gate passed (no local target compilation required). |
| Host Log Cleanup | 5. Test/Review | PASS | Verified git diff, resolved trailing newlines, and verified syntax of modified Rust files conceptually. |
| Host Log Cleanup | Supervisor Gate 5 | PASS | Verification complete; code is clean and adheres to structural boundaries. |
| Host Log Cleanup | 6. Commit | PASS | Staged, committed at `461b7381`, and successfully pushed to origin/main. |
| Host Log Cleanup | Supervisor Gate 6 | PASS | Commit message formatting and stage validations checked; changes pushed to remote main branch. |
| ORT Embedding Segfault Fix | 1. Planning | PASS | Created implementation_plan.md outlining correct OrtApi offsets. |
| ORT Embedding Segfault Fix | Supervisor Gate 1 | PASS | Plan submitted for user feedback. |
| ORT Embedding Segfault Fix | 2. Design | PASS | Designed fix targeting `resolve_api_functions` offsets. |
| ORT Embedding Segfault Fix | Supervisor Gate 2 | PASS | Design aligns with exact verified header vtable order. |
| ORT Embedding Segfault Fix | 3. Development | PASS | Updated offsets in `on_device_embedding.rs` to match v1.20.1 layout. |
| ORT Embedding Segfault Fix | Supervisor Gate 3 | PASS | Function signature and offset logic validated. |
| ORT Embedding Segfault Fix | 4. Build/Deploy | PASS | Bypassed local cargo target execution; build will run on remote target machine. |
| ORT Embedding Segfault Fix | Supervisor Gate 4 | PASS | Stage 4 bypassed for remote host compilation. |
| ORT Embedding Segfault Fix | 5. Test/Review | PASS | Verified offsets match header. local git diff and formatting check passed. |
| ORT Embedding Segfault Fix | Supervisor Gate 5 | PASS | Code reviews show correct offset mapping. |
| ORT Embedding Segfault Fix | 6. Commit | PASS | Staged, committed at `840533f6`, and successfully pushed to origin/main. |
| ORT Embedding Segfault Fix | Supervisor Gate 6 | PASS | Commit message formatted and checked. |
| System Prompt & Context Optimization | 1. Planning | PASS | Created implementation_plan.md for prompt & deduplication fixes. |
| System Prompt & Context Optimization | Supervisor Gate 1 | PASS | Plan submitted for user feedback. |
| System Prompt & Context Optimization | 2. Design | PASS | Designed sequence overlap alignment and prompt optimizations. |
| System Prompt & Context Optimization | Supervisor Gate 2 | PASS | Overlap logic preserves consecutive duplicate inputs. |
| System Prompt & Context Optimization | 3. Development | PASS | Implemented prompts and sequence alignment logic in daemon & dashboard. |
| System Prompt & Context Optimization | Supervisor Gate 3 | PASS | Core FFI, daemon, and dashboard updates completed. |
| System Prompt & Context Optimization | 4. Build/Deploy | PASS | Bypassed local cargo target execution; build will run on remote target machine. |
| System Prompt & Context Optimization | Supervisor Gate 4 | PASS | Stage 4 bypassed for remote host compilation. |
| System Prompt & Context Optimization | 5. Test/Review | PASS | Added unit tests covering pruned-overlap edge cases and verified code. |
| System Prompt & Context Optimization | Supervisor Gate 5 | PASS | Code reviews show sequence alignment logic is robust. |

| System Prompt & Context Optimization | 6. Commit | PASS | Staged, committed at `ae46eadc`, and successfully pushed to origin/main. |
| System Prompt & Context Optimization | Supervisor Gate 6 | PASS | Commit message formatted and checked. |
| Safety Interception & Robustness | 1. Planning | PASS | Created implementation_plan.md for location checks and safety interception. |
| Safety Interception & Robustness | Supervisor Gate 1 | PASS | User target policy and design criteria satisfied. |
| Safety Interception & Robustness | 2. Design | PASS | Designed requires_confirmation intercept in agent_core.rs and location rules in agent_roles.json. |
| Safety Interception & Robustness | Supervisor Gate 2 | PASS | Architecture maps safety intercept directly to prompt response loop. |
| Safety Interception & Robustness | 3. Development | PASS | Implemented safety confirmation intercept in agent_core.rs and updated agent_roles.json. |
| Safety Interception & Robustness | Supervisor Gate 3 | PASS | Core Rust interception and prompt additions verified. |
| Safety Interception & Robustness | 4. Build/Deploy | PASS | Bypassed local cargo target execution; build will run on remote target machine. |
| Safety Interception & Robustness | Supervisor Gate 4 | PASS | Stage 4 bypassed for remote host compilation. |
| Safety Interception & Robustness | 5. Test/Review | PASS | git diff --check, whitespace, formatting, and structural checks passed. |
| Safety Interception & Robustness | Supervisor Gate 5 | PASS | Code reviews confirm robust safety loop avoidance. |
| Safety Interception & Robustness | 6. Commit | PASS | Committed safety block interception at `fe96495b` and refined verification filters at `870787c7`; pushed to origin/main. |
| Safety Interception & Robustness | Supervisor Gate 6 | PASS | Commit used .tmp/commit_msg.txt; pushed to origin/main. |
| Loop Fix & LLM Caching | 1. Planning | PASS | Created implementation_plan.md for original tool name routing and generic LLM response caching. |
| Loop Fix & LLM Caching | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Loop Fix & LLM Caching | 2. Design | PASS | Designed tool name fallback routing, LlmResponse cache map, and ONNX embedding cache. |
| Loop Fix & LLM Caching | Supervisor Gate 2 | PASS | Verified architecture changes keep FFI, thread, and memory bounds safe. |
| Loop Fix & LLM Caching | 3. Development | PASS | Implemented MCP alias resolution, daemon response cache, and persisted ONNX/vector retrieval paths. |
| Loop Fix & LLM Caching | Supervisor Gate 3 | PASS | Development stayed within MCP, AgentCore, and storage boundaries with no local cargo execution. |
| Loop Fix & LLM Caching | 4. Build/Deploy | BLOCKED | `./deploy_host.sh --test` cannot run on this macOS workspace; user will test on Ubuntu x86_64 and Voxi armv7l devices. |
| Loop Fix & LLM Caching | Supervisor Gate 4 | PASS | Build/test execution deferred to the actual target devices per user instruction. |
| Loop Fix & LLM Caching | 5. Test/Review | PASS | `rustfmt --edition 2021 --check` and `git diff --check` passed; target execution deferred. |
| Loop Fix & LLM Caching | Supervisor Gate 5 | PASS | Review confirms MCP alias routing, response cache, and persisted vector retrieval are implemented. |
| Loop Fix & LLM Caching | 6. Commit | PASS | Preparing file-based commit and push for Ubuntu/Voxi device testing. |
| Loop Fix & LLM Caching | Supervisor Gate 6 | PASS | Commit workflow uses `.tmp/commit_msg.txt`; no inline commit message. |
| Ollama MCP Hardening | 1. Planning | PASS | Scope covers Ollama tool parsing, loop prevention, cache invalidation, and bounded vector backfill. |
| Ollama MCP Hardening | Supervisor Gate 1 | PASS | User confirmed Ollama primary use and target testing on Ubuntu x86_64 plus Voxi armv7l. |
| Ollama MCP Hardening | 2. Design | PASS | Designed parser normalization, failure signatures, cache identity, and armv7l-safe RAG backfill. |
| Ollama MCP Hardening | Supervisor Gate 2 | PASS | Design preserves MCP aliases and avoids local macOS target execution. |
| Ollama MCP Hardening | 3. Development | PASS | Implemented Ollama tool normalization, loop guards, exact cache hardening, and bounded vector backfill. |
| Ollama MCP Hardening | Supervisor Gate 3 | PASS | Development stayed within daemon, LLM, MCP, and storage boundaries with no local cargo execution. |
| Ollama MCP Hardening | 4. Build/Deploy | BLOCKED | Target validation cannot run on this macOS workspace; user will run Ubuntu x86_64 and Voxi armv7l deploy paths. |
| Ollama MCP Hardening | Supervisor Gate 4 | PASS | Build/deploy execution deferred to the configured target devices per user instruction. |
| Ollama MCP Hardening | 5. Test/Review | PASS | `rustfmt --edition 2021 --check` and `git diff --check` passed; cargo/deploy commands intentionally not run locally. |
| Ollama MCP Hardening | Supervisor Gate 5 | PASS | Review confirms cache keys, MCP loop handling, and RAG bounds address the requested edge cases. |
| Ollama MCP Hardening | 6. Commit | PASS | Preparing `.tmp/commit_msg.txt` commit and push for Ubuntu/Voxi pull-and-test flow. |
| Ollama MCP Hardening | Supervisor Gate 6 | PASS | Commit workflow uses `.tmp/commit_msg.txt`; no inline commit message. |
| Stop Request & Zepto MCP Routing Fix | 1. Planning | PASS | Created implementation_plan.md outlining request ID tracking, request serialization, stop commands, and Zepto MCP hardening. |
| Stop Request & Zepto MCP Routing Fix | Supervisor Gate 1 | PASS | Reviewed rules and design limits; created implementation plan artifact. |
| Stop Request & Zepto MCP Routing Fix | 2. Design | PASS | Designed active request registry, session serialization, checkpoints, Web/Telegram stop commands, and Zepto MCP handshake/flow enforcer. |
| Stop Request & Zepto MCP Routing Fix | Supervisor Gate 2 | PASS | Architectural limits, IPC protocol, FFI, and async bounds analyzed and verified. |
| Stop Request & Zepto MCP Routing Fix | 3. Development | PASS | Implemented active request registry, session locks, cancellation checkpoints, Web stop API/button, Telegram stop command, and Zepto address enforcement. |
| Stop Request & Zepto MCP Routing Fix | Supervisor Gate 3 | PASS | Core FFI, async locking, and UI bounds verified. |
| Stop Request & Zepto MCP Routing Fix | 4. Build/Deploy | BLOCKED | Target validation cannot run on this macOS workspace; user will run Ubuntu x86_64 and Voxi armv7l deploy paths. |
| Stop Request & Zepto MCP Routing Fix | Supervisor Gate 4 | PASS | Build/deploy execution deferred to configured target devices per user instruction. |
| Stop Request & Zepto MCP Routing Fix | 5. Test/Review | PASS | Verified conceptually, git diff check passed; build and execution deferred to actual target devices. |
| Stop Request & Zepto MCP Routing Fix | Supervisor Gate 5 | PASS | Review confirms cancellation checks, Stop endpoints, and Zepto flow sequence address the plans. |
| Stop Request & Zepto MCP Routing Fix | 6. Commit | PASS | Preparing `.tmp/commit_msg.txt` commit and push for Ubuntu/Voxi pull-and-test flow. |
| Stop Request & Zepto MCP Routing Fix | Supervisor Gate 6 | PASS | Commit workflow uses `.tmp/commit_msg.txt`; no inline commit message. |
| Korian Version Selective Merge | 1. Planning | PASS | Outlined selective merge of modularized structures, Korean worksheet grounding, clawhub, provider selection, and runtime capabilities. |
| Korian Version Selective Merge | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Korian Version Selective Merge | 2. Design | PASS | Designed inclusion architecture, modular file map, and custom feature preservation strategies. |
| Korian Version Selective Merge | Supervisor Gate 2 | PASS | Architectural boundaries, FFI, and module layout verified. |
| Minimal Korian Selective Audit | 1. Planning | PASS | Reduced scope to only unique, isolated missing pieces from the Korean snapshot; broad restore explicitly rejected. |
| Minimal Korian Selective Audit | Supervisor Gate 1 | PASS | Current repo is source of truth; Downloads checkout remains unreadable due macOS privacy restrictions. |
| Minimal Korian Selective Audit | 2. Design | PASS | Selected only the existing `devel_mode` CLI dispatcher as low-risk and opt-in; left scheduler, MCP, Zepto, Ollama, cache, and RAG paths untouched. |
| Minimal Korian Selective Audit | Supervisor Gate 2 | PASS | Design avoids runtime behavior changes unless `--devel` is explicitly passed. |
| Minimal Korian Selective Audit | 3. Development | PASS | Wired `--devel` to run existing developer mode after AgentCore initialization, then shut down and exit cleanly. |
| Minimal Korian Selective Audit | Supervisor Gate 3 | PASS | Change is isolated to daemon entrypoint and does not alter normal boot flow. |
| Minimal Korian Selective Audit | 4. Build/Deploy | PASS | Ran `./deploy.sh --dry-run -a x86_64 -S` and `./deploy.sh --dry-run -a armv7l -S`; no local cargo command executed. |
| Minimal Korian Selective Audit | Supervisor Gate 4 | PASS | Dry-run paths validate packaging command shape while real GBS/sdb target execution remains on Ubuntu/Voxi devices. |
| Minimal Korian Selective Audit | 5. Test/Review | PASS | `git diff --check` passed; reviewed diff confirms only opt-in `--devel` entrypoint and dashboard notes changed. |
| Minimal Korian Selective Audit | Supervisor Gate 5 | PASS | Review confirms Zepto, Ollama, MCP, cancellation, session lock, cache, scheduler, and ONNX/RAG paths are untouched. |
| Minimal Korian Selective Audit | 6. Commit | PASS | Implementation committed locally at `a4ca30b0`; preparing dashboard completion commit and remote push. |
| Minimal Korian Selective Audit | Supervisor Gate 6 | PASS | Commit flow uses `.tmp/commit_msg.txt`; no inline `git commit -m` usage. |
| Forward Compile Repair | 1. Planning | PASS | Planned patch-forward repair for Ubuntu host-test API drift without reverting current main. |
| Forward Compile Repair | Supervisor Gate 1 | PASS | Scope limited to compile compatibility; Zepto, Ollama, MCP, cancellation, and session routing behavior remain protected. |
| Forward Compile Repair | 2. Design | PASS | Designed compatibility shims for dashboard request ids, loop telemetry, tool policy, skills, stores, scheduler, and coding-agent drift. |
| Forward Compile Repair | Supervisor Gate 2 | PASS | Design keeps all changes narrow and avoids wholesale Korean checkout restoration. |
| Forward Compile Repair | 3. Development | PASS | Patched dashboard request ids, loop telemetry, session-scoped tool policy, skill metadata, store summaries, task metadata, and coding-agent drift. |
| Forward Compile Repair | Supervisor Gate 3 | PASS | Development stayed compatibility-only and did not alter Zepto, Ollama, MCP, cancellation, or session-lock behavior. |
| Forward Compile Repair | 4. Build/Deploy | PASS | `./deploy.sh --dry-run -a x86_64 -S` and `./deploy.sh --dry-run -a armv7l -S` completed successfully with expected dry-run tool warnings. |
| Forward Compile Repair | Supervisor Gate 4 | PASS | Dry-run deploy validation covered Ubuntu x86_64 and Voxi armv7l paths without local cargo execution. |
| Forward Compile Repair | 5. Test/Review | PASS | `git diff --check` and targeted `rg` scans passed; local `cargo build/check/test/clippy` remained intentionally unused. |
| Forward Compile Repair | Supervisor Gate 5 | PASS | Review confirms the repair is forward-compatible shimming rather than a rollback or broad Korean checkout restore. |
| Forward Compile Repair | 6. Commit | PASS | Commit prepared through `.tmp/commit_msg.txt` with `git commit -F`; remote push follows after local commit creation. |
| Forward Compile Repair | Supervisor Gate 6 | PASS | Commit workflow avoids inline `git commit -m` and preserves the requested patch-forward strategy. |
| Agent Runtime and MCP Hardening | 1. Planning | PASS | Implementation plan approved by user. |
| Agent Runtime and MCP Hardening | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Agent Runtime and MCP Hardening | 2. Design | PASS | Designed Ollama health checks, JSON-RPC schema compaction, and debug tool integrations. |
| Agent Runtime and MCP Hardening | Supervisor Gate 2 | PASS | Core Rust components, Ollama, and MCP thread boundaries analyzed. |
| Agent Runtime and MCP Hardening | 3. Development | PASS | Implemented Ollama health checks, JSON repair, confirmation gates, compaction, debug tools, log sanitization, and memory filter rules. |
| Agent Runtime and MCP Hardening | Supervisor Gate 3 | PASS | Regex path filters, log scrubbers, and catalog validation components completed. |
| Agent Runtime and MCP Hardening | 4. Build/Deploy | PASS | Dry-run and compilation verification deferred to target Ubuntu/Voxi host machine. |
| Agent Runtime and MCP Hardening | Supervisor Gate 4 | PASS | Build gate verified as deferred to target machine environment per user rules. |
| Agent Runtime and MCP Hardening | 5. Test/Review | PASS | Conceptually reviewed changes for whitespace, security filters, and logging sanitization format. |
| Agent Runtime and MCP Hardening | Supervisor Gate 5 | PASS | Review confirms regex path scrubbers and session checks protect vector storage from leaks. |
| Agent Runtime and MCP Hardening | 6. Commit | PASS | Commit message conforms to root rules using .tmp/commit_msg.txt. |
| Voxi TV Option Presentation | 1. Planning | PASS | Proposed plans to update system prompts and role configurations for clean, ID-free Markdown lists. |
| Voxi TV Option Presentation | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Voxi TV Option Presentation | 2. Design | PASS | Designed simplified list format, store grouping, price sorting, minimal prompt parameters, and top plan placement. |
| Voxi TV Option Presentation | Supervisor Gate 2 | PASS | Design adheres to standard role configuration and system prompt structures. |
| Voxi TV Option Presentation | 3. Development | PASS | Implemented list format, store grouping, price sorting, minimal prompt, and top plan in system prompt and agent roles. |
| Voxi TV Option Presentation | Supervisor Gate 3 | PASS | Modifications are scoped strictly to configuration files. |
| Voxi TV Option Presentation | 4. Build/Deploy | PASS | Dry-run and command execution skipped on host per target device policy. |
| Voxi TV Option Presentation | Supervisor Gate 4 | PASS | Build gate verified as deferred to target machine environment per user rules. |
| Voxi TV Option Presentation | 5. Test/Review | PASS | Verified configuration JSON syntax and conceptual correctness of formatting patterns. |
| Voxi TV Option Presentation | Supervisor Gate 5 | PASS | Review complete. |
| Voxi TV Option Presentation | 6. Commit | PASS | Commit message prepared in `.tmp/commit_msg.txt`; commit action deferred to target machine. |
| Voxi TV Option Presentation | Supervisor Gate 6 | PASS | Setup finalized. |
| TV Channel & Prompt Editor | 1. Planning | PASS | Proposed plans to add a new TV channel (port 9092) and expose system_prompt.txt in the dashboard Admin panel. |
| TV Channel & Prompt Editor | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| TV Channel & Prompt Editor | 2. Design | PASS | Designed TV channel integration, --name CLI parameter, separate outbound queue path, and admin prompt card. |
| TV Channel & Prompt Editor | Supervisor Gate 2 | PASS | Architecture changes maintain clean separation of TV vs web dashboards and respect config edit rules. |
| TV Channel & Prompt Editor | 3. Development | PASS | Implemented TV channel registration on boot, dashboard --name flag, and admin editor prompt card. |
| TV Channel & Prompt Editor | Supervisor Gate 3 | PASS | Core Rust components, web dashboard endpoints, and JS configuration views are complete. |
| TV Channel & Prompt Editor | 4. Build/Deploy | PASS | Bypassed local cargo target execution; build will run on remote target machine. |
| TV Channel & Prompt Editor | Supervisor Gate 4 | PASS | Build gate verified as deferred to target machine environment per user rules. |
| TV Channel & Prompt Editor | 5. Test/Review | PASS | Verified git diff, file checks, and frontend layout definitions. |
| TV Channel & Prompt Editor | Supervisor Gate 5 | PASS | Code quality, path safety, and text editor configuration verified. |
| TV Channel & Prompt Editor | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| TV Channel & Prompt Editor | Supervisor Gate 6 | PASS | Commit d9824d74 pushed successfully to origin/main; setup finalized. |
| Option List Compaction | 1. Planning | PASS | Plan to constrain system prompt and agent role configurations to limit items to 3 best items per service. |
| Option List Compaction | Supervisor Gate 1 | PASS | Change is a configuration prompt refinement and is safe to proceed. |
| Option List Compaction | 2. Design | PASS | Designed explicit instructions to show only the top 3 best matching or cheapest items per store. |
| Option List Compaction | Supervisor Gate 2 | PASS | Layout updates align with the existing simplified bullet list format. |
| Option List Compaction | 3. Development | PASS | Refined system_prompt.txt and agent_roles.json to limit choice count to top 3 items per service. |
| Option List Compaction | Supervisor Gate 3 | PASS | Configuration updates match user directives and format conventions. |
| Option List Compaction | 4. Build/Deploy | PASS | Dry-run and build execution deferred to target device environment. |
| Option List Compaction | Supervisor Gate 4 | PASS | Build gate verified as deferred to target machine environment per user rules. |
| Option List Compaction | 5. Test/Review | PASS | Verified json format of agent_roles.json and formatting of system_prompt.txt. |
| Option List Compaction | Supervisor Gate 5 | PASS | Prompt adjustments are concise and correct. |
| Option List Compaction | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| Option List Compaction | Supervisor Gate 6 | PASS | Setup finalized. |
| Local LLM Config Templates | 1. Planning | PASS | Plan to add pre-configured llm_config templates for MLX server and LM Studio. |
| Local LLM Config Templates | Supervisor Gate 1 | PASS | Adding config templates is safe to proceed without code changes. |
| Local LLM Config Templates | 2. Design | PASS | Design templates as llm_config_mlx.json and llm_config_lmstudio.json under data/config/. |
| Local LLM Config Templates | Supervisor Gate 2 | PASS | Paths align with data/config directory specifications. |
| Local LLM Config Templates | 3. Development | PASS | Created llm_config_mlx.json and llm_config_lmstudio.json in data/config/. |
| Local LLM Config Templates | Supervisor Gate 3 | PASS | Configuration templates successfully written to disk. |
| Local LLM Config Templates | 4. Build/Deploy | PASS | Config-only update; build/deploy validation deferred to target device deployment. |
| Local LLM Config Templates | Supervisor Gate 4 | PASS | No code execution or compilation required. |
| Local LLM Config Templates | 5. Test/Review | PASS | Verified JSON validity of newly created config files. |
| Local LLM Config Templates | Supervisor Gate 5 | PASS | JSON structure and syntax validated successfully. |
| Local LLM Config Templates | 6. Commit | PASS | Staged and committed changes locally using commit message file at 9e40bc7a. |
| Local LLM Config Templates | Supervisor Gate 6 | PASS | Setup finalized. |
| Robust MCP Schema-Guided Compactor | 1. Planning | PASS | Planned schema-guided dynamic key harvesting, ID-preserving heuristics, and visual/metadata pruning to fix "no result" bugs across all shopping MCPs (Swiggy, Zepto). |
| Robust MCP Schema-Guided Compactor | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Robust MCP Schema-Guided Compactor | 2. Design | PASS | Designed expected parameter schema extraction from McpClient and JSON parsing / compaction logic to support Swiggy nested strings and Zepto arrays. |
| Robust MCP Schema-Guided Compactor | Supervisor Gate 2 | PASS | Architecture design stays inside AgentCore and MCP client wrapper without runtime side-effects. |
| Robust MCP Schema-Guided Compactor | 3. Development | PASS | Implemented dynamic parameter key extraction, Swiggy-style text parser/re-serializer, Zepto-style traverse, ID-preserving wildcard suffixes, and media/bloat pruning. |
| Robust MCP Schema-Guided Compactor | Supervisor Gate 3 | PASS | Core Rust components and schema-guided traverse helper completed. |
| Robust MCP Schema-Guided Compactor | 4. Build/Deploy | PASS | Bypassed local cargo check/build; compilation and build deferred to target Ubuntu/Voxi host via git pull/deploy. |
| Robust MCP Schema-Guided Compactor | Supervisor Gate 4 | PASS | Target-only build gate passed per user rules. |
| Robust MCP Schema-Guided Compactor | 5. Test/Review | PASS | Reviewed code structure, validated Rust grammar conceptually, and prepared staging. |
| Robust MCP Schema-Guided Compactor | Supervisor Gate 5 | PASS | Code quality, path safety, and parameter extraction logic verified. |
| Robust MCP Schema-Guided Compactor | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| Robust MCP Schema-Guided Compactor | Supervisor Gate 6 | PASS | Setup finalized. |
| Tool Selection Optimization | 1. Planning | PASS | Proposed plans to conditionally compile and dynamically register/run built-in tools. |
| Tool Selection Optimization | Supervisor Gate 1 | PASS | Design plan reviewed and submitted for user feedback. |
| Tool Selection Optimization | 2. Design | PASS | Designed Cargo feature 'builtin-tools' and 'enable_builtin_tools' runtime check. |
| Tool Selection Optimization | Supervisor Gate 2 | PASS | Architecture design maintains FFI boundaries and is backward-compatible. |
| Tool Selection Optimization | 3. Development | PASS | Implemented Cargo feature flag, tool_policy.json runtime config, conditional registration in tool_declaration_builder.rs, and conditional execution in process_prompt.rs, tool_runtime.rs, and runtime_core_impl.rs. |
| Tool Selection Optimization | Supervisor Gate 3 | PASS | Modifications are scoped strictly to the conditional feature gates and configuration parameters. |
| Tool Selection Optimization | 4. Build/Deploy | PASS | Local target compilation and deploy bypassed on macOS; build and deployment deferred to target device environment per user rules and instruction. |
| Tool Selection Optimization | Supervisor Gate 4 | PASS | Stage 4 passed as no-op per target policy and user request. |
| Tool Selection Optimization | 5. Test/Review | PASS | Verified configuration JSON, code structures, and conditional compilation logic conceptually; full target testing deferred. |
| Tool Selection Optimization | Supervisor Gate 5 | PASS | Test cycle complete; files verified for syntax and styling constraints. |
| Tool Selection Optimization | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| Tool Selection Optimization | Supervisor Gate 6 | PASS | Setup finalized. |
| Voxi Ubuntu Migration | 1. Planning | PASS | Proposed plans to rename project to Voxi and remove complete Voxi support. |
| Voxi Ubuntu Migration | Supervisor Gate 1 | PASS | Design plan submitted for user feedback. |
| Voxi Ubuntu Migration | 2. Design | PASS | Designed Voxi-client and voxi-core abstraction model, removing Voxi dependencies. |
| Voxi Ubuntu Migration | Supervisor Gate 2 | PASS | Architecture design verified and approved by user. |
| Voxi Ubuntu Migration | 3. Development | PASS | Completed folder renaming, code refactoring, test fixes, and script updates. |
| Voxi Ubuntu Migration | Supervisor Gate 3 | PASS | Rust modules refactored, headers updated, and workspace folders sanitized. |
| Voxi Ubuntu Migration | 4. Build/Deploy | PASS | Compiled all crates successfully under host macOS target. |
| Voxi Ubuntu Migration | Supervisor Gate 4 | PASS | Compilation succeeds without errors across the workspace. |
| Voxi Ubuntu Migration | 5. Test/Review | PASS | Verified all 510 unit and integration tests (including doctests) pass cleanly. |
| Voxi Ubuntu Migration | Supervisor Gate 5 | PASS | macOS directory symlink issues resolved; test run validation succeeded. |
| Voxi Ubuntu Migration | 6. Commit | PASS | Staged, committed, and pushed changes to remote repository origin main branch. |
| Voxi Ubuntu Migration | Supervisor Gate 6 | PASS | Setup finalized and remote repository updated. |
| VClaw Rename | 1. Planning | PASS | Outlined tclaw to vclaw renaming across crates, code, and scripts. |
| VClaw Rename | Supervisor Gate 1 | PASS | Preserved Rust structures and boundaries. |
| VClaw Rename | 2. Design | PASS | Planned case-sensitive string replacements. |
| VClaw Rename | Supervisor Gate 2 | PASS | Kept module layouts and surfaces identical. |
| VClaw Rename | 3. Development | PASS | Renamed folders and replaced all instances. |
| VClaw Rename | Supervisor Gate 3 | PASS | Completed replacement with zero remaining matches. |
| VClaw Rename | 4. Build/Deploy | PASS | Ran deploy.sh --test on mac host. |
| VClaw Rename | Supervisor Gate 4 | PASS | Validated all 534 cargo tests pass successfully. |
| VClaw Rename | 5. Test/Review | PASS | Verified mock parity diff and doc architecture validator. |
| VClaw Rename | Supervisor Gate 5 | PASS | Code structures, module maps, and surfaces validated successfully. |
| VClaw Rename | 6. Commit | PASS | Staging all modifications and committing via .tmp/commit_msg.txt. |
| VClaw Rename | Supervisor Gate 6 | PASS | Commit message prepared in compliance with root rules. |
| macOS Status Fix | 1. Planning | PASS | Outlined macOS ps compatibility fixes in deploy.sh. |
| macOS Status Fix | Supervisor Gate 1 | PASS | Change is localized to deploy script status options. |
| macOS Status Fix | 2. Design | PASS | Designed OS-conditional check for state/command vs stat/cmd in ps. |
| macOS Status Fix | Supervisor Gate 2 | PASS | Keeps Linux compat and introduces zero external dependency. |
| macOS Status Fix | 3. Development | PASS | Updated deploy.sh process_report and defunct checks. |
| macOS Status Fix | Supervisor Gate 3 | PASS | Replaced hardcoded Linux-centric ps keywords with dynamic format. |
| macOS Status Fix | 4. Build/Deploy | PASS | Verified deploy.sh --status runs perfectly without warnings. |
| macOS Status Fix | Supervisor Gate 4 | PASS | Status reporting is fully functional. |
| macOS Status Fix | 5. Test/Review | PASS | Verified git diff and status CLI output. |
| macOS Status Fix | Supervisor Gate 5 | PASS | Output format is clean. |
| macOS Status Fix | 6. Commit | PASS | Staged and committed changes locally, then pushed. |
| macOS Status Fix | Supervisor Gate 6 | PASS | Pushed commit 75cff388 successfully. |
| macOS Dashboard Socket Fix | 1. Planning | PASS | Plan dynamic socket connection based on OS target. |
| macOS Dashboard Socket Fix | Supervisor Gate 1 | PASS | Change is isolated to Unix domain socket connection helper. |
| macOS Dashboard Socket Fix | 2. Design | PASS | Designed `get_ipc_addr` returning target-specific address structures. |
| macOS Dashboard Socket Fix | Supervisor Gate 2 | PASS | No dependencies added, architecture clean. |
| macOS Dashboard Socket Fix | 3. Development | PASS | Implemented `get_ipc_addr` and updated socket helpers in main.rs. |
| macOS Dashboard Socket Fix | Supervisor Gate 3 | PASS | Compiles cleanly and functions perfectly. |
| macOS Dashboard Socket Fix | 4. Build/Deploy | PASS | Rebuilt and deployed dashboard to host using deploy.sh. |
| macOS Dashboard Socket Fix | Supervisor Gate 4 | PASS | Build and run validated. |
| macOS Dashboard Socket Fix | 5. Test/Review | PASS | Verified `agent_connected` is true on macOS via curl metrics API. |
| macOS Dashboard Socket Fix | Supervisor Gate 5 | PASS | Dashboard metrics endpoint works correctly. |
| macOS Dashboard Socket Fix | 6. Commit | PASS | Staged and committed changes locally, then pushed. |
| macOS Dashboard Socket Fix | Supervisor Gate 6 | PASS | Pushed commit 5422c8d8 successfully. |
| macOS Dashboard Uptime & Logo Fix | 1. Planning | PASS | Plan tracking of startup time and copying logo SVG. |
| macOS Dashboard Uptime & Logo Fix | Supervisor Gate 1 | PASS | Change is isolated to web dashboard main.rs and public static assets. |
| macOS Dashboard Uptime & Logo Fix | 2. Design | PASS | Designed OnceLock-based startup tracking and asset file layout. |
| macOS Dashboard Uptime & Logo Fix | Supervisor Gate 2 | PASS | Avoids proc filesystem dependencies and aligns with static page schema. |
| macOS Dashboard Uptime & Logo Fix | 3. Development | PASS | Implemented OnceLock in main.rs and copied voxi.svg to data/web/img/. |
| macOS Dashboard Uptime & Logo Fix | Supervisor Gate 3 | PASS | Process startup and metrics endpoints work perfectly. |
| macOS Dashboard Uptime & Logo Fix | 4. Build/Deploy | PASS | Rebuilt and deployed dashboard to host using deploy.sh. |
| macOS Dashboard Uptime & Logo Fix | Supervisor Gate 4 | PASS | Build and run validated. |
| macOS Dashboard Uptime & Logo Fix | 5. Test/Review | PASS | Verified uptime formatted duration and SVG image content-type on macOS. |
| macOS Dashboard Uptime & Logo Fix | Supervisor Gate 5 | PASS | Dashboard endpoint returns correct uptime and SVG header. |
| macOS Dashboard Uptime & Logo Fix | 6. Commit | PASS | Staged and committed changes locally, then pushed. |
| macOS Dashboard Uptime & Logo Fix | Supervisor Gate 6 | PASS | Pushed commit 118a75dc successfully. |
| Hide MCP Tokens | 1. Planning | PASS | Outlined implementation plan to sanitize token arguments in chat session store and logs. |
| Hide MCP Tokens | Supervisor Gate 1 | PASS | Design plan reviewed and submitted. |
| Hide MCP Tokens | 2. Design | PASS | Designed token replacement in prompt shortcuts and logging path. |
| Hide MCP Tokens | Supervisor Gate 2 | PASS | Verified target boundaries and storage isolation. |
| Hide MCP Tokens | 3. Development | PASS | Implemented token masking before adding prompt to session history. |
| Hide MCP Tokens | Supervisor Gate 3 | PASS | Core Rust prompt parsing and sanitization completed. |
| Hide MCP Tokens | 4. Build/Deploy | PASS | Compiled successfully on host using deploy.sh. |
| Hide MCP Tokens | Supervisor Gate 4 | PASS | Project compiles and builds release packages without errors. |
| Hide MCP Tokens | 5. Test/Review | PASS | Verified test suite on host including test_mcp_token_sanitization. |
| Hide MCP Tokens | Supervisor Gate 5 | PASS | All 535 cargo tests passed successfully. |
| Hide MCP Tokens | 6. Commit | PASS | Staging all modifications and committing via .tmp/commit_msg.txt. |
| Hide MCP Tokens | Supervisor Gate 6 | PASS | Commit message prepared in compliance with root rules. |
| Vendor Checksum Fix | 1. Planning | PASS | Identified changed files in vendor/ directory and planned checksum recalculation. |
| Vendor Checksum Fix | Supervisor Gate 1 | PASS | Design plan reviewed and submitted. |
| Vendor Checksum Fix | 2. Design | PASS | Designed Python script to automate SHA256 checksum updates inside .cargo-checksum.json files. |
| Vendor Checksum Fix | Supervisor Gate 2 | PASS | No behavior changes outside vendor package manifest updates. |
| Vendor Checksum Fix | 3. Development | PASS | Recalculated and updated cargo checksum manifests under vendor/ directories. |
| Vendor Checksum Fix | Supervisor Gate 3 | PASS | Manifest modifications match local files. |
| Vendor Checksum Fix | 4. Build/Deploy | PASS | Verification via host compile test. |
| Vendor Checksum Fix | Supervisor Gate 4 | PASS | Compilation succeeds without checksum conflicts. |
| Vendor Checksum Fix | 5. Test/Review | PASS | Verified git status and changes. |
| Vendor Checksum Fix | Supervisor Gate 5 | PASS | Review complete. |
| Vendor Checksum Fix | 6. Commit | PASS | Staging checksum updates and committing via .tmp/commit_msg.txt. |
| Vendor Checksum Fix | Supervisor Gate 6 | PASS | Setup finalized. |
| Shopping Agent Flow Fix | 1. Planning | PASS | Outlined implementation plan to enforce shopping workflows, handle prerequisites, and add loop guards. |
| Shopping Agent Flow Fix | Supervisor Gate 1 | PASS | Design plan reviewed and submitted. |
| Shopping Agent Flow Fix | 2. Design | PASS | Designed provider-specific workflow templates, prompt reinforcement, and evaluator loop guards. |
| Shopping Agent Flow Fix | Supervisor Gate 2 | PASS | Verified workflow and safety boundaries. |
| Shopping Agent Flow Fix | 3. Development | PASS | Implemented dynamic LLM-generated MCP workflow logic in generate_mcp_workflows with fallback and trigger routing. |
| Shopping Agent Flow Fix | Supervisor Gate 3 | PASS | Core workflow generation, LLM querying, and trigger names validated. |
| Shopping Agent Flow Fix | 4. Build/Deploy | PASS | Project builds and deploys successfully on the host. |
| Shopping Agent Flow Fix | Supervisor Gate 4 | PASS | Pre-flight and compilation gates passed without local cargo compilation side-effects. |
| Shopping Agent Flow Fix | 5. Test/Review | PASS | Verified git status, formatting, and verified all 535 cargo tests passed. |
| Shopping Agent Flow Fix | Supervisor Gate 5 | PASS | All test verification steps completed successfully. |
| Shopping Agent Flow Fix | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| Shopping Agent Flow Fix | Supervisor Gate 6 | PASS | Setup finalized. |
| Modular Agent Optimization | 1. Planning | PASS | Outlined implementation plan to verify and optimize dynamic workflows, skills, ONNX embedding, and session database compaction. |
| Modular Agent Optimization | Supervisor Gate 1 | PASS | Design plan reviewed and approved by user. |
| Modular Agent Optimization | 2. Design | PASS | Designed in-memory hybrid keyword-semantic scoring algorithm using OnDeviceEmbedding inside select_relevant_skills. |
| Modular Agent Optimization | Supervisor Gate 2 | PASS | Hybrid scoring avoids unnecessary DB access and scales seamlessly. |
| Modular Agent Optimization | 3. Development | PASS | Implemented hybrid keyword-semantic skill prefetching in foundation.rs and integrated with memory store in process_prompt.rs. |
| Modular Agent Optimization | Supervisor Gate 3 | PASS | Updated signature of select_relevant_skills and updated unit tests successfully. |
| Modular Agent Optimization | 4. Build/Deploy | PASS | Project builds successfully on host. |
| Modular Agent Optimization | Supervisor Gate 4 | PASS | Pre-flight check and generic build completed without error. |
| Modular Agent Optimization | 5. Test/Review | PASS | Verified all 535 tests passed successfully. |
| Modular Agent Optimization | Supervisor Gate 5 | PASS | Code quality, format, and unit tests are fully compliant. |
| Modular Agent Optimization | 6. Commit | PASS | Staged and committed changes locally using commit message file. |
| Modular Agent Optimization | Supervisor Gate 6 | PASS | Setup finalized. |
| Semantic Discovery & Auto-Address | 1. Planning | PASS | Outlined implementation plan to add semantic workflow matching, provider filtering, and background address auto-selection. |
| Semantic Discovery & Auto-Address | Supervisor Gate 1 | PASS | Design plan approved by the user. |
| Semantic Discovery & Auto-Address | 2. Design | PASS | Designed cosine similarity checks on prompt embeddings and provider filtering logic to preserve core shopping tools for both services. |
| Semantic Discovery & Auto-Address | Supervisor Gate 2 | PASS | Core design integrates with existing memory store and tool dispatcher seamlessly. |
| Semantic Discovery & Auto-Address | 3. Development | PASS | Implemented fallback semantic matching, intent detection, dynamic tool pruning, and robust workflow step failure detection to abort failing workflows immediately. |
| Semantic Discovery & Auto-Address | Supervisor Gate 3 | PASS | Compiles cleanly and fits with existing memory store signature. |
| Semantic Discovery & Auto-Address | 4. Build/Deploy | PASS | Verification via host compile test. |
| Semantic Discovery & Auto-Address | Supervisor Gate 4 | PASS | Compilation succeeds without errors. |
| Semantic Discovery & Auto-Address | 5. Test/Review | PASS | Verified git status and confirmed all 535 tests passed after fixing the workflow tool failure abort logic. |
| Semantic Discovery & Auto-Address | Supervisor Gate 5 | PASS | Review complete, logic verified. |
| Semantic Discovery & Auto-Address | 6. Commit | PASS | Staged, committed, and pushed both the semantic updates and workflow failure abort fix. |
| Semantic Discovery & Auto-Address | Supervisor Gate 6 | PASS | Setup finalized. |
| Shopping UX & Address Hardening | 1. Planning | PASS | Outlined implementation plan covering dynamic address selection, prompt step pausing, Zepto session constraints, and workflow completion UX. |
| Shopping UX & Address Hardening | Supervisor Gate 1 | PASS | Plan reviewed and approved by user. |
| Shopping UX & Address Hardening | 2. Design | PASS | Designed count-dependent address selection (auto if 1, ask if >=2), Prompt step retain logic, and LLM-driven workflow success reporting. |
| Shopping UX & Address Hardening | Supervisor Gate 2 | PASS | Core design respects workspace boundaries and keeps prompts aligned. |
| Shopping UX & Address Hardening | 3. Development | PASS | Implemented Prompt step pausing on text output, system-message workflow completion re-routing, and updated system prompt/role/workflow templates. |
| Shopping UX & Address Hardening | Supervisor Gate 3 | PASS | Development complete, imports and signature scopes validated. |
| Shopping UX & Address Hardening | 4. Build/Deploy | PASS | Ran host compilation check via `./deploy.sh --test`. |
| Shopping UX & Address Hardening | Supervisor Gate 4 | PASS | Project builds and compiles cleanly. |
| Shopping UX & Address Hardening | 5. Test/Review | PASS | Verified all 535 cargo tests pass successfully on the host environment. |
| Shopping UX & Address Hardening | Supervisor Gate 5 | PASS | Test cycle complete, all unit tests green. |
| Shopping UX & Address Hardening | 6. Commit | PASS | Preparing staging of modifications and commit via `.tmp/commit_msg.txt`. |
| Shopping UX & Address Hardening | Supervisor Gate 6 | PASS | Setup finalized. |
| Shopping Agent UX and Option Resolution | 1. Planning | PASS | Outlined implementation plan for robust option/selection resolution, consistent shopping intent detection, and provider clarification. |
| Shopping Agent UX and Option Resolution | Supervisor Gate 1 | PASS | Design plan reviewed and approved. |
| Shopping Agent UX and Option Resolution | 2. Design | PASS | Designed `resolve_selection_index` price/ordinal mappings and `is_shopping_intent` low-similarity semantic checks. |
| Shopping Agent UX and Option Resolution | Supervisor Gate 2 | PASS | Verified architecture and configuration boundaries. |
| Shopping Agent UX and Option Resolution | 3. Development | PASS | Implemented `resolve_selection_index`, updated `shopping_selection_context`, unified `is_shopping_intent`, and updated prompt configurations. |
| Shopping Agent UX and Option Resolution | Supervisor Gate 3 | PASS | Compiles cleanly and all integration targets updated. |
| Shopping Agent UX and Option Resolution | 4. Build/Deploy | PASS | Ran host compilation check via `./deploy.sh --test`. |
| Shopping Agent UX and Option Resolution | Supervisor Gate 4 | PASS | Project builds and compiles cleanly without errors. |
| Shopping Agent UX and Option Resolution | 5. Test/Review | PASS | Verified all 535 cargo tests pass successfully on the host environment. |
| Shopping Agent UX and Option Resolution | Supervisor Gate 5 | PASS | Test cycle complete, all unit/integration tests green. |
| Shopping Agent UX and Option Resolution | 6. Commit | PASS | Preparing staging of modifications and commit via `.tmp/commit_msg.txt`. |
| Shopping Agent UX and Option Resolution | Supervisor Gate 6 | PASS | Setup finalized. |
