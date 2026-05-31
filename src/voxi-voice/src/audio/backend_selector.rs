//! Audio backend selection.
//!
//! Centralizes the "which backend do we actually use" decision so it can be
//! unit-tested without touching audio hardware. The resolution order is:
//!   1. `VOXI_AUDIO_BACKEND` env override (highest precedence)
//!   2. the configured [`AudioBackendType`]
//!   3. `Auto` → compile-time capability detection
//!
//! Selecting a backend whose feature is not compiled in degrades to the null
//! backend at construction time (see [`super::build_backend`]); this module only
//! decides the *type*.

use crate::config::AudioBackendType;

/// Resolve the effective backend type from a request, consulting the
/// `VOXI_AUDIO_BACKEND` env override first, then resolving `Auto` against the
/// compiled-in capabilities.
pub fn resolve_type(requested: AudioBackendType) -> AudioBackendType {
    if let Some(env) = std::env::var_os("VOXI_AUDIO_BACKEND") {
        if let Some(s) = env.to_str() {
            // An explicit env value still resolves `Auto` to a concrete backend.
            return resolve_auto(AudioBackendType::parse(s));
        }
    }
    resolve_auto(requested)
}

/// Map `Auto` to the best compiled-in backend; pass other variants through.
fn resolve_auto(ty: AudioBackendType) -> AudioBackendType {
    match ty {
        AudioBackendType::Auto => AudioBackendType::detect(),
        other => other,
    }
}

impl AudioBackendType {
    /// Compile-time capability detection used to resolve `Auto`. CPAL covers
    /// macOS and desktop Linux/Windows; embedded ALSA arrives in a later stage.
    pub fn detect() -> AudioBackendType {
        if cfg!(feature = "cpal-audio") {
            AudioBackendType::Cpal
        } else {
            AudioBackendType::Null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests mutate a process-global env var; keep them in one function so
    // they run serially and restore state, avoiding cross-test interference.
    #[test]
    fn resolution_precedence() {
        let saved = std::env::var_os("VOXI_AUDIO_BACKEND");
        std::env::remove_var("VOXI_AUDIO_BACKEND");

        // Explicit request passes through untouched.
        assert_eq!(resolve_type(AudioBackendType::Null), AudioBackendType::Null);
        assert_eq!(resolve_type(AudioBackendType::Cpal), AudioBackendType::Cpal);

        // Auto resolves to the compile-time detected backend.
        assert_eq!(resolve_type(AudioBackendType::Auto), AudioBackendType::detect());

        // Env override wins over the configured value.
        std::env::set_var("VOXI_AUDIO_BACKEND", "null");
        assert_eq!(resolve_type(AudioBackendType::Cpal), AudioBackendType::Null);

        // An `auto` env value still resolves to a concrete backend.
        std::env::set_var("VOXI_AUDIO_BACKEND", "auto");
        assert_eq!(resolve_type(AudioBackendType::Cpal), AudioBackendType::detect());

        match saved {
            Some(v) => std::env::set_var("VOXI_AUDIO_BACKEND", v),
            None => std::env::remove_var("VOXI_AUDIO_BACKEND"),
        }
    }
}
