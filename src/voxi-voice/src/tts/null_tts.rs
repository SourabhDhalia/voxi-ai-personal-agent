//! No-op TTS engine.
//!
//! Returns an empty PCM buffer so the playback path can be exercised without a
//! model. Used for tests/CI and as the graceful fallback when no real TTS
//! model is installed.

use super::{TtsAudio, TtsEngine, TtsError};

pub struct NullTts {
    sample_rate: u32,
}

impl NullTts {
    pub fn new(sample_rate: u32) -> Self {
        NullTts { sample_rate }
    }
}

impl Default for NullTts {
    fn default() -> Self {
        NullTts::new(24_000)
    }
}

impl TtsEngine for NullTts {
    fn name(&self) -> &str {
        "null"
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn synthesize(&self, text: &str) -> Result<TtsAudio, TtsError> {
        log::debug!(
            "NullTts: synthesize {} chars (no audio produced)",
            text.chars().count()
        );
        Ok(TtsAudio {
            pcm: Vec::new(),
            sample_rate: self.sample_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_audio() {
        let t = NullTts::new(16_000);
        let a = t.synthesize("hello").unwrap();
        assert!(a.pcm.is_empty());
        assert_eq!(a.sample_rate, 16_000);
    }
}
