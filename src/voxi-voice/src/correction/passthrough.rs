//! Passthrough correction — returns the transcript unchanged.
//!
//! Used as the host-default and as the graceful fallback when no correction
//! model is installed. It also applies a couple of safe, model-free cleanups
//! (trimming and whitespace collapse) that never change meaning.

use super::{CorrectionEngine, CorrectionError};

#[derive(Default)]
pub struct Passthrough;

impl CorrectionEngine for Passthrough {
    fn name(&self) -> &str {
        "passthrough"
    }

    fn correct(&self, raw: &str) -> Result<String, CorrectionError> {
        // Collapse runs of whitespace and trim; purely cosmetic, meaning-safe.
        let cleaned = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace() {
        let c = Passthrough::default();
        assert_eq!(c.correct("  hello   world \n").unwrap(), "hello world");
    }
}
