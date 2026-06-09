# Changelog — Integration & Modernization Effort

All notable code changes from this effort. Dates are absolute. Each entry links
to its detailed change doc. Validation is via `./deploy.sh --test` (offline).

## 2026-06-08

### Added
- **OpenRouter LLM backend (P2).** New `openrouter` provider routed through the
  existing OpenAI-compatible path, unlocking 200+ models behind one key.
  Default model `openai/gpt-4o`, overridable via config.
  → [changes/P2-openrouter-backend.md](changes/P2-openrouter-backend.md)
- **5 new unit tests** for provider resolution and OpenRouter endpoint mapping.
- **10 new SafetyGuard regression tests** covering denylist bypass vectors.

### Changed
- **SafetyGuard hardened (F2).** Replaced the bypassable case-sensitive
  substring denylist with input normalization + a 13-pattern regex matcher +
  structured-arg blob scanning + an allowlist mode.
  → [changes/F2-safety-guard-hardening.md](changes/F2-safety-guard-hardening.md)
- **Plugin FFI migrated to `libloading` (F3).** `plugin_llm_backend.rs` no
  longer uses raw `libc::dlopen`/`dlsym`/`dlclose` + `std::mem::transmute_copy`.
  → [changes/F3-libloading-migration.md](changes/F3-libloading-migration.md)

### Removed
- **Hand-rolled `unsafe impl Send/Sync for PluginLlmBackend`** — now derived
  automatically because `libloading::Library` is `Send + Sync`.
- **`transmute_copy` of `dlsym` results** — replaced by typed
  `Library::get::<T>()`.

### Test status
- Before: 513 tests passing. After: **518 passing, 0 failing.** No new
  compiler warnings introduced by these changes.

### Not yet committed
- All changes live in the working tree on branch
  `claude/eloquent-murdock-6d3fa3`. Commit on request (repo rule: write message
  to `.tmp/commit_msg.txt`, then `git commit -F`).
