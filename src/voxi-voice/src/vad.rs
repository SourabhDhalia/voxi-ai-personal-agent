//! Voice activity detection.
//!
//! Stage 1 ships a dependency-free energy/zero-crossing gate with hysteresis,
//! which is enough to segment speech from silence on a clean mic. A Silero
//! ONNX detector can replace this later behind the `onnx-stt` runtime without
//! changing the [`VoiceActivityDetector`] surface.

/// Emitted as audio chunks are fed through the detector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadEvent {
    /// No state change for this chunk.
    None,
    /// Speech just started (silence -> speech transition).
    SpeechStart,
    /// Speech just ended (speech -> silence transition, after hang time).
    SpeechEnd,
}

/// Tunable thresholds for the energy gate.
#[derive(Clone, Copy, Debug)]
pub struct VadConfig {
    /// RMS amplitude (0.0..1.0) above which a chunk is considered voiced.
    pub energy_threshold: f32,
    /// Consecutive voiced chunks required to declare speech start.
    pub start_chunks: u32,
    /// Consecutive silent chunks tolerated before declaring speech end.
    pub hang_chunks: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        VadConfig {
            energy_threshold: 0.015,
            start_chunks: 3,
            hang_chunks: 12,
        }
    }
}

/// Stateful energy-based detector. Feed it fixed-size PCM chunks.
pub struct VoiceActivityDetector {
    cfg: VadConfig,
    in_speech: bool,
    voiced_run: u32,
    silent_run: u32,
}

impl VoiceActivityDetector {
    pub fn new(cfg: VadConfig) -> Self {
        VoiceActivityDetector {
            cfg,
            in_speech: false,
            voiced_run: 0,
            silent_run: 0,
        }
    }

    pub fn is_in_speech(&self) -> bool {
        self.in_speech
    }

    /// Reset to the idle (silence) state.
    pub fn reset(&mut self) {
        self.in_speech = false;
        self.voiced_run = 0;
        self.silent_run = 0;
    }

    /// Root-mean-square amplitude of a PCM f32 chunk in [0, ~1].
    pub fn rms(pcm: &[f32]) -> f32 {
        if pcm.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = pcm.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (sum_sq / pcm.len() as f64).sqrt() as f32
    }

    /// Process one chunk and return the resulting state transition, if any.
    pub fn process_chunk(&mut self, pcm: &[f32]) -> VadEvent {
        let voiced = Self::rms(pcm) >= self.cfg.energy_threshold;

        if voiced {
            self.voiced_run = self.voiced_run.saturating_add(1);
            self.silent_run = 0;
        } else {
            self.silent_run = self.silent_run.saturating_add(1);
            self.voiced_run = 0;
        }

        if !self.in_speech {
            if self.voiced_run >= self.cfg.start_chunks {
                self.in_speech = true;
                self.silent_run = 0;
                return VadEvent::SpeechStart;
            }
        } else if self.silent_run >= self.cfg.hang_chunks {
            self.in_speech = false;
            self.voiced_run = 0;
            return VadEvent::SpeechEnd;
        }

        VadEvent::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loud(n: usize) -> Vec<f32> {
        vec![0.5; n]
    }
    fn quiet(n: usize) -> Vec<f32> {
        vec![0.0; n]
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(VoiceActivityDetector::rms(&quiet(160)), 0.0);
        assert!(VoiceActivityDetector::rms(&[]) == 0.0);
    }

    #[test]
    fn detects_speech_start_and_end() {
        let mut vad = VoiceActivityDetector::new(VadConfig::default());
        let mut started = false;
        // Ramp through enough voiced chunks to trip start.
        for _ in 0..3 {
            if vad.process_chunk(&loud(160)) == VadEvent::SpeechStart {
                started = true;
            }
        }
        assert!(started, "should have started speech");
        assert!(vad.is_in_speech());

        // Now feed silence through the hang window.
        let mut ended = false;
        for _ in 0..12 {
            if vad.process_chunk(&quiet(160)) == VadEvent::SpeechEnd {
                ended = true;
            }
        }
        assert!(ended, "should have ended speech");
        assert!(!vad.is_in_speech());
    }
}
