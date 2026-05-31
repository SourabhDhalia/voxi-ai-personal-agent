//! No-op audio backend for tests, CI, and headless devices.
//!
//! Capture yields a receiver that never produces audio (the sender is held
//! alive so the channel stays open but idle); playback is a logged no-op.

use super::{AudioBackend, AudioError, AudioFormat};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

#[derive(Default)]
pub struct NullBackend {
    /// Keeps capture senders alive so receivers don't see a closed channel.
    keepalive: Mutex<Vec<Sender<Vec<f32>>>>,
}

impl AudioBackend for NullBackend {
    fn name(&self) -> &str {
        "null"
    }

    fn start_capture(&self, _format: AudioFormat) -> Result<Receiver<Vec<f32>>, AudioError> {
        let (tx, rx) = channel();
        if let Ok(mut guard) = self.keepalive.lock() {
            guard.push(tx);
        }
        log::debug!("NullBackend: capture started (no audio will be produced)");
        Ok(rx)
    }

    fn play(&self, pcm: &[f32], format: AudioFormat) -> Result<(), AudioError> {
        log::debug!(
            "NullBackend: play {} samples @ {} Hz (discarded)",
            pcm.len(),
            format.sample_rate
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::TryRecvError;

    #[test]
    fn capture_channel_stays_open_but_empty() {
        let b = NullBackend::default();
        let rx = b.start_capture(AudioFormat::mono(16_000)).unwrap();
        // Empty, not disconnected.
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn play_is_ok() {
        let b = NullBackend::default();
        assert!(b.play(&[0.0; 32], AudioFormat::mono(24_000)).is_ok());
    }
}
