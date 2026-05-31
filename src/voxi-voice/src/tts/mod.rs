//! Text-to-speech abstraction.
//!
//! `synthesize` returns raw mono f32 PCM at the engine's native sample rate.
//! Stage 1 always-available engine is [`NullTts`]; Kokoro ONNX is built behind
//! `onnx-tts`.

pub mod null_tts;

#[cfg(feature = "onnx-tts")]
pub mod kokoro;

/// Synthesized audio plus the rate it was produced at.
#[derive(Clone, Debug, Default)]
pub struct TtsAudio {
    pub pcm: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Debug)]
pub enum TtsError {
    Load(String),
    Synthesis(String),
    Unsupported(String),
}

impl std::fmt::Display for TtsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtsError::Load(s) => write!(f, "TTS load error: {s}"),
            TtsError::Synthesis(s) => write!(f, "TTS synthesis error: {s}"),
            TtsError::Unsupported(s) => write!(f, "TTS unsupported: {s}"),
        }
    }
}

impl std::error::Error for TtsError {}

/// A text-to-speech engine.
pub trait TtsEngine: Send + Sync {
    fn name(&self) -> &str;

    /// The native output sample rate of this engine.
    fn sample_rate(&self) -> u32;

    /// Synthesize speech for `text`.
    fn synthesize(&self, text: &str) -> Result<TtsAudio, TtsError>;
}
