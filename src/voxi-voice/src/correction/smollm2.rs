//! SmolLM2-360M ONNX transcript correction (ultra-low-power alternative).
//!
//! Built under `onnx`. Same contract as the Qwen engine; inference is wired in
//! a later stage. Falls back to the raw transcript via the engine's error path.

use super::{CorrectionEngine, CorrectionError, CORRECTION_PROMPT};
use crate::model_store::ModelStore;
use std::path::PathBuf;

const REQUIRED_FILES: &[&str] = &["model.onnx", "tokenizer.json", "config.json"];

pub struct SmolLm2Correction {
    model_id: String,
    model_dir: PathBuf,
}

impl SmolLm2Correction {
    pub fn load(store: &ModelStore, model_id: &str) -> Result<Self, CorrectionError> {
        let resolved = store
            .resolve(model_id, REQUIRED_FILES)
            .map_err(|e| CorrectionError::Load(e.to_string()))?;
        Ok(SmolLm2Correction {
            model_id: model_id.to_string(),
            model_dir: resolved.dir,
        })
    }

    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }

    pub fn build_prompt(raw: &str) -> String {
        format!("{CORRECTION_PROMPT}{raw}")
    }
}

impl CorrectionEngine for SmolLm2Correction {
    fn name(&self) -> &str {
        "smollm2-360m"
    }

    fn correct(&self, _raw: &str) -> Result<String, CorrectionError> {
        Err(CorrectionError::Unsupported(format!(
            "SmolLM2 ONNX correction not yet wired for model '{}'",
            self.model_id
        )))
    }
}
