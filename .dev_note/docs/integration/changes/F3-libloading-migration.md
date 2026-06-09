# F3 — Plugin FFI Migration to `libloading`

**Status:** ✅ Implemented & test-verified
**File:** `src/voxi/src/llm/plugin_llm_backend.rs`
**Skill lens:** unsafe-checker, rust-engineer

## Problem
The dynamic LLM-plugin loader used raw FFI that violated the project's own rule
("symbols must be loaded via `libloading`") and was UB-prone:
- `libc::dlopen` / `libc::dlsym` / `libc::dlclose` / `libc::dlerror` directly.
- `std::mem::transmute_copy(&sym)` to cast a `*mut c_void` symbol into a typed
  function pointer — a classic footgun.
- Hand-written `unsafe impl Send for / Sync for PluginLlmBackend {}` asserting
  thread-safety on a struct holding a raw handle, with no compiler backing.

## Change
- Struct field `lib_handle: Option<*mut libc::c_void>` → `lib: Option<Library>`
  (`libloading::Library`, already a dependency `libloading = "0.8"`).
- `load_library()` uses `unsafe { Library::new(path) }` with graceful `Err`
  return (no panic when a `.so` is absent — preserves the "never panic if a
  native library is absent" rule).
- Symbol resolution uses typed `unsafe { lib.get::<FnType>(b"NAME\0") }` at each
  call site (`initialize`, `chat`, `shutdown`, name lookup). Removed the generic
  `resolve_symbol`/`resolve_symbol_static` helpers and the `transmute_copy`.
- `shutdown()` resolves `VOXI_LLM_BACKEND_SHUTDOWN`, calls it, then `drop(lib)`
  (equivalent to `dlclose`).
- **Deleted the manual `unsafe impl Send/Sync`** — `libloading::Library` is
  itself `Send + Sync` (verified in vendored `libloading/src/safe.rs`), so
  `PluginLlmBackend` derives those bounds automatically. The clean compile after
  deletion is the proof.

The C-ABI signatures (`PluginInitFn`, `PluginChatFn`, etc.) and the
`voxi_core::llm_types` handle-marshalling helpers are unchanged; `libc` is still
used for the C types (`c_void`, `c_char`, `free`).

## Why it's safer
`unsafe` is now confined to two justified categories — loading a library and
asserting a symbol's ABI type — each with a `// SAFETY:` note, instead of an
unchecked pointer transmute plus an unbacked thread-safety assertion.

## Verification
`./deploy.sh --test` → workspace compiles with the manual `unsafe impl` removed;
518 passing, 0 failing; no new warnings.

## Follow-ups
- `unsafe-checker` pass over the remaining `voxi_core::llm_types` FFI helpers.
- Wire the `on_chunk` streaming callback (currently `None`; pre-existing TODO).
