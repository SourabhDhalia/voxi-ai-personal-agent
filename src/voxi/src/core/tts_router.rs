use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsEngineType {
    KokoroBrowser,
    PocketLocal,
    Indextts2Local,
    CloudTts,
}

impl TtsEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsEngineType::KokoroBrowser => "kokoro_browser",
            TtsEngineType::PocketLocal => "pocket_local",
            TtsEngineType::Indextts2Local => "indextts2_local",
            TtsEngineType::CloudTts => "cloud_tts",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "kokoro_browser" => Some(TtsEngineType::KokoroBrowser),
            "pocket_local" => Some(TtsEngineType::PocketLocal),
            "indextts2_local" => Some(TtsEngineType::Indextts2Local),
            "cloud_tts" => Some(TtsEngineType::CloudTts),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TtsIntent {
    Chat,
    Assistant,
    Storytelling,
}

impl TtsIntent {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "story" | "storytelling" | "expressive" => TtsIntent::Storytelling,
            "assistant" | "command" | "system" => TtsIntent::Assistant,
            _ => TtsIntent::Chat,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TtsEngineConfig {
    pub model_path: Option<String>,
    pub tokenizer_path: Option<String>,
    pub voice: Option<String>,
    pub gender: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub engine: String,
    pub language: String,
    pub speak_replies: bool,
    pub browser_voice: String,
    pub available_engines: Vec<String>,
    pub priorities: BTreeMap<String, String>,
    pub engines: BTreeMap<String, TtsEngineConfig>,
    pub fallback_order: Vec<String>,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        let mut priorities = BTreeMap::new();
        priorities.insert("chat".to_string(), "kokoro_browser".to_string());
        priorities.insert("assistant".to_string(), "pocket_local".to_string());
        priorities.insert("storytelling".to_string(), "indextts2_local".to_string());

        let mut engines = BTreeMap::new();
        engines.insert(
            "kokoro_browser".to_string(),
            TtsEngineConfig {
                model_path: Some("models/kokoro-82m.onnx".to_string()),
                tokenizer_path: None,
                voice: Some("af_bella".to_string()),
                gender: Some("female".to_string()),
                enabled: true,
            },
        );
        engines.insert(
            "pocket_local".to_string(),
            TtsEngineConfig {
                model_path: Some("models/pocket-voice.onnx".to_string()),
                tokenizer_path: None,
                voice: Some("en_US-lessac-medium".to_string()),
                gender: Some("male".to_string()),
                enabled: true,
            },
        );
        engines.insert(
            "indextts2_local".to_string(),
            TtsEngineConfig {
                model_path: Some("models/style_tts2.onnx".to_string()),
                tokenizer_path: None,
                voice: Some("expressive_cloned".to_string()),
                gender: Some("female".to_string()),
                enabled: true,
            },
        );
        engines.insert(
            "cloud_tts".to_string(),
            TtsEngineConfig {
                model_path: None,
                tokenizer_path: None,
                voice: Some("alloy".to_string()),
                gender: Some("neutral".to_string()),
                enabled: false,
            },
        );

        VoiceConfig {
            engine: "browser".to_string(),
            language: "en-US".to_string(),
            speak_replies: true,
            browser_voice: "".to_string(),
            available_engines: vec![
                "kokoro_browser".to_string(),
                "pocket_local".to_string(),
                "indextts2_local".to_string(),
                "cloud_tts".to_string(),
            ],
            priorities,
            engines,
            fallback_order: vec![
                "indextts2_local".to_string(),
                "pocket_local".to_string(),
                "kokoro_browser".to_string(),
            ],
        }
    }
}

pub struct TtsRouter {
    config: VoiceConfig,
}

impl TtsRouter {
    pub fn new(config: VoiceConfig) -> Self {
        TtsRouter { config }
    }

    pub fn load(config_dir: &Path) -> Self {
        let path = config_dir.join("voice_config.json");
        if !path.exists() {
            return Self::new(VoiceConfig::default());
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => Self::new(config),
                Err(e) => {
                    log::error!("Failed to parse voice_config.json: {}", e);
                    Self::new(VoiceConfig::default())
                }
            },
            Err(e) => {
                log::error!("Failed to read voice_config.json: {}", e);
                Self::new(VoiceConfig::default())
            }
        }
    }

    /// Selects the best TTS engine according to the intent and available engines.
    /// Available engines are provided by checking runtime availability.
    pub fn route(&self, intent: TtsIntent, available_engines: &[String]) -> Option<String> {
        if !self.config.speak_replies {
            return None;
        }

        // 1. Determine preferred engine based on intent priorities
        let intent_key = match intent {
            TtsIntent::Chat => "chat",
            TtsIntent::Assistant => "assistant",
            TtsIntent::Storytelling => "storytelling",
        };

        if let Some(preferred) = self.config.priorities.get(intent_key) {
            if available_engines.contains(preferred) {
                if let Some(engine_cfg) = self.config.engines.get(preferred) {
                    if engine_cfg.enabled {
                        return Some(preferred.clone());
                    }
                }
            }
        }

        // 2. Fall back through priorities fallback list
        for fallback in &self.config.fallback_order {
            if available_engines.contains(fallback) {
                if let Some(engine_cfg) = self.config.engines.get(fallback) {
                    if engine_cfg.enabled {
                        return Some(fallback.clone());
                    }
                }
            }
        }

        // 3. Fall back to warning and no engine
        log::warn!("No suitable TTS engine found or enabled for intent {:?}. Text-only fallback.", intent);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_router_routing_logic() {
        let config = VoiceConfig::default();
        let router = TtsRouter::new(config);

        // Standard available engines
        let available = vec![
            "kokoro_browser".to_string(),
            "pocket_local".to_string(),
            "indextts2_local".to_string(),
        ];

        // Normal chat -> kokoro_browser
        assert_eq!(
            router.route(TtsIntent::Chat, &available),
            Some("kokoro_browser".to_string())
        );

        // Assistant -> pocket_local
        assert_eq!(
            router.route(TtsIntent::Assistant, &available),
            Some("pocket_local".to_string())
        );

        // Storytelling -> indextts2_local
        assert_eq!(
            router.route(TtsIntent::Storytelling, &available),
            Some("indextts2_local".to_string())
        );
    }

    #[test]
    fn test_tts_router_fallback_logic() {
        let config = VoiceConfig::default();
        let router = TtsRouter::new(config);

        // storytelling preferred is indextts2_local. If it's missing, fall back to pocket_local.
        let available_no_story = vec![
            "kokoro_browser".to_string(),
            "pocket_local".to_string(),
        ];
        assert_eq!(
            router.route(TtsIntent::Storytelling, &available_no_story),
            Some("pocket_local".to_string())
        );

        // If pocket_local is also missing, fall back to kokoro_browser
        let available_only_browser = vec!["kokoro_browser".to_string()];
        assert_eq!(
            router.route(TtsIntent::Storytelling, &available_only_browser),
            Some("kokoro_browser".to_string())
        );

        // If everything is missing, return None (text only)
        assert_eq!(
            router.route(TtsIntent::Storytelling, &[]),
            None
        );
    }

    #[test]
    fn test_disabled_engines_are_skipped() {
        let mut config = VoiceConfig::default();
        // Disable pocket_local
        if let Some(engine) = config.engines.get_mut("pocket_local") {
            engine.enabled = false;
        }

        let router = TtsRouter::new(config);
        let available = vec![
            "kokoro_browser".to_string(),
            "pocket_local".to_string(),
            "indextts2_local".to_string(),
        ];

        // Assistant should fall back to pocket_local -> but it's disabled, so fallback to kokoro_browser
        // Wait, standard fallback list: ["indextts2_local", "pocket_local", "kokoro_browser"]
        // So first fallback check in fallback_order is indextts2_local, which is enabled!
        assert_eq!(
            router.route(TtsIntent::Assistant, &available),
            Some("indextts2_local".to_string())
        );
    }
}
