//! No-op STT engine.
//!
//! Accumulates the number of samples seen so the daemon can prove the capture
//! path is wired, but never fabricates transcript text. Used for tests/CI and
//! as the graceful fallback when no real STT model is installed.

use super::{SttEngine, SttError, SttResult};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct NullStt {
    samples_seen: AtomicUsize,
}

impl SttEngine for NullStt {
    fn name(&self) -> &str {
        "null"
    }

    fn accept_chunk(&self, pcm: &[f32]) -> Result<SttResult, SttError> {
        self.samples_seen.fetch_add(pcm.len(), Ordering::Relaxed);
        Ok(SttResult::default())
    }

    fn finalize(&self) -> Result<SttResult, SttError> {
        let n = self.samples_seen.swap(0, Ordering::Relaxed);
        log::debug!("NullStt: finalize over {n} samples (no transcript produced)");
        // No model => no transcript. Empty final_text signals "nothing to send".
        Ok(SttResult::default())
    }

    fn reset(&self) {
        self.samples_seen.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_no_transcript() {
        let s = NullStt::default();
        s.accept_chunk(&[0.1; 160]).unwrap();
        let r = s.finalize().unwrap();
        assert!(r.final_text.is_none());
    }
}
