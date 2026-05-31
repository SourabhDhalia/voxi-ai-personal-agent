//! Speech-to-text abstraction.
//!
//! The engine feeds PCM chunks during a speech segment and asks for a final
//! transcript at `SpeechEnd`. Stage 1 always-available engine is
//! [`NullStt`]; the Moonshine ONNX engine is built behind `onnx-stt`.

pub mod null_stt;

#[cfg(feature = "onnx-stt")]
pub mod moonshine;

/// Result of a transcription step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SttResult {
    /// Streaming partial hypothesis, if the engine supports it.
    pub partial: Option<String>,
    /// Final transcript once the segment is complete.
    pub final_text: Option<String>,
    /// Confidence in [0, 1]; engines without a score report 0.0.
    pub confidence: f32,
}

impl SttResult {
    pub fn partial(text: impl Into<String>) -> Self {
        SttResult {
            partial: Some(text.into()),
            final_text: None,
            confidence: 0.0,
        }
    }

    pub fn final_text(text: impl Into<String>, confidence: f32) -> Self {
        SttResult {
            partial: None,
            final_text: Some(text.into()),
            confidence,
        }
    }
}

#[derive(Debug)]
pub enum SttError {
    /// Model could not be loaded (missing/corrupt files).
    Load(String),
    /// Inference failure.
    Inference(String),
    /// Engine/feature not compiled in.
    Unsupported(String),
}

impl std::fmt::Display for SttError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SttError::Load(s) => write!(f, "STT load error: {s}"),
            SttError::Inference(s) => write!(f, "STT inference error: {s}"),
            SttError::Unsupported(s) => write!(f, "STT unsupported: {s}"),
        }
    }
}

impl std::error::Error for SttError {}

/// A speech-to-text engine. Implementations are stateful across a single
/// speech segment and reset on [`SttEngine::reset`].
pub trait SttEngine: Send + Sync {
    fn name(&self) -> &str;

    /// Feed one PCM chunk; may return a partial hypothesis.
    fn accept_chunk(&self, pcm: &[f32]) -> Result<SttResult, SttError>;

    /// Signal end of segment and return the final transcript.
    fn finalize(&self) -> Result<SttResult, SttError>;

    /// Reset internal state before the next segment.
    fn reset(&self);
}
