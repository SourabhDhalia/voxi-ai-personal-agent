//! Kokoro-82M ONNX text-to-speech.
//!
//! Built under `onnx-tts`. Stage 1 resolves `model.onnx` + `voices.bin` on
//! disk and selects a voice; the `ort` synthesis run is the Stage 1 follow-up
//! (pending the ONNX URL/checksum/license review in the plan), so synthesis
//! currently reports `Unsupported` rather than returning silence.

use super::{TtsAudio, TtsEngine, TtsError};
use crate::model_store::ModelStore;
use std::path::PathBuf;

const REQUIRED_FILES: &[&str] = &["model.onnx", "voices.bin", "config.json"];
const DEFAULT_VOICE: &str = "af_heart";
const NATIVE_SAMPLE_RATE: u32 = 24_000;

pub struct KokoroTts {
    model_id: String,
    model_dir: PathBuf,
    voice: String,
}

impl KokoroTts {
    pub fn load(store: &ModelStore, model_id: &str, voice: Option<&str>) -> Result<Self, TtsError> {
        let resolved = store
            .resolve(model_id, REQUIRED_FILES)
            .map_err(|e| TtsError::Load(e.to_string()))?;
        log::info!(
            "KokoroTts: resolved '{}' at {}",
            model_id,
            resolved.dir.display()
        );
        Ok(KokoroTts {
            model_id: model_id.to_string(),
            model_dir: resolved.dir,
            voice: voice.unwrap_or(DEFAULT_VOICE).to_string(),
        })
    }

    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }

    pub fn voice(&self) -> &str {
        &self.voice
    }
}

impl TtsEngine for KokoroTts {
    fn name(&self) -> &str {
        "kokoro"
    }

    fn sample_rate(&self) -> u32 {
        NATIVE_SAMPLE_RATE
    }

    fn synthesize(&self, _text: &str) -> Result<TtsAudio, TtsError> {
        // TODO(stage1-followup): run Kokoro phonemizer + `ort` decoder using
        // the selected voice embedding from `voices.bin`.
        Err(TtsError::Unsupported(format!(
            "Kokoro ONNX synthesis not yet wired for model '{}' (voice '{}')",
            self.model_id, self.voice
        )))
    }
}
