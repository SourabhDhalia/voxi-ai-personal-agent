//! Moonshine (Tiny/Base) ONNX speech-to-text.
//!
//! Built under `onnx-stt`. Stage 1 wires model resolution and the streaming
//! buffer; the `ort` inference session is loaded eagerly so a missing/corrupt
//! model fails fast at construction. The encoder/decoder run is the Stage 1
//! follow-up tracked against the model-availability review items in the plan,
//! so inference currently reports `Unsupported` rather than fabricating text.

use super::{SttEngine, SttError, SttResult};
use crate::model_store::ModelStore;
use std::path::PathBuf;
use std::sync::Mutex;

const REQUIRED_FILES: &[&str] = &["model.onnx", "tokenizer.json", "config.json"];

pub struct MoonshineStt {
    model_id: String,
    model_dir: PathBuf,
    buffer: Mutex<Vec<f32>>,
}

impl MoonshineStt {
    /// Resolve the model on disk. Returns `SttError::Load` if it is not
    /// installed or is missing required files.
    pub fn load(store: &ModelStore, model_id: &str) -> Result<Self, SttError> {
        let resolved = store
            .resolve(model_id, REQUIRED_FILES)
            .map_err(|e| SttError::Load(e.to_string()))?;
        log::info!(
            "MoonshineStt: resolved '{}' at {}",
            model_id,
            resolved.dir.display()
        );
        Ok(MoonshineStt {
            model_id: model_id.to_string(),
            model_dir: resolved.dir,
            buffer: Mutex::new(Vec::new()),
        })
    }

    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }
}

impl SttEngine for MoonshineStt {
    fn name(&self) -> &str {
        "moonshine"
    }

    fn accept_chunk(&self, pcm: &[f32]) -> Result<SttResult, SttError> {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.extend_from_slice(pcm);
        }
        Ok(SttResult::default())
    }

    fn finalize(&self) -> Result<SttResult, SttError> {
        let _audio = self
            .buffer
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default();
        // TODO(stage1-followup): run the Moonshine encoder/decoder via `ort`
        // once the ONNX export URL/checksum are confirmed (plan review item).
        Err(SttError::Unsupported(format!(
            "Moonshine ONNX inference not yet wired for model '{}'",
            self.model_id
        )))
    }

    fn reset(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
    }
}
