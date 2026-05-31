//! Keyword-spotting ONNX wake-word detector (open-source, no account needed).
//!
//! Built under `onnx`. Resolves a small keyword-spotting model on disk; the
//! `ort` scoring loop is wired in a later stage. Until then `detect_chunk`
//! returns false (capture stays gated), matching the null detector's behaviour
//! but with the model assets validated up front.

use super::{WakeWordDetector, WakeWordError};
use crate::model_store::ModelStore;
use std::path::PathBuf;

const REQUIRED_FILES: &[&str] = &["model.onnx"];

pub struct KeywordOnnx {
    keyword: String,
    model_dir: PathBuf,
}

impl KeywordOnnx {
    pub fn load(store: &ModelStore, model_id: &str, keyword: &str) -> Result<Self, WakeWordError> {
        let resolved = store
            .resolve(model_id, REQUIRED_FILES)
            .map_err(|e| WakeWordError::Load(e.to_string()))?;
        Ok(KeywordOnnx {
            keyword: keyword.to_string(),
            model_dir: resolved.dir,
        })
    }

    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }
}

impl WakeWordDetector for KeywordOnnx {
    fn name(&self) -> &str {
        "keyword-onnx"
    }

    fn detect_chunk(&self, _pcm: &[f32]) -> bool {
        false
    }

    fn reset(&self) {}
}
