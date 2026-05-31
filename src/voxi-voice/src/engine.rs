//! Voice pipeline orchestrator.
//!
//! [`VoiceEngine`] owns the audio backend, VAD, STT, and TTS engines and runs
//! the capture -> VAD -> STT loop on a worker thread. Final transcripts are
//! delivered to the host over an mpsc channel ([`VoiceEngine::take_transcript_receiver`]);
//! the host feeds them to the agent and pushes the reply back via
//! [`VoiceEngine::speak`]. Lifecycle events flow to a [`VoiceEventSink`].
//!
//! The engine is `Arc`-shareable: `speak` takes `&self` so a tokio task can
//! hold a clone and synthesize replies while the capture thread runs.

use crate::audio::{self, AudioBackend, AudioFormat};
use crate::config::VoiceConfig;
use crate::correction::{passthrough::Passthrough, CorrectionEngine};
use crate::events::{NullEventSink, VoiceEvent, VoiceEventSink};
use crate::model_store::ModelStore;
use crate::stt::{null_stt::NullStt, SttEngine};
use crate::tts::{null_tts::NullTts, TtsEngine};
use crate::vad::{VadConfig, VadEvent, VoiceActivityDetector};
use crate::wake_word::{null_wake::NullWakeWord, WakeWordDetector};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// A finalized transcript handed to the host for agent processing.
#[derive(Clone, Debug, PartialEq)]
pub struct TranscriptItem {
    pub text: String,
    pub confidence: f32,
}

/// Shared, thread-safe pieces driven by the capture worker.
struct Pipeline {
    audio: Box<dyn AudioBackend>,
    stt: Arc<dyn SttEngine>,
    tts: Arc<dyn TtsEngine>,
    correction: Arc<dyn CorrectionEngine>,
    wake: Arc<dyn WakeWordDetector>,
    events: Arc<dyn VoiceEventSink>,
    config: VoiceConfig,
    // `std::mpsc::Sender` is `!Sync`; guard it so `Pipeline` (and thus
    // `Arc<Pipeline>`) is `Send + Sync` and can cross the capture thread.
    transcript_tx: Mutex<Sender<TranscriptItem>>,
}

pub struct VoiceEngine {
    pipeline: Arc<Pipeline>,
    running: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
    transcript_rx: Mutex<Option<Receiver<TranscriptItem>>>,
    /// Push-to-talk gate; when the trigger mode is PushToTalk, capture is only
    /// processed while this is true.
    ptt_active: Arc<AtomicBool>,
}

impl VoiceEngine {
    /// Build an engine from config, selecting STT/TTS by model id. Falls back
    /// to null engines (with a logged warning) when a real model is missing or
    /// the relevant feature is not compiled in — the daemon never fails to boot
    /// over a missing voice model.
    pub fn new(config: VoiceConfig, events: Option<Arc<dyn VoiceEventSink>>) -> Self {
        let events: Arc<dyn VoiceEventSink> =
            events.unwrap_or_else(|| Arc::new(NullEventSink::default()));
        let store = ModelStore::new(config.model_dir.clone());

        let audio = audio::build_backend(config.audio_backend);
        let stt = build_stt(&store, &config, &events);
        let tts = build_tts(&store, &config, &events);
        let correction = build_correction(&store, &config, &events);
        let wake = build_wake_word(&store, &config, &events);

        log::info!(
            "VoiceEngine: audio={}, stt={}, tts={}, correction={}, wake={}, trigger={:?}",
            audio.name(),
            stt.name(),
            tts.name(),
            correction.name(),
            wake.name(),
            config.trigger_mode
        );

        let (transcript_tx, transcript_rx) = channel();

        VoiceEngine {
            pipeline: Arc::new(Pipeline {
                audio,
                stt,
                tts,
                correction,
                wake,
                events,
                config,
                transcript_tx: Mutex::new(transcript_tx),
            }),
            running: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
            transcript_rx: Mutex::new(Some(transcript_rx)),
            ptt_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Take the transcript receiver (once). The host loops on this to forward
    /// final transcripts to the agent.
    pub fn take_transcript_receiver(&self) -> Option<Receiver<TranscriptItem>> {
        self.transcript_rx.lock().ok().and_then(|mut g| g.take())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Mark the start of a push-to-talk utterance.
    pub fn push_to_talk_start(&self) {
        self.ptt_active.store(true, Ordering::SeqCst);
    }

    /// Mark the end of a push-to-talk utterance.
    pub fn push_to_talk_stop(&self) {
        self.ptt_active.store(false, Ordering::SeqCst);
    }

    /// Start the capture/STT worker. Returns false if already running or the
    /// audio capture stream could not be opened.
    pub fn start(&self) -> bool {
        if self.running.swap(true, Ordering::SeqCst) {
            return true;
        }

        let format = AudioFormat::mono(self.pipeline.config.sample_rate);
        let rx = match self.pipeline.audio.start_capture(format) {
            Ok(rx) => rx,
            Err(e) => {
                log::warn!("VoiceEngine: capture start failed: {e}");
                self.pipeline.events.emit(VoiceEvent::Error(e.to_string()));
                self.running.store(false, Ordering::SeqCst);
                return false;
            }
        };

        let pipeline = self.pipeline.clone();
        let running = self.running.clone();
        let ptt = self.ptt_active.clone();

        let handle = std::thread::spawn(move || {
            capture_loop(pipeline, running, ptt, rx);
        });

        if let Ok(mut g) = self.worker.lock() {
            *g = Some(handle);
        }
        log::info!("VoiceEngine started");
        true
    }

    /// Stop the worker thread and join it.
    pub fn stop(&self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            return;
        }
        if let Ok(mut g) = self.worker.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
            }
        }
        log::info!("VoiceEngine stopped");
    }

    /// Synthesize `text` and play it through the audio backend. Emits agent
    /// response lifecycle events. Returns Err on synthesis/playback failure.
    pub fn speak(&self, text: &str) -> Result<(), String> {
        let p = &self.pipeline;
        p.events.emit(VoiceEvent::AgentResponseStarted);

        let audio = match p.tts.synthesize(text) {
            Ok(a) => a,
            Err(e) => {
                p.events.emit(VoiceEvent::Error(e.to_string()));
                return Err(e.to_string());
            }
        };

        let played = if audio.pcm.is_empty() {
            // Null/utility TTS produced nothing — nothing to play, not an error.
            Ok(())
        } else {
            p.audio
                .play(&audio.pcm, AudioFormat::mono(audio.sample_rate))
                .map_err(|e| e.to_string())
        };

        p.events
            .emit(VoiceEvent::AgentResponseComplete(text.to_string()));

        played
    }
}

impl Drop for VoiceEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The capture worker: read PCM chunks, gate by VAD/PTT, run STT, and on
/// `SpeechEnd` emit a final transcript.
fn capture_loop(
    pipeline: Arc<Pipeline>,
    running: Arc<AtomicBool>,
    ptt: Arc<AtomicBool>,
    rx: Receiver<Vec<f32>>,
) {
    use crate::config::TriggerMode;

    let mut vad = VoiceActivityDetector::new(VadConfig::default());
    let ptt_mode = pipeline.config.trigger_mode == TriggerMode::PushToTalk;
    let wake_mode = pipeline.config.trigger_mode == TriggerMode::WakeWord;
    // Tracks whether we were capturing on the previous PTT chunk, to detect the
    // release edge and finalize exactly once.
    let mut ptt_was_active = false;
    // In wake-word mode, STT stays gated until the wake phrase fires; once armed
    // we capture a single VAD-delimited utterance, then re-gate.
    let mut wake_armed = false;
    pipeline.stt.reset();
    pipeline.wake.reset();

    while running.load(Ordering::SeqCst) {
        // Block briefly for the next chunk so we stay responsive to `stop`.
        let chunk = match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(c) => c,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // On timeout, still honour a pending PTT release.
                if ptt_mode && ptt_was_active && !ptt.load(Ordering::SeqCst) {
                    finalize_segment(&pipeline);
                    ptt_was_active = false;
                }
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if ptt_mode {
            let active = ptt.load(Ordering::SeqCst);
            if active {
                if !ptt_was_active {
                    pipeline.stt.reset();
                }
                let _ = pipeline.stt.accept_chunk(&chunk);
            } else if ptt_was_active {
                // Release edge: finalize the held utterance.
                finalize_segment(&pipeline);
            }
            ptt_was_active = active;
            continue;
        }

        if wake_mode && !wake_armed {
            // Feed the detector; arm STT capture on the wake chunk.
            if pipeline.wake.detect_chunk(&chunk) {
                log::debug!("wake word detected; arming capture");
                wake_armed = true;
                vad.reset();
                pipeline.stt.reset();
            }
            continue;
        }

        match vad.process_chunk(&chunk) {
            VadEvent::SpeechStart => {
                pipeline.stt.reset();
                let _ = pipeline.stt.accept_chunk(&chunk);
            }
            VadEvent::None => {
                if vad.is_in_speech() {
                    if let Ok(res) = pipeline.stt.accept_chunk(&chunk) {
                        if let Some(partial) = res.partial {
                            pipeline.events.emit(VoiceEvent::PartialTranscript(partial));
                        }
                    }
                }
            }
            VadEvent::SpeechEnd => {
                finalize_segment(&pipeline);
                if wake_mode {
                    // Utterance done — re-gate until the next wake phrase.
                    wake_armed = false;
                    pipeline.wake.reset();
                }
            }
        }
    }

    log::debug!("VoiceEngine capture loop exited");
}

/// Finalize the current STT segment and dispatch the transcript.
///
/// Low-confidence transcripts are run through the correction engine before
/// dispatch; the corrected text (if it differs) is emitted as a
/// `CorrectedTranscript` event and forwarded to the agent.
fn finalize_segment(pipeline: &Arc<Pipeline>) {
    match pipeline.stt.finalize() {
        Ok(res) => {
            if let Some(text) = res.final_text {
                let raw = text.trim().to_string();
                if raw.is_empty() {
                    return;
                }
                pipeline.events.emit(VoiceEvent::FinalTranscript {
                    text: raw.clone(),
                    confidence: res.confidence,
                });

                let text = maybe_correct(pipeline, &raw, res.confidence);

                if let Ok(tx) = pipeline.transcript_tx.lock() {
                    let _ = tx.send(TranscriptItem {
                        text,
                        confidence: res.confidence,
                    });
                }
            }
        }
        Err(e) => {
            log::debug!("VoiceEngine: finalize produced no transcript: {e}");
        }
    }
}

/// Apply the correction engine when confidence is below threshold. Returns the
/// text to forward (corrected when it changed, otherwise the raw transcript).
fn maybe_correct(pipeline: &Arc<Pipeline>, raw: &str, confidence: f32) -> String {
    if confidence >= pipeline.config.confidence_threshold {
        return raw.to_string();
    }
    match pipeline.correction.correct(raw) {
        Ok(corrected) => {
            let corrected = corrected.trim().to_string();
            if !corrected.is_empty() && corrected != raw {
                pipeline
                    .events
                    .emit(VoiceEvent::CorrectedTranscript(corrected.clone()));
                corrected
            } else {
                raw.to_string()
            }
        }
        Err(e) => {
            log::debug!("VoiceEngine: correction skipped ({e})");
            raw.to_string()
        }
    }
}

fn build_stt(
    store: &ModelStore,
    config: &VoiceConfig,
    events: &Arc<dyn VoiceEventSink>,
) -> Arc<dyn SttEngine> {
    #[cfg(feature = "onnx-stt")]
    {
        match crate::stt::moonshine::MoonshineStt::load(store, &config.stt_model) {
            Ok(m) => return Arc::new(m),
            Err(e) => {
                log::warn!("STT '{}' unavailable ({e}); using null STT", config.stt_model);
                events.emit(VoiceEvent::Error(format!("stt fallback: {e}")));
            }
        }
    }
    #[cfg(not(feature = "onnx-stt"))]
    {
        let _ = (store, events);
        log::warn!(
            "onnx-stt feature disabled; STT model '{}' will not transcribe (null STT)",
            config.stt_model
        );
    }
    Arc::new(NullStt::default())
}

fn build_tts(
    store: &ModelStore,
    config: &VoiceConfig,
    events: &Arc<dyn VoiceEventSink>,
) -> Arc<dyn TtsEngine> {
    #[cfg(feature = "onnx-tts")]
    {
        match crate::tts::kokoro::KokoroTts::load(store, &config.tts_model, None) {
            Ok(t) => return Arc::new(t),
            Err(e) => {
                log::warn!("TTS '{}' unavailable ({e}); using null TTS", config.tts_model);
                events.emit(VoiceEvent::Error(format!("tts fallback: {e}")));
            }
        }
    }
    #[cfg(not(feature = "onnx-tts"))]
    {
        let _ = (store, events);
        log::warn!(
            "onnx-tts feature disabled; TTS model '{}' will not synthesize (null TTS)",
            config.tts_model
        );
    }
    Arc::new(NullTts::new(config.sample_rate))
}

/// Build the correction engine from config. When no correction model is set, or
/// the relevant feature/model is unavailable, falls back to the pure-Rust
/// [`Passthrough`] engine (whitespace normalization only).
fn build_correction(
    store: &ModelStore,
    config: &VoiceConfig,
    events: &Arc<dyn VoiceEventSink>,
) -> Arc<dyn CorrectionEngine> {
    let model_id = match config.correction_model.as_deref() {
        Some(id) if !id.is_empty() => id,
        _ => return Arc::new(Passthrough::default()),
    };

    #[cfg(feature = "onnx")]
    {
        // Pick the engine by a coarse name match; both share the same contract.
        if model_id.contains("smol") {
            match crate::correction::smollm2::SmolLm2Correction::load(store, model_id) {
                Ok(c) => return Arc::new(c),
                Err(e) => {
                    log::warn!("correction '{model_id}' unavailable ({e}); using passthrough");
                    events.emit(VoiceEvent::Error(format!("correction fallback: {e}")));
                }
            }
        } else {
            match crate::correction::qwen_onnx::QwenCorrection::load(store, model_id) {
                Ok(c) => return Arc::new(c),
                Err(e) => {
                    log::warn!("correction '{model_id}' unavailable ({e}); using passthrough");
                    events.emit(VoiceEvent::Error(format!("correction fallback: {e}")));
                }
            }
        }
    }
    #[cfg(not(feature = "onnx"))]
    {
        let _ = (store, events);
        log::warn!(
            "onnx feature disabled; correction model '{model_id}' inactive (passthrough only)"
        );
    }
    Arc::new(Passthrough::default())
}

/// Build the wake-word detector from config. Falls back to [`NullWakeWord`]
/// (never triggers) when no real detector is available; in that case wake-word
/// trigger mode keeps capture gated and logs a warning.
fn build_wake_word(
    store: &ModelStore,
    config: &VoiceConfig,
    events: &Arc<dyn VoiceEventSink>,
) -> Arc<dyn WakeWordDetector> {
    #[cfg(feature = "onnx")]
    {
        match crate::wake_word::keyword_onnx::KeywordOnnx::load(
            store,
            &config.wake_word,
            &config.wake_word,
        ) {
            Ok(w) => return Arc::new(w),
            Err(e) => {
                log::warn!("wake-word '{}' unavailable ({e}); using null", config.wake_word);
                events.emit(VoiceEvent::Error(format!("wake-word fallback: {e}")));
            }
        }
    }
    #[cfg(not(feature = "onnx"))]
    {
        let _ = (store, events, config);
    }
    Arc::new(NullWakeWord::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AudioBackendType, VoiceConfig};

    fn null_config() -> VoiceConfig {
        let mut c = VoiceConfig::default();
        c.audio_backend = AudioBackendType::Null;
        c.model_dir = std::env::temp_dir().join("voxi-voice-test-models");
        c
    }

    #[test]
    fn builds_and_starts_with_null_backend() {
        let engine = VoiceEngine::new(null_config(), None);
        assert!(engine.take_transcript_receiver().is_some());
        // Second take yields None.
        assert!(engine.take_transcript_receiver().is_none());
        assert!(engine.start());
        assert!(engine.is_running());
        engine.stop();
        assert!(!engine.is_running());
    }

    #[test]
    fn speak_with_null_tts_is_ok() {
        let engine = VoiceEngine::new(null_config(), None);
        // Null TTS yields empty PCM => speak succeeds without playback.
        assert!(engine.speak("hello world").is_ok());
    }

    #[test]
    fn high_confidence_skips_correction() {
        let engine = VoiceEngine::new(null_config(), None);
        let p = &engine.pipeline;
        // At/above threshold, the raw transcript passes through unchanged.
        let out = super::maybe_correct(p, "  hello   world  ", 0.99);
        assert_eq!(out, "  hello   world  ");
    }

    #[test]
    fn low_confidence_runs_passthrough_correction() {
        let engine = VoiceEngine::new(null_config(), None);
        let p = &engine.pipeline;
        // Below threshold, the passthrough engine collapses whitespace.
        let out = super::maybe_correct(p, "hello   world", 0.10);
        assert_eq!(out, "hello world");
    }
}
