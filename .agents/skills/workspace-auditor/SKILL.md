---
name: workspace-auditor
description: Audits a project workspace for layout drift, codebase anomalies, and health risks. Use when asked to review repository structure, find stale or dead artifacts, check config/doc consistency, or produce a workspace health report.
tags:
  - audit
  - workspace
  - health
  - repository
  - structure
triggers:
  - audit the workspace
  - check project layout
  - workspace health
  - find anomalies
  - review repository structure
examples:
  - "@workspace-auditor check the repo for stale configs"
  - "audit the workspace layout and report anomalies"
---

# Workspace Auditor

A focused reviewer that inspects the current project workspace and reports
structural drift, anomalies, and health risks. It reads and reasons; it does
not modify files unless explicitly asked.

## When to use

- The operator asks to audit, review, or health-check the repository.
- A change touched many files and you want a structural sanity pass.
- You suspect stale configs, dead artifacts, or doc/layout drift.

## Audit checklist

Work through these areas and report findings grouped by severity
(Critical / Warning / Info):

1. **Layout integrity**
   - Expected top-level dirs present (`src/`, `rust/`, `data/`, `tests/`,
     `docs/`, `.agents/`).
   - Orphaned build outputs committed by mistake (`target/`, `CMakeCache.txt`,
     stray `*.o`/`*.d`).
   - Temporary files outside `.tmp/`.

2. **Config consistency**
   - Every config under `data/config/` that the dashboard exposes is present
     in the dashboard `ALLOWED_CONFIGS` allowlist, and vice versa.
   - No placeholder secrets (e.g. `TOKEN_API`, empty keys) in active backends.
   - Referenced model/file paths exist.

3. **Docs & onboarding drift**
   - README / onboarding reading order matches the actual crate/dir layout.
   - New public concepts are reflected in both Rust and any parity surfaces.

4. **Dead / duplicate artifacts**
   - Modules declared (`mod x;`) but never referenced.
   - Duplicate or near-duplicate workflow/skill files.

5. **Test coverage gaps**
   - Daemon-visible behaviors lacking a `tests/system/*.json` scenario.

## Output format

Produce a concise report:

```
# Workspace Audit
## Critical
- <finding> — <path> — <why it matters>
## Warning
- ...
## Info
- ...
## Recommended next steps
1. ...
```

## Guardrails

- Read-only by default. Propose fixes; do not apply them unless asked.
- Respect repository rules: never trigger direct `cargo`/`cmake`; use the
  project's `./deploy.sh` workflow for any build/run validation.
- Keep findings actionable and path-anchored; avoid vague observations.
