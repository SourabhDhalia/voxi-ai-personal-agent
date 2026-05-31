//! Qwen2.5-0.5B ONNX transcript correction.
//!
//! Built under `onnx`. Resolves the model on disk; the `ort` token-generation
//! loop is wired once the ONNX export is confirmed (plan review item). Until
//! then `correct` reports `Unsupported`, and the engine falls back to the raw
//! transcript so utterances are never dropped.

use super::{CorrectionEngine, CorrectionError, CORRECTION_PROMPT};
use crate::model_store::ModelStore;
use std::path::PathBuf;

const REQUIRED_FILES: &[&str] = &[
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "generation_config.json",
];

pub struct QwenCorrection {
    model_id: String,
    model_dir: PathBuf,
}

impl QwenCorrection {
    pub fn load(store: &ModelStore, model_id: &str) -> Result<Self, CorrectionError> {
        let resolved = store
            .resolve(model_id, REQUIRED_FILES)
            .map_err(|e| CorrectionError::Load(e.to_string()))?;
        Ok(QwenCorrection {
            model_id: model_id.to_string(),
            model_dir: resolved.dir,
        })
    }

    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }

    /// The prompt that will be fed to the model for a given transcript.
    pub fn build_prompt(raw: &str) -> String {
        format!("{CORRECTION_PROMPT}{raw}")
    }
}

impl CorrectionEngine for QwenCorrection {
    fn name(&self) -> &str {
        "qwen2.5-0.5b"
    }

    fn correct(&self, _raw: &str) -> Result<String, CorrectionError> {
        Err(CorrectionError::Unsupported(format!(
            "Qwen ONNX correction not yet wired for model '{}'",
            self.model_id
        )))
    }
}
