//! Wake-word detection.
//!
//! When `trigger_mode == WakeWord`, the engine keeps STT idle until a detector
//! reports the wake phrase ("hey voxi"). Stage 4 always-available detector is
//! [`null_wake::NullWakeWord`] (never triggers — capture stays gated, logged
//! clearly). Porcupine (FFI) and a keyword-spotting ONNX detector are built
//! behind features and resolve their assets on disk.

pub mod null_wake;

#[cfg(feature = "onnx")]
pub mod keyword_onnx;

#[cfg(feature = "porcupine")]
pub mod porcupine;

#[derive(Debug)]
pub enum WakeWordError {
    Load(String),
    Detection(String),
    Unsupported(String),
}

impl std::fmt::Display for WakeWordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WakeWordError::Load(s) => write!(f, "wake-word load error: {s}"),
            WakeWordError::Detection(s) => write!(f, "wake-word detection error: {s}"),
            WakeWordError::Unsupported(s) => write!(f, "wake-word unsupported: {s}"),
        }
    }
}

impl std::error::Error for WakeWordError {}

/// A wake-word detector. Fed the same PCM chunks as the VAD; returns true on
/// the chunk where the wake phrase completes.
pub trait WakeWordDetector: Send + Sync {
    fn name(&self) -> &str;

    /// Process one PCM chunk; return true if the wake word was just detected.
    fn detect_chunk(&self, pcm: &[f32]) -> bool;

    /// Reset detector state (e.g. after the armed utterance ends).
    fn reset(&self);
}
