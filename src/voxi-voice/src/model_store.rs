//! Local model resolution and the managed model registry.
//!
//! [`ModelStore`] resolves on-disk model directories and verifies required
//! files exist. [`ModelRegistry`] (Stage 2) parses `models.voice.json`, lists
//! installed vs available models, verifies SHA-256 checksums, removes models,
//! and downloads them through a pluggable [`Downloader`] (the daemon/CLI
//! injects a concrete HTTP fetcher; the core stays transport-agnostic and
//! offline-safe).

use crate::sha256::{to_hex, Sha256};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ModelStoreError {
    /// The model directory does not exist under `model_dir`.
    NotInstalled { model_id: String, expected: PathBuf },
    /// The directory exists but a required file is missing.
    MissingFile {
        model_id: String,
        file: String,
        path: PathBuf,
    },
}

impl fmt::Display for ModelStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelStoreError::NotInstalled { model_id, expected } => write!(
                f,
                "voice model '{}' is not installed (expected at {}); \
                 run `voxi model install {}` (Stage 2) or install it manually",
                model_id,
                expected.display(),
                model_id
            ),
            ModelStoreError::MissingFile {
                model_id,
                file,
                path,
            } => write!(
                f,
                "voice model '{}' is missing required file '{}' at {}",
                model_id,
                file,
                path.display()
            ),
        }
    }
}

impl std::error::Error for ModelStoreError {}

/// A resolved, on-disk model ready to be loaded by an engine.
#[derive(Clone, Debug)]
pub struct ResolvedModel {
    pub model_id: String,
    pub dir: PathBuf,
}

impl ResolvedModel {
    /// Absolute path to a file inside the model directory.
    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

/// Resolves model ids to local directories rooted at `model_dir`.
pub struct ModelStore {
    model_dir: PathBuf,
}

impl ModelStore {
    pub fn new(model_dir: impl Into<PathBuf>) -> Self {
        ModelStore {
            model_dir: model_dir.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.model_dir
    }

    /// Directory a given model id is expected to live in.
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        self.model_dir.join(model_id)
    }

    /// Returns true if the model directory exists at all.
    pub fn is_installed(&self, model_id: &str) -> bool {
        self.model_path(model_id).is_dir()
    }

    /// Resolve a model and verify every `required_files` entry is present.
    pub fn resolve(
        &self,
        model_id: &str,
        required_files: &[&str],
    ) -> Result<ResolvedModel, ModelStoreError> {
        let dir = self.model_path(model_id);
        if !dir.is_dir() {
            return Err(ModelStoreError::NotInstalled {
                model_id: model_id.to_string(),
                expected: dir,
            });
        }
        for file in required_files {
            let p = dir.join(file);
            if !p.exists() {
                return Err(ModelStoreError::MissingFile {
                    model_id: model_id.to_string(),
                    file: (*file).to_string(),
                    path: p,
                });
            }
        }
        Ok(ResolvedModel {
            model_id: model_id.to_string(),
            dir,
        })
    }
}

/// One entry in `models.voice.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub model_id: String,
    pub task: String,
    pub backend: String,
    #[serde(default)]
    pub size_mb: u64,
    #[serde(default)]
    pub device_class: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub recommended_sample_rate: Option<u32>,
    #[serde(default)]
    pub memory_mb: u64,
    #[serde(default)]
    pub download_url: String,
    /// Expected SHA-256 of `model.onnx` (hex). May be a placeholder until
    /// verified upstream; placeholders are skipped during verification.
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub version: String,
}

impl RegistryEntry {
    fn required_files(&self) -> Vec<&str> {
        self.files.iter().map(|s| s.as_str()).collect()
    }

    fn checksum_is_placeholder(&self) -> bool {
        self.sha256.is_empty()
            || self.sha256.starts_with('<')
            || self.sha256.eq_ignore_ascii_case("to-be-verified")
    }
}

#[derive(Deserialize)]
struct RegistryFile {
    models: Vec<RegistryEntry>,
}

/// Abstracts file download so the core stays transport-agnostic. The CLI/daemon
/// supplies a concrete HTTP (or `curl`-backed) implementation.
pub trait Downloader {
    /// Fetch `url` into `dest` (overwriting). Returns a human-readable error.
    fn fetch(&self, url: &str, dest: &Path) -> Result<(), String>;
}

/// Outcome of verifying a single model's files.
#[derive(Debug)]
pub struct VerifyReport {
    pub model_id: String,
    pub files_present: bool,
    pub checksum_ok: Option<bool>, // None = skipped (placeholder/unknown)
    pub detail: String,
}

/// A model entry annotated with its install state, for `list`.
#[derive(Debug)]
pub struct ModelStatus {
    pub entry: RegistryEntry,
    pub installed: bool,
}

#[derive(Debug)]
pub enum RegistryError {
    Read(String),
    Parse(String),
    UnknownModel(String),
    Io(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Read(s) => write!(f, "registry read error: {s}"),
            RegistryError::Parse(s) => write!(f, "registry parse error: {s}"),
            RegistryError::UnknownModel(s) => write!(f, "unknown model id: {s}"),
            RegistryError::Io(s) => write!(f, "io error: {s}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// The managed model registry over a [`ModelStore`].
pub struct ModelRegistry {
    entries: Vec<RegistryEntry>,
    store: ModelStore,
}

impl ModelRegistry {
    /// Load the registry JSON and root the store at `model_dir`.
    pub fn load(
        registry_path: impl AsRef<Path>,
        model_dir: impl Into<PathBuf>,
    ) -> Result<Self, RegistryError> {
        let raw = std::fs::read_to_string(registry_path.as_ref())
            .map_err(|e| RegistryError::Read(e.to_string()))?;
        let parsed: RegistryFile =
            serde_json::from_str(&raw).map_err(|e| RegistryError::Parse(e.to_string()))?;
        Ok(ModelRegistry {
            entries: parsed.models,
            store: ModelStore::new(model_dir),
        })
    }

    /// Build a registry directly from entries (tests / programmatic use).
    pub fn from_entries(entries: Vec<RegistryEntry>, model_dir: impl Into<PathBuf>) -> Self {
        ModelRegistry {
            entries,
            store: ModelStore::new(model_dir),
        }
    }

    pub fn store(&self) -> &ModelStore {
        &self.store
    }

    pub fn find(&self, model_id: &str) -> Option<&RegistryEntry> {
        self.entries.iter().find(|e| e.model_id == model_id)
    }

    /// All known models annotated with install state.
    pub fn list(&self) -> Vec<ModelStatus> {
        self.entries
            .iter()
            .map(|e| ModelStatus {
                entry: e.clone(),
                installed: self.store.is_installed(&e.model_id),
            })
            .collect()
    }

    /// Resolve a model by id using its declared file list.
    pub fn resolve(&self, model_id: &str) -> Result<ResolvedModel, RegistryError> {
        let entry = self
            .find(model_id)
            .ok_or_else(|| RegistryError::UnknownModel(model_id.to_string()))?;
        self.store
            .resolve(model_id, &entry.required_files())
            .map_err(|e| RegistryError::Io(e.to_string()))
    }

    /// Verify a model's files exist and (when a real checksum is present) that
    /// `model.onnx` matches the expected SHA-256.
    pub fn verify(&self, model_id: &str) -> Result<VerifyReport, RegistryError> {
        let entry = self
            .find(model_id)
            .ok_or_else(|| RegistryError::UnknownModel(model_id.to_string()))?;

        let dir = self.store.model_path(model_id);
        if !dir.is_dir() {
            return Ok(VerifyReport {
                model_id: model_id.to_string(),
                files_present: false,
                checksum_ok: None,
                detail: "not installed".to_string(),
            });
        }

        let mut missing = Vec::new();
        for file in &entry.files {
            if !dir.join(file).exists() {
                missing.push(file.clone());
            }
        }
        if !missing.is_empty() {
            return Ok(VerifyReport {
                model_id: model_id.to_string(),
                files_present: false,
                checksum_ok: None,
                detail: format!("missing files: {}", missing.join(", ")),
            });
        }

        if entry.checksum_is_placeholder() {
            return Ok(VerifyReport {
                model_id: model_id.to_string(),
                files_present: true,
                checksum_ok: None,
                detail: "files present; checksum not pinned (skipped)".to_string(),
            });
        }

        let onnx = dir.join("model.onnx");
        let actual = match hash_file(&onnx) {
            Ok(h) => h,
            Err(e) => return Err(RegistryError::Io(e)),
        };
        let ok = actual.eq_ignore_ascii_case(&entry.sha256);
        Ok(VerifyReport {
            model_id: model_id.to_string(),
            files_present: true,
            checksum_ok: Some(ok),
            detail: if ok {
                "checksum OK".to_string()
            } else {
                format!("checksum MISMATCH (got {actual})")
            },
        })
    }

    /// Remove an installed model directory.
    pub fn remove(&self, model_id: &str) -> Result<(), RegistryError> {
        let dir = self.store.model_path(model_id);
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| RegistryError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// Download every file of a model via `downloader`, then verify. Files are
    /// fetched from `download_url` + filename.
    pub fn download(
        &self,
        model_id: &str,
        downloader: &dyn Downloader,
    ) -> Result<VerifyReport, RegistryError> {
        let entry = self
            .find(model_id)
            .ok_or_else(|| RegistryError::UnknownModel(model_id.to_string()))?
            .clone();
        if entry.download_url.is_empty() {
            return Err(RegistryError::Io(format!(
                "model '{model_id}' has no download_url; install manually"
            )));
        }
        let dir = self.store.model_path(model_id);
        std::fs::create_dir_all(&dir).map_err(|e| RegistryError::Io(e.to_string()))?;

        let base = if entry.download_url.ends_with('/') {
            entry.download_url.clone()
        } else {
            format!("{}/", entry.download_url)
        };
        for file in &entry.files {
            let url = format!("{base}{file}");
            let dest = dir.join(file);
            log::info!("downloading {url} -> {}", dest.display());
            downloader
                .fetch(&url, &dest)
                .map_err(|e| RegistryError::Io(format!("download {file}: {e}")))?;
        }
        self.verify(model_id)
    }
}

/// Stream a file through SHA-256 and return the hex digest.
fn hash_file(path: &Path) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_is_not_installed() {
        let store = ModelStore::new(std::env::temp_dir().join("voxi-voice-nope-xyz"));
        assert!(!store.is_installed("moonshine-tiny"));
        let err = store.resolve("moonshine-tiny", &["model.onnx"]).unwrap_err();
        matches!(err, ModelStoreError::NotInstalled { .. });
        // Error message points the user at the install path.
        assert!(err.to_string().contains("moonshine-tiny"));
    }

    #[test]
    fn resolves_when_files_present() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(tmp.path());
        let mdir = store.model_path("kokoro-82m");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(mdir.join("model.onnx"), b"stub").unwrap();
        std::fs::write(mdir.join("voices.bin"), b"stub").unwrap();

        let resolved = store
            .resolve("kokoro-82m", &["model.onnx", "voices.bin"])
            .unwrap();
        assert_eq!(resolved.model_id, "kokoro-82m");
        assert_eq!(resolved.file("model.onnx"), mdir.join("model.onnx"));

        // A missing file is reported precisely.
        let err = store
            .resolve("kokoro-82m", &["model.onnx", "config.json"])
            .unwrap_err();
        assert!(err.to_string().contains("config.json"));
    }

    fn sample_entry(id: &str, sha: &str) -> RegistryEntry {
        RegistryEntry {
            model_id: id.to_string(),
            task: "stt".into(),
            backend: "onnx".into(),
            size_mb: 27,
            device_class: "embedded".into(),
            language: "multilingual".into(),
            recommended_sample_rate: Some(16000),
            memory_mb: 150,
            download_url: "https://example.invalid/model/".into(),
            sha256: sha.to_string(),
            files: vec!["model.onnx".into()],
            version: "1.0.0".into(),
        }
    }

    #[test]
    fn registry_lists_and_verifies() {
        let tmp = tempfile::tempdir().unwrap();
        // Real checksum of b"hello" content.
        let content = b"hello";
        let expected = crate::sha256::hex_digest(content);
        let reg = ModelRegistry::from_entries(
            vec![
                sample_entry("with-checksum", &expected),
                sample_entry("placeholder", "<to-be-verified>"),
            ],
            tmp.path(),
        );

        // Nothing installed yet.
        let list = reg.list();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|m| !m.installed));

        // Install "with-checksum" by hand and verify checksum passes.
        let dir = reg.store().model_path("with-checksum");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("model.onnx"), content).unwrap();
        let report = reg.verify("with-checksum").unwrap();
        assert!(report.files_present);
        assert_eq!(report.checksum_ok, Some(true));

        // Placeholder checksum -> skipped, but files must be present.
        let pdir = reg.store().model_path("placeholder");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(pdir.join("model.onnx"), b"anything").unwrap();
        let preport = reg.verify("placeholder").unwrap();
        assert!(preport.files_present);
        assert_eq!(preport.checksum_ok, None);

        // Remove works.
        reg.remove("with-checksum").unwrap();
        assert!(!reg.store().is_installed("with-checksum"));

        // Unknown model id errors.
        assert!(reg.verify("nope").is_err());
    }

    #[test]
    fn registry_parses_json() {
        let json = r#"{ "models": [
            { "model_id": "moonshine-tiny", "task": "stt", "backend": "onnx",
              "size_mb": 27, "files": ["model.onnx", "tokenizer.json"],
              "download_url": "https://example.invalid/m/", "sha256": "<to-be-verified>" }
        ]}"#;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("models.voice.json");
        std::fs::write(&path, json).unwrap();
        let reg = ModelRegistry::load(&path, tmp.path()).unwrap();
        assert!(reg.find("moonshine-tiny").is_some());
        assert_eq!(reg.list().len(), 1);
    }
}
