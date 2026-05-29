use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationKind {
    Tool,
    Skill,
}

impl RegistrationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegistrationKind::Tool => "tool",
            RegistrationKind::Skill => "skill",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegisteredPaths {
    #[serde(default)]
    pub tool_paths: Vec<String>,
    #[serde(default)]
    pub skill_paths: Vec<String>,
}

impl RegisteredPaths {
    pub fn load(config_dir: &Path) -> Self {
        let path = config_dir.join("registered_paths.json");
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self, config_dir: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(config_dir).map_err(|err| {
            format!(
                "Failed to create config dir '{}': {}",
                config_dir.display(),
                err
            )
        })?;
        let path = config_dir.join("registered_paths.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|err| format!("Failed to serialize registered paths: {}", err))?;
        std::fs::write(&path, content)
            .map_err(|err| format!("Failed to write '{}': {}", path.display(), err))?;
        Ok(path)
    }

    pub fn list_for_kind(&self, kind: RegistrationKind) -> &[String] {
        match kind {
            RegistrationKind::Tool => &self.tool_paths,
            RegistrationKind::Skill => &self.skill_paths,
        }
    }

    fn list_for_kind_mut(&mut self, kind: RegistrationKind) -> &mut Vec<String> {
        match kind {
            RegistrationKind::Tool => &mut self.tool_paths,
            RegistrationKind::Skill => &mut self.skill_paths,
        }
    }
}

fn normalize_registration_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Registration path cannot be empty".to_string());
    }

    let expanded = if trimmed == "~" || trimmed.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        if trimmed == "~" {
            home
        } else {
            format!("{}/{}", home, &trimmed[2..])
        }
    } else {
        trimmed.to_string()
    };

    let candidate = PathBuf::from(&expanded);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join(candidate)
    };

    Ok(absolute)
}

pub fn canonicalize_registration_path(raw: &str) -> Result<String, String> {
    let absolute = normalize_registration_path(raw)?;
    let canonical = std::fs::canonicalize(&absolute)
        .map_err(|err| format!("Failed to resolve '{}': {}", absolute.display(), err))?;
    if !canonical.exists() {
        return Err(format!("Path '{}' does not exist", canonical.display()));
    }
    if !canonical.is_dir() {
        return Err(format!("Path '{}' is not a directory", canonical.display()));
    }

    Ok(canonical.to_string_lossy().to_string())
}

pub fn best_effort_registration_path(raw: &str) -> Result<String, String> {
    let absolute = normalize_registration_path(raw)?;
    match std::fs::canonicalize(&absolute) {
        Ok(canonical) => Ok(canonical.to_string_lossy().to_string()),
        Err(_) => Ok(absolute.to_string_lossy().to_string()),
    }
}

pub fn register_path(
    config_dir: &Path,
    kind: RegistrationKind,
    raw_path: &str,
) -> Result<(RegisteredPaths, String), String> {
    let canonical = canonicalize_registration_path(raw_path)?;
    let mut registrations = RegisteredPaths::load(config_dir);
    let entries = registrations.list_for_kind_mut(kind);
    if !entries.iter().any(|existing| existing == &canonical) {
        entries.push(canonical.clone());
        entries.sort();
        entries.dedup();
    }
    let saved_path = registrations.save(config_dir)?;
    Ok((registrations, saved_path.to_string_lossy().to_string()))
}

pub fn unregister_path(
    config_dir: &Path,
    kind: RegistrationKind,
    raw_path: &str,
) -> Result<(RegisteredPaths, bool, String), String> {
    let canonical = best_effort_registration_path(raw_path)?;
    let mut registrations = RegisteredPaths::load(config_dir);
    let entries = registrations.list_for_kind_mut(kind);
    let before = entries.len();
    entries.retain(|entry| entry != &canonical);
    let removed = before != entries.len();
    let saved_path = registrations.save(config_dir)?;
    Ok((
        registrations,
        removed,
        saved_path.to_string_lossy().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_path_deduplicates_entries() {
        let dir = tempfile::tempdir().unwrap();
        let skills = dir.path().join("skills");
        std::fs::create_dir_all(&skills).unwrap();

        let (first, _) = register_path(
            dir.path(),
            RegistrationKind::Skill,
            skills.to_str().unwrap(),
        )
        .unwrap();
        let (second, _) = register_path(
            dir.path(),
            RegistrationKind::Skill,
            skills.to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(first.skill_paths.len(), 1);
        assert_eq!(second.skill_paths.len(), 1);
    }
}
