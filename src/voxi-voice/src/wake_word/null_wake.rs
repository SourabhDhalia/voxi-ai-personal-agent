//! No-op wake-word detector — never triggers.
//!
//! Used when wake-word mode is selected but no real detector is compiled in.
//! Capture stays gated and a single warning is logged so the operator knows to
//! install Porcupine or a keyword model, or switch trigger mode.

use super::WakeWordDetector;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct NullWakeWord {
    warned: AtomicBool,
}

impl WakeWordDetector for NullWakeWord {
    fn name(&self) -> &str {
        "null"
    }

    fn detect_chunk(&self, _pcm: &[f32]) -> bool {
        if !self.warned.swap(true, Ordering::Relaxed) {
            log::warn!(
                "wake-word mode selected but no detector is compiled in; \
                 voice capture will not arm. Build with `porcupine`/`onnx` or \
                 switch trigger_mode to vad/push_to_talk."
            );
        }
        false
    }

    fn reset(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_triggers() {
        let w = NullWakeWord::default();
        assert!(!w.detect_chunk(&[0.9; 160]));
    }
}
