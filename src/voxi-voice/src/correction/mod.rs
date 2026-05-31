//! Transcript correction.
//!
//! A correction engine cleans up STT output (grammar, homophones, spacing)
//! before the text reaches the agent. The engine only invokes it when STT
//! confidence is below `confidence_threshold`, so high-confidence transcripts
//! skip the extra latency.
//!
//! Stage 1/host-default engine is [`passthrough::Passthrough`] (returns the
//! input unchanged). Qwen2.5 / SmolLM2 ONNX engines are built behind
//! `onnx-stt`/`onnx` and resolve their model on disk in this stage; their
//! token-generation loop is wired once the model URLs/checksums are confirmed.

pub mod passthrough;

#[cfg(feature = "onnx")]
pub mod qwen_onnx;

#[cfg(feature = "onnx")]
pub mod smollm2;

#[derive(Debug)]
pub enum CorrectionError {
    Load(String),
    Inference(String),
    Unsupported(String),
}

impl std::fmt::Display for CorrectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorrectionError::Load(s) => write!(f, "correction load error: {s}"),
            CorrectionError::Inference(s) => write!(f, "correction inference error: {s}"),
            CorrectionError::Unsupported(s) => write!(f, "correction unsupported: {s}"),
        }
    }
}

impl std::error::Error for CorrectionError {}

/// Corrects a raw transcript. Implementations must be cheap to call repeatedly
/// and must never panic on odd input.
pub trait CorrectionEngine: Send + Sync {
    fn name(&self) -> &str;

    /// Return a corrected version of `raw`. On error, callers fall back to the
    /// raw transcript rather than dropping the utterance.
    fn correct(&self, raw: &str) -> Result<String, CorrectionError>;
}

/// The shared prompt template used by the LLM-backed correction engines.
pub const CORRECTION_PROMPT: &str = "Correct transcription errors and grammar in the following text \
without changing the meaning. If the text is already correct, return it unchanged.\nText: ";
