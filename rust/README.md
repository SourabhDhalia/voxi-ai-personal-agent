# Canonical Rust Workspace

This directory is the forward-looking production workspace for Voxi.

- `crates/vclaw-runtime` owns runtime orchestration
- `crates/vclaw-api` owns shared contracts
- `crates/vclaw-cli` owns the CLI surface
- `crates/vclaw-tools` owns tool abstractions
- `crates/vclaw-plugins` owns plugin boundaries

The legacy root Rust workspace remains available while the reconstruction
prompt series migrates functionality into this layout.
