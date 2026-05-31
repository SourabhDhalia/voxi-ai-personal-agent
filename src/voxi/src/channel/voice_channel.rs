//! Voice channel — bridges the standalone `voxi-voice` pipeline to the agent.
//!
//! Capture -> VAD -> STT runs inside [`voxi_voice::VoiceEngine`]. This channel
//! consumes final transcripts, forwards them to `AgentCore::process_prompt`,
//! and pushes the reply back through the engine's TTS path. The daemon never
//! panics or fails to boot when voice models are missing: the engine falls back
//! to null STT/TTS and `start()` simply reports the degraded state.

use super::{Channel, ChannelConfig};
use crate::core::agent_core::AgentCore;
use crate::core::event_bus::{EventBus, EventType, SystemEvent};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use voxi_voice::{VoiceConfig, VoiceEngine, VoiceEvent, VoiceEventSink};

/// Session id used for all voice interactions on this channel.
const VOICE_SESSION_ID: &str = "voice";

/// EventBus custom-event name carrying voice lifecycle events. Dashboard and
/// other subscribers filter on `EventType::Custom("voice")`.
const VOICE_EVENT_NAME: &str = "voice";

/// Bridges `voxi-voice` [`VoiceEvent`]s onto the daemon [`EventBus`] so UI
/// surfaces (dashboard, etc.) can observe the pipeline without coupling to it.
struct EventBusVoiceSink {
    bus: Arc<EventBus>,
}

impl EventBusVoiceSink {
    fn new(bus: Arc<EventBus>) -> Self {
        EventBusVoiceSink { bus }
    }
}

impl VoiceEventSink for EventBusVoiceSink {
    fn emit(&self, event: VoiceEvent) {
        let kind = event.kind();
        let data = match &event {
            VoiceEvent::PartialTranscript(text)
            | VoiceEvent::CorrectedTranscript(text)
            | VoiceEvent::AgentResponseChunk(text)
            | VoiceEvent::AgentResponseComplete(text)
            | VoiceEvent::Error(text) => json!({ "kind": kind, "text": text }),
            VoiceEvent::FinalTranscript { text, confidence } => {
                json!({ "kind": kind, "text": text, "confidence": confidence })
            }
            VoiceEvent::AgentResponseStarted => json!({ "kind": kind }),
        };
        self.bus.publish(SystemEvent {
            event_type: EventType::Custom(VOICE_EVENT_NAME.to_string()),
            source: "voice".to_string(),
            data,
            timestamp: 0,
        });
    }
}

pub struct VoiceChannel {
    name: String,
    config: VoiceConfig,
    agent: Option<Arc<AgentCore>>,
    engine: Mutex<Option<Arc<VoiceEngine>>>,
    running: Arc<AtomicBool>,
    consumer: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl VoiceChannel {
    pub fn new(config: &ChannelConfig, agent: Option<Arc<AgentCore>>) -> Self {
        let voice_config = VoiceConfig::from_settings(&config.settings);
        VoiceChannel {
            name: config.name.clone(),
            config: voice_config,
            agent,
            engine: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            consumer: Mutex::new(None),
        }
    }
}

impl Channel for VoiceChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&mut self) -> bool {
        if self.running.load(Ordering::SeqCst) {
            return true;
        }

        // When an agent is wired in, forward voice events onto its EventBus so
        // the dashboard and other subscribers can observe the pipeline.
        let sink: Option<Arc<dyn VoiceEventSink>> = self
            .agent
            .as_ref()
            .map(|a| Arc::new(EventBusVoiceSink::new(a.event_bus())) as Arc<dyn VoiceEventSink>);
        let engine = Arc::new(VoiceEngine::new(self.config.clone(), sink));

        // Hand final transcripts to the agent on a dedicated thread, blocking on
        // the agent's async API via a captured tokio handle. Falls back to a
        // freshly built current-thread runtime if no ambient runtime exists.
        let transcript_rx = engine.take_transcript_receiver();
        if let (Some(rx), Some(agent)) = (transcript_rx, self.agent.clone()) {
            let engine_for_task = engine.clone();
            let running = self.running.clone();
            let handle = tokio::runtime::Handle::try_current().ok();
            let thread = std::thread::spawn(move || {
                run_transcript_consumer(rx, agent, engine_for_task, running, handle);
            });
            if let Ok(mut g) = self.consumer.lock() {
                *g = Some(thread);
            }
        } else if self.agent.is_none() {
            log::warn!("VoiceChannel: no AgentCore available; transcripts will not be processed");
        }

        let started = engine.start();
        if let Ok(mut g) = self.engine.lock() {
            *g = Some(engine);
        }
        self.running.store(true, Ordering::SeqCst);

        if started {
            log::info!("VoiceChannel '{}' started", self.name);
        } else {
            log::warn!(
                "VoiceChannel '{}' started in degraded mode (no audio capture)",
                self.name
            );
        }
        // Returning true keeps the channel registered/alive even in degraded
        // mode so it can still speak() and be reconfigured at runtime.
        true
    }

    fn stop(&mut self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            return;
        }
        // Stop the engine first so the transcript channel disconnects and the
        // consumer thread can observe the shutdown and exit.
        if let Ok(mut g) = self.engine.lock() {
            if let Some(engine) = g.take() {
                engine.stop();
            }
        }
        if let Ok(mut g) = self.consumer.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
            }
        }
        log::info!("VoiceChannel '{}' stopped", self.name);
    }

    fn send_message(&self, text: &str) -> Result<(), String> {
        let engine = self
            .engine
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .ok_or_else(|| "Voice engine not started".to_string())?;
        // TTS/playback may block; run it off the async reactor when inside one.
        engine.speak(text)
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Consume final transcripts, run them through the agent, and speak replies.
///
/// Runs on a dedicated thread. `handle` is the daemon's tokio runtime handle;
/// when present we `block_on` it, otherwise we build a transient current-thread
/// runtime so the channel still works in non-async test harnesses.
fn run_transcript_consumer(
    rx: std::sync::mpsc::Receiver<voxi_voice::TranscriptItem>,
    agent: Arc<AgentCore>,
    engine: Arc<VoiceEngine>,
    running: Arc<AtomicBool>,
    handle: Option<tokio::runtime::Handle>,
) {
    let fallback = handle.is_none().then(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build fallback voice runtime")
    });

    while running.load(Ordering::SeqCst) {
        let item = match rx.recv_timeout(std::time::Duration::from_millis(250)) {
            Ok(item) => item,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        log::info!(
            "VoiceChannel: transcript -> agent: {} (conf={:.2})",
            preview(&item.text, 80),
            item.confidence
        );

        let fut = agent.process_prompt_with_request(VOICE_SESSION_ID, &item.text, None, None);
        let response = match (&handle, &fallback) {
            (Some(h), _) => h.block_on(fut),
            (None, Some(rt)) => rt.block_on(fut),
            (None, None) => unreachable!("fallback runtime is built when no handle"),
        };

        if let Err(e) = engine.speak(&response) {
            log::warn!("VoiceChannel: speak failed: {e}");
        }
    }
    log::debug!("VoiceChannel transcript consumer exited");
}

fn preview(text: &str, max_chars: usize) -> &str {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => &text[..idx],
        None => text,
    }
}
