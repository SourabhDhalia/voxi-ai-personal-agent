# ADR-0002 — `ToolSandbox` trait with tiered isolation backends

**Status:** Proposed
**Gap:** P1 / F6 — no execution isolation (now table-stakes)
**Peer evidence:** Hermes Agent (local/Docker/SSH/Singularity/Modal + namespace
isolation); OpenClaw (NVIDIA NemoClaw secure runtime); Devin 2.0 (cloud VM).

## Context
Tools (shell, file, `system_cli_adapter`) run in the daemon's own process,
gated only by `SafetyGuard` (a policy filter, not isolation). For an always-on
agent that runs shell/file commands, the blast radius of a bad/poisoned tool
call is the whole device. Both the #1 and #3 peer agents ship isolation.

## Decision
Introduce a `ToolSandbox` trait with selectable backends, default to
least-privilege:

```rust
enum Isolation { None, Subprocess, Namespace, Container }
trait ToolSandbox: Send + Sync {
    fn run(&self, spec: &ToolExecSpec) -> ToolExecResult; // rlimits, cwd jail, env scrub
}
```

- `None` — host (dev only).
- `Subprocess` — child process + `rlimit` (CPU/mem/fds) + scrubbed env + cwd jail.
- `Namespace` — Linux user/mount/pid namespaces (cross-platform: `#[cfg(target_os="linux")]`, macOS falls back to `Subprocess`).
- `Container` — Docker/OCI for full isolation.

Sandbox selection is a capability granted by `policy_engine` (see ADR-0001),
so isolation and authorization share one model.

## Consequences
- (+) Bounds blast radius; matches peer table-stakes; composes with SafetyGuard.
- (−) Largest effort; per-OS backends; on-device profile must respect Voxi
  resource limits (coordinate with embedded-systems constraints).

## Validation
Add `tests/system/sandbox_*_runtime_contract.json` asserting a denied/escaped
command cannot touch the host FS outside the jail.
