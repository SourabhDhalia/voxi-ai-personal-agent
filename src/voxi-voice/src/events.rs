//! Voice events and the sink abstraction the daemon bridges to its EventBus.

use std::sync::Arc;

/// Lifecycle events emitted by the voice pipeline. Any UI surface (web
/// dashboard, Telegram, TV) can observe these without coupling to the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceEvent {
    /// Streaming partial transcript for live UI feedback.
    PartialTranscript(String),
    /// Final transcript ready to send to the agent.
    FinalTranscript { text: String, confidence: f32 },
    /// Transcript after the correction engine (Stage 3).
    CorrectedTranscript(String),
    /// Agent began producing a response.
    AgentResponseStarted,
    /// A streamed chunk of the agent response.
    AgentResponseChunk(String),
    /// Agent response complete (full text).
    AgentResponseComplete(String),
    /// A recoverable error in the pipeline.
    Error(String),
}

impl VoiceEvent {
    /// Short, stable kind tag — useful for serialization to an event bus.
    pub fn kind(&self) -> &'static str {
        match self {
            VoiceEvent::PartialTranscript(_) => "partial_transcript",
            VoiceEvent::FinalTranscript { .. } => "final_transcript",
            VoiceEvent::CorrectedTranscript(_) => "corrected_transcript",
            VoiceEvent::AgentResponseStarted => "agent_response_started",
            VoiceEvent::AgentResponseChunk(_) => "agent_response_chunk",
            VoiceEvent::AgentResponseComplete(_) => "agent_response_complete",
            VoiceEvent::Error(_) => "error",
        }
    }
}

/// A consumer of voice events. The daemon implements this to forward events
/// onto its own EventBus; tests use [`NullEventSink`].
pub trait VoiceEventSink: Send + Sync {
    fn emit(&self, event: VoiceEvent);
}

/// Logs events at debug level and otherwise drops them.
#[derive(Default)]
pub struct NullEventSink;

impl VoiceEventSink for NullEventSink {
    fn emit(&self, event: VoiceEvent) {
        log::debug!("[voice-event] {}: {:?}", event.kind(), event);
    }
}

/// Convenience for an optional, shared sink.
pub type SharedEventSink = Arc<dyn VoiceEventSink>;
