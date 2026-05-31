//! Audio capture/playback abstraction.
//!
//! The engine talks to audio hardware only through [`AudioBackend`]. Stage 1
//! always-available backend is [`null_backend::NullBackend`]; CPAL is built in
//! behind the `cpal-audio` feature, ALSA arrives in a later stage.

use crate::config::AudioBackendType;
use std::sync::mpsc::Receiver;

pub mod backend_selector;
pub mod null_backend;

#[cfg(feature = "cpal-audio")]
pub mod cpal_backend;

/// Audio format the pipeline operates on: mono f32 PCM at `sample_rate`.
#[derive(Clone, Copy, Debug)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioFormat {
    pub fn mono(sample_rate: u32) -> Self {
        AudioFormat {
            sample_rate,
            channels: 1,
        }
    }
}

/// Errors raised while starting or using an audio backend.
#[derive(Debug)]
pub enum AudioError {
    /// No capable device was found.
    NoDevice(String),
    /// The backend feature is not compiled in.
    Unsupported(String),
    /// A device/stream level failure.
    Backend(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoDevice(s) => write!(f, "no audio device: {s}"),
            AudioError::Unsupported(s) => write!(f, "audio backend unsupported: {s}"),
            AudioError::Backend(s) => write!(f, "audio backend error: {s}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// Capture + playback for a single audio device.
///
/// Implementations must be `Send + Sync` so the engine can drive capture from a
/// worker thread while playback is invoked from elsewhere.
pub trait AudioBackend: Send + Sync {
    /// Human-readable backend name (for logs).
    fn name(&self) -> &str;

    /// Begin capture and return a receiver of fixed-size mono f32 PCM chunks.
    /// The stream stops when the backend is dropped.
    fn start_capture(&self, format: AudioFormat) -> Result<Receiver<Vec<f32>>, AudioError>;

    /// Play a buffer of mono f32 PCM at the given sample rate, blocking until
    /// playback completes (or returning immediately for the null backend).
    fn play(&self, pcm: &[f32], format: AudioFormat) -> Result<(), AudioError>;
}

/// Pick a concrete backend for the requested type, honouring compiled features
/// and the `VOXI_AUDIO_BACKEND` env override.
///
/// Stage 1 falls back to the null backend whenever a real backend is requested
/// but its feature is absent — the daemon must never fail to boot over audio.
pub fn build_backend(requested: AudioBackendType) -> Box<dyn AudioBackend> {
    let resolved = backend_selector::resolve_type(requested);
    match resolved {
        AudioBackendType::Cpal => build_cpal(),
        AudioBackendType::Alsa => {
            log::warn!("ALSA backend not yet implemented; using null backend");
            Box::new(null_backend::NullBackend::default())
        }
        AudioBackendType::Null | AudioBackendType::Auto => {
            Box::new(null_backend::NullBackend::default())
        }
    }
}

#[cfg(feature = "cpal-audio")]
fn build_cpal() -> Box<dyn AudioBackend> {
    match cpal_backend::CpalBackend::new() {
        Ok(b) => Box::new(b),
        Err(e) => {
            log::warn!("CPAL backend init failed ({e}); falling back to null backend");
            Box::new(null_backend::NullBackend::default())
        }
    }
}

#[cfg(not(feature = "cpal-audio"))]
fn build_cpal() -> Box<dyn AudioBackend> {
    log::warn!("CPAL backend requested but `cpal-audio` feature is disabled; using null backend");
    Box::new(null_backend::NullBackend::default())
}
