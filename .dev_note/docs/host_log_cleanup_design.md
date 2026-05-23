# Design Document — Host Log Cleanup

This document details the changes to bypass Tizen-specific service initializations on non-Tizen host environments, preventing error logs during Generic Linux testing.

## 1. Package Manager Listener Bypass (`main.rs`)

### Current Behavior
In `src/tizenclaw/src/main.rs`, the daemon unconditionally adds a `PkgmgrListener` via:
`PkgmgrClient::global().add_listener(...)`

This spawns a thread that attempts to run Tizen's DBus/Cynara package manager listener, which always fails on generic Linux and logs:
`[E] pkgmgr_client.rs:109 pkgmgr_client_new(PC_LISTENING) failed — DBus/cynara not ready or privilege missing`

### Proposed Change
Condition the listener registration in `main.rs` on the platform being "Tizen":
```rust
if platform.platform_name() == "Tizen" {
    PkgmgrClient::global().add_listener(Arc::new(AgentPkgmgrListener(agent.clone())));
} else {
    log::info!("Skipping Tizen package manager listener setup on generic Linux host");
}
```

## 2. Action Framework Client Bypass (`action_bridge.rs`)

### Current Behavior
In `src/tizenclaw/src/tizen/core/action_bridge.rs`, `ActionBridge::start()` unconditionally runs the Tizen Action Framework API `action_client_create(&mut state.client)`. This always fails on Generic Linux and logs:
`[E] action_bridge.rs:68 [TIZENCLAW] ActionBridge: failed to create action client: -1`

### Proposed Change
Add a platform check inside `ActionBridge::start()` using Tizen-specific file system markers. If they are absent, log a clean info message and return early:
```rust
if !std::path::Path::new("/etc/tizen-release").exists() && !std::path::Path::new("/opt/usr/share/tizenclaw").exists() {
    log::info!("Skipping ActionBridge start on non-Tizen platform");
    return false;
}
```
This avoids triggering the native client creation entirely.

## 3. ToolIndexer LLM Parse Fallback Log Level (`tool_indexer.rs`)

### Current Behavior
During startup, if the LLM's returned index JSON is malformed (e.g. contains minor formatting or truncation errors, which is common with small local models), `ToolIndexer::apply_llm_index_result` logs a noisy `log::error!` message:
`[E] tool_indexer.rs:687 ToolIndexer: Failed to parse LLM index response: ...`

Since this condition is fully anticipated and handled by a graceful fallback mechanism (which dynamically generates a clean templates catalog `tools.md` on disk), it should not be logged as a critical error.

### Proposed Change
Downgrade the log severity from `log::error!` to `log::warn!` and log the response snippet to aid debugging:
```rust
Err(e) => {
    log::warn!(
        "ToolIndexer: Failed to parse LLM index response: {} (response snippet: {:?})",
        e,
        clean.chars().take(200).collect::<String>()
    );
    return 0;
}
```
