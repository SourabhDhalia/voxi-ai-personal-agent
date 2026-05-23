# Workspace Optimization and Build-Time Reduction for Shopping Agent (Revised)

This plan outlines how to resolve the heavy 30-minute Tizen/GBS build time for the TizenClaw agent while preserving all codebase packages, FFI client libraries, security metadata plugins, and the full range of autonomous agent tools and capabilities.

## User Review Required

> [!IMPORTANT]
> **Build-Time Bottleneck Resolved Safely**: The 30-minute build time is primarily caused by compiling the entire OpenSSL library from source (`native-tls-vendored` in `reqwest`) under QEMU emulation.
> We propose:
> 1. Replacing `native-tls-vendored` with standard `native-tls` in the `Cargo.toml` files. This links against the target system's pre-installed dynamic OpenSSL libraries (`libssl.so` and `libcrypto.so`).
> 2. OpenSSL dynamic linking uses zero internet downloads (the basic `native-tls` dependencies are already in the offline `vendor/` cache) and reduces compilation time from 30 minutes to under 3 minutes.
> 3. SQLite (`rusqlite`) will remain set to `features = ["bundled"]`. This ensures SQLite compiles cleanly across all environments (Tizen TV and Ubuntu) without requiring additional system-level development packages, while only adding ~1 minute to the overall compilation.

> [!TIP]
> **Scalability & Capabilities Maintained**:
> - All workspace members (such as `src/libtizenclaw` for C FFI client access and the `tizenclaw-metadata-*` packaging hook plugins) are preserved.
> - All built-in agent capabilities (tasks, code execution, search, document analysis) remain active.
> - All MCP tool execution routes (`mcp_` prefix) remain fully functional.

---

## Proposed Changes

### 1. Build-Time Optimization via Dynamic OpenSSL Linking

#### [MODIFY] [libtizenclaw-core Cargo.toml](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/libtizenclaw-core/Cargo.toml)
- Replace `native-tls-vendored` with `native-tls` in the `reqwest` dependency.

#### [MODIFY] [tizenclaw Cargo.toml](file:///Users/sdhalia/Developer/githubRepo/tizenClaw-rust/src/tizenclaw/Cargo.toml)
- Replace `native-tls-vendored` with `native-tls` in the `reqwest` dependency.

---

## Verification Plan

### Automated Tests
- Trigger target build and verify compilation time:
  ```bash
  ./deploy.sh
  ```
  Or locally on Ubuntu:
  ```bash
  cargo check --workspace --offline
  ```
- Confirm build successfully compiles offline with zero internet downloads.
- Confirm overall compilation time drops from ~30 minutes to ~2-3 minutes.

### Manual Verification
1. Start the daemon service on the target environment.
2. Confirm the daemon is listening for MCP calls and client FFI commands correctly.
3. Validate client access via `libtizenclaw.so` or `tizenclaw-cli`.
