# Voxi Integration & Modernization — Documentation Set

This folder is the structured, multi-file documentation for the competitive
analysis, code fixes, API design, and integration plan produced for Voxi.
Each concern lives in its own file.

## Map

| Area | File | Status |
|------|------|--------|
| Full competitive analysis (5 OSS agents, 2026 refresh, real top-usage peers, flaws, plan) | [../oss-agent-integration-plan.md](../oss-agent-integration-plan.md) | Canonical analysis |
| Changelog of everything implemented this effort | [CHANGELOG.md](CHANGELOG.md) | Living |
| **Implemented changes** (one file each) | [`changes/`](changes/) | |
| ↳ F2 — SafetyGuard hardening | [changes/F2-safety-guard-hardening.md](changes/F2-safety-guard-hardening.md) | ✅ shipped + tested |
| ↳ P2 — OpenRouter LLM backend | [changes/P2-openrouter-backend.md](changes/P2-openrouter-backend.md) | ✅ shipped + tested |
| ↳ F3 — libloading FFI migration | [changes/F3-libloading-migration.md](changes/F3-libloading-migration.md) | ✅ shipped + tested |
| **API specifications** (OpenAPI 3.1, layered) | [`api/`](api/) | |
| ↳ Daemon JSON-RPC IPC contract | [api/ipc-jsonrpc.openapi.yaml](api/ipc-jsonrpc.openapi.yaml) | spec |
| ↳ Web dashboard HTTP API | [api/dashboard.openapi.yaml](api/dashboard.openapi.yaml) | spec |
| ↳ New integration REST API | [api/integration-api.openapi.yaml](api/integration-api.openapi.yaml) | spec (proposed) |
| **ADRs** for large architectural gaps | [`adr/`](adr/) | designed, not implemented |

## What's implemented vs designed

- **Implemented & verified this effort:** F2 (SafetyGuard hardening),
  P2 (OpenRouter backend), F3 (libloading FFI migration). All covered by unit
  tests and validated via `./deploy.sh --test`.
- **Designed only (ADRs + scaffolds):** execution sandbox (P1), skill
  self-authoring (P3), durable checkpoint (F8), tiered memory (F7),
  self-verification loop (G-A), `unwrap`/panic strategy (F1). These are larger
  initiatives captured as ADRs so they can be implemented deliberately and
  reviewed before landing.

## How to validate

```bash
./deploy.sh --test      # build + run workspace tests (offline)
```
See the project memory note: the host build/test entrypoint is `./deploy.sh`
(not `deploy_host.sh`), and a cold build is ~25 min.
