//! Voice pipeline configuration and the parsing of channel `settings` JSON.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Selects the audio I/O backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioBackendType {
    /// Pick ALSA on embedded Linux, CPAL on desktop/macOS, else null.
    Auto,
    /// Force CPAL (cross-platform).
    Cpal,
    /// Force ALSA (embedded Linux).
    Alsa,
    /// No real audio — for tests/CI and headless devices.
    Null,
}

impl Default for AudioBackendType {
    fn default() -> Self {
        AudioBackendType::Auto
    }
}

impl AudioBackendType {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "cpal" => AudioBackendType::Cpal,
            "alsa" => AudioBackendType::Alsa,
            "null" | "none" | "off" => AudioBackendType::Null,
            _ => AudioBackendType::Auto,
        }
    }
}

/// What activates speech capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerMode {
    /// Voice-activity detection gates capture.
    Vad,
    /// Explicit push-to-talk start/stop.
    PushToTalk,
    /// Wake word ("hey voxi") arms capture (Stage 4).
    WakeWord,
}

impl Default for TriggerMode {
    fn default() -> Self {
        TriggerMode::Vad
    }
}

impl TriggerMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "pushtotalk" | "push_to_talk" | "ptt" => TriggerMode::PushToTalk,
            "wakeword" | "wake_word" | "wake" => TriggerMode::WakeWord,
            _ => TriggerMode::Vad,
        }
    }
}

/// Full voice pipeline configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub audio_backend: AudioBackendType,
    pub stt_model: String,
    pub tts_model: String,
    /// `None` in Stage 1; the correction engine arrives in Stage 3.
    pub correction_model: Option<String>,
    pub trigger_mode: TriggerMode,
    pub wake_word: String,
    pub language: String,
    pub sample_rate: u32,
    /// Skip correction when STT confidence is above this threshold.
    pub confidence_threshold: f32,
    pub model_dir: PathBuf,
    pub auto_download: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        VoiceConfig {
            enabled: false,
            audio_backend: AudioBackendType::Auto,
            stt_model: "moonshine-tiny".to_string(),
            tts_model: "kokoro-82m".to_string(),
            correction_model: None,
            trigger_mode: TriggerMode::Vad,
            wake_word: "hey voxi".to_string(),
            language: "auto".to_string(),
            sample_rate: 16_000,
            confidence_threshold: 0.85,
            model_dir: default_model_dir(),
            auto_download: true,
        }
    }
}

/// `~/.voxi/models/voice/` with a sane fallback when `$HOME` is unset.
pub fn default_model_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".voxi").join("models").join("voice")
}

impl VoiceConfig {
    /// Build a config from a channel `settings` JSON object. Unknown or missing
    /// keys fall back to [`VoiceConfig::default`].
    pub fn from_settings(settings: &serde_json::Value) -> Self {
        let d = VoiceConfig::default();
        let get_str = |k: &str| settings.get(k).and_then(|v| v.as_str());

        VoiceConfig {
            enabled: settings
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(d.enabled),
            audio_backend: get_str("audio_backend")
                .map(AudioBackendType::parse)
                .unwrap_or(d.audio_backend),
            stt_model: get_str("stt_model").unwrap_or(&d.stt_model).to_string(),
            tts_model: get_str("tts_model").unwrap_or(&d.tts_model).to_string(),
            correction_model: settings
                .get("correction_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            trigger_mode: get_str("trigger_mode")
                .map(TriggerMode::parse)
                .unwrap_or(d.trigger_mode),
            wake_word: get_str("wake_word").unwrap_or(&d.wake_word).to_string(),
            language: get_str("language").unwrap_or(&d.language).to_string(),
            sample_rate: settings
                .get("sample_rate")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(d.sample_rate),
            confidence_threshold: settings
                .get("confidence_threshold")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(d.confidence_threshold),
            model_dir: settings
                .get("model_dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or(d.model_dir),
            auto_download: settings
                .get("auto_download")
                .and_then(|v| v.as_bool())
                .unwrap_or(d.auto_download),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_backend_and_trigger() {
        assert_eq!(AudioBackendType::parse("CPAL"), AudioBackendType::Cpal);
        assert_eq!(AudioBackendType::parse("null"), AudioBackendType::Null);
        assert_eq!(AudioBackendType::parse("weird"), AudioBackendType::Auto);
        assert_eq!(TriggerMode::parse("ptt"), TriggerMode::PushToTalk);
        assert_eq!(TriggerMode::parse("wake_word"), TriggerMode::WakeWord);
    }

    #[test]
    fn from_settings_overrides_defaults() {
        let settings = serde_json::json!({
            "enabled": true,
            "audio_backend": "null",
            "stt_model": "moonshine-base",
            "trigger_mode": "push_to_talk",
            "sample_rate": 24000,
            "confidence_threshold": 0.9
        });
        let cfg = VoiceConfig::from_settings(&settings);
        assert!(cfg.enabled);
        assert_eq!(cfg.audio_backend, AudioBackendType::Null);
        assert_eq!(cfg.stt_model, "moonshine-base");
        assert_eq!(cfg.trigger_mode, TriggerMode::PushToTalk);
        assert_eq!(cfg.sample_rate, 24000);
        assert!((cfg.confidence_threshold - 0.9).abs() < 1e-6);
        // Untouched keys keep their defaults.
        assert_eq!(cfg.tts_model, "kokoro-82m");
        assert!(cfg.correction_model.is_none());
    }

    #[test]
    fn from_empty_settings_is_default() {
        let cfg = VoiceConfig::from_settings(&serde_json::json!({}));
        assert_eq!(cfg.sample_rate, 16_000);
        assert_eq!(cfg.audio_backend, AudioBackendType::Auto);
    }
}
