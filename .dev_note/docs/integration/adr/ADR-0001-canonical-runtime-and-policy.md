# ADR-0001 — Canonicalize on `vclaw-*` runtime; unify safety via `policy_engine`

**Status:** Proposed
**Gaps:** F5 (two competing runtimes), F2 (safety), F6 (isolation)

## Context
Two runtimes coexist: legacy `src/voxi` (active, ~77k LOC) and canonical
`rust/crates/vclaw-*` (~15k LOC) which already contains `policy_engine`,
`permission_enforcer`, and MCP plumbing. There is no migration seam, so
policy/permission logic is duplicated and drifts. The hardened `SafetyGuard`
(F2) lives only in the legacy tree.

## Decision
1. Declare `vclaw-*` the **target** runtime; freeze net-new features in the
   legacy tree where a canonical equivalent exists.
2. Define an adapter trait boundary so legacy call sites delegate
   safety/permission decisions to the canonical `policy_engine` (capability
   tokens), rather than each tree owning its own checks.
3. First beachhead: route `SafetyGuard::check_tool_call` through
   `policy_engine` so there is one authority for tool authorization.

## Consequences
- (+) Single source of truth for safety; no drift; clear migration order.
- (+) Capability-token model generalizes to F6 sandbox scoping.
- (−) Requires an FFI/interface shim during the transition; incremental.

## Implementation sketch
- Add `trait ToolAuthorizer { fn authorize(&self, call) -> Decision }` in
  `vclaw-api`; implement in `policy_engine`.
- Legacy `agent_core` holds `Arc<dyn ToolAuthorizer>` instead of a bare
  `SafetyGuard`; `SafetyGuard` becomes one implementation behind the trait.
- Per-subsystem cutover checklist tracked in DASHBOARD.md.
