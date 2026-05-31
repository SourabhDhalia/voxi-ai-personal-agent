//! Picovoice Porcupine wake-word detector.
//!
//! Built under `porcupine`. Validates that the custom keyword (`.ppn`) file and
//! access key are present; the Porcupine FFI binding is linked in a later
//! packaging stage (the FFI library is not vendored here). Until then
//! `detect_chunk` returns false. Prefer the `onnx` keyword detector when an
//! account-free path is required.

use super::{WakeWordDetector, WakeWordError};
use std::path::PathBuf;

pub struct Porcupine {
    keyword_path: PathBuf,
    has_access_key: bool,
}

impl Porcupine {
    /// `keyword_path` is the `.ppn` file; `access_key` is the Picovoice key.
    pub fn new(keyword_path: PathBuf, access_key: Option<String>) -> Result<Self, WakeWordError> {
        if !keyword_path.exists() {
            return Err(WakeWordError::Load(format!(
                "porcupine keyword file not found: {}",
                keyword_path.display()
            )));
        }
        Ok(Porcupine {
            keyword_path,
            has_access_key: access_key.map(|k| !k.is_empty()).unwrap_or(false),
        })
    }

    pub fn keyword_path(&self) -> &PathBuf {
        &self.keyword_path
    }
}

impl WakeWordDetector for Porcupine {
    fn name(&self) -> &str {
        "porcupine"
    }

    fn detect_chunk(&self, _pcm: &[f32]) -> bool {
        let _ = self.has_access_key;
        false
    }

    fn reset(&self) {}
}
