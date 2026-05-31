//! voxi-voice — standalone bidirectional voice pipeline for the Voxi daemon.
//!
//! Flow: microphone -> VAD -> STT -> (correction) -> agent -> TTS -> speaker.
//!
//! The crate is intentionally decoupled from the daemon: it does not depend on
//! `voxi` or `AgentCore`. The host wires an agent in by consuming final
//! transcripts (via [`VoiceEngine::take_transcript_receiver`]) and pushing the
//! agent's reply back through [`VoiceEngine::speak`]. UI surfaces subscribe to
//! [`VoiceEvent`]s through a [`VoiceEventSink`].
//!
//! Stage 1 ships the full architecture with a dependency-free `null` path that
//! degrades gracefully. Real audio (`cpal-audio`) and ONNX inference
//! (`onnx-stt` / `onnx-tts`) live behind off-by-default cargo features.

pub mod audio;
pub mod config;
pub mod correction;
pub mod engine;
pub mod events;
pub mod model_store;
pub mod sha256;
pub mod stt;
pub mod tts;
pub mod vad;
pub mod wake_word;

pub use config::{AudioBackendType, TriggerMode, VoiceConfig};
pub use correction::CorrectionEngine;
pub use engine::{TranscriptItem, VoiceEngine};
pub use events::{NullEventSink, VoiceEvent, VoiceEventSink};
pub use model_store::{
    Downloader, ModelRegistry, ModelStatus, ModelStore, ModelStoreError, RegistryEntry,
    VerifyReport,
};
pub use stt::SttResult;
