use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bash::BashExecutionPlan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    Read,
    Write,
    Execute,
    Network,
}

impl Default for PermissionScope {
    fn default() -> Self {
        Self::Read
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Ask,
    AllowAll,
    DenyAll,
    RepoPolicy,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    Low,
    Standard,
    Sensitive,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOutcome {
    Allowed,
    Denied,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecisionSource {
    Mode,
    PolicyRule,
    Validation,
    Sandbox,
    Prompt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPromptDecision {
    AllowOnce,
    DenyOnce,
    AllowAlways,
    DenyAlways,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PermissionRequest {
    pub scope: PermissionScope,
    pub target: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub minimum_level: PermissionLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bash_plan: Option<BashExecutionPlan>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionPromptRequest {
    pub request: PermissionRequest,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionPromptRecord {
    pub prompt: PermissionPromptRequest,
    pub decision: PermissionPromptDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionDecision {
    pub request: PermissionRequest,
    pub allowed: bool,
    pub outcome: PermissionOutcome,
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    pub source: PermissionDecisionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PermissionPromptRecord>,
}

impl PermissionDecision {
    pub fn allow(
        request: PermissionRequest,
        rationale: impl Into<String>,
        source: PermissionDecisionSource,
    ) -> Self {
        let rationale = rationale.into();
        Self {
            request,
            allowed: true,
            outcome: PermissionOutcome::Allowed,
            reasons: vec![rationale.clone()],
            rationale,
            source,
            matched_rule: None,
            prompt: None,
        }
    }

    pub fn deny(
        request: PermissionRequest,
        rationale: impl Into<String>,
        source: PermissionDecisionSource,
    ) -> Self {
        let rationale = rationale.into();
        Self {
            request,
            allowed: false,
            outcome: PermissionOutcome::Denied,
            reasons: vec![rationale.clone()],
            rationale,
            source,
            matched_rule: None,
            prompt: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bash::BashCommand;

    #[test]
    fn permission_decision_round_trip_serializes_cleanly() {
        let decision = PermissionDecision {
            request: PermissionRequest {
                scope: PermissionScope::Write,
                target: "rust/crates/tclaw-runtime/src/lib.rs".to_string(),
                reason: "update public exports".to_string(),
                tool_name: Some("apply_patch".to_string()),
                minimum_level: PermissionLevel::Sensitive,
                bash_plan: Some(BashExecutionPlan {
                    commands: vec![BashCommand {
                        program: "git".to_string(),
                        args: vec!["status".to_string()],
                        working_dir: None,
                    }],
                    require_clean_environment: true,
                }),
                metadata: BTreeMap::from([("origin".to_string(), "unit-test".to_string())]),
            },
            allowed: true,
            outcome: PermissionOutcome::Allowed,
            rationale: "repo policy allows local edits".to_string(),
            reasons: vec![
                "repo policy allows local edits".to_string(),
                "tool minimum level is sensitive".to_string(),
            ],
            source: PermissionDecisionSource::PolicyRule,
            matched_rule: Some("allow-local-edits".to_string()),
            prompt: Some(PermissionPromptRecord {
                prompt: PermissionPromptRequest {
                    request: PermissionRequest {
                        scope: PermissionScope::Write,
                        target: "rust/crates/tclaw-runtime/src/lib.rs".to_string(),
                        reason: "update public exports".to_string(),
                        tool_name: Some("apply_patch".to_string()),
                        minimum_level: PermissionLevel::Sensitive,
                        bash_plan: None,
                        metadata: BTreeMap::new(),
                    },
                    message: "Approve edit?".to_string(),
                    reasons: vec!["unit-test".to_string()],
                },
                decision: PermissionPromptDecision::AllowOnce,
            }),
        };

        let json = serde_json::to_string(&decision).expect("serialize decision");
        let restored: PermissionDecision =
            serde_json::from_str(&json).expect("deserialize decision");

        assert!(restored.allowed);
        assert_eq!(restored.request.scope, PermissionScope::Write);
        assert_eq!(restored.request.minimum_level, PermissionLevel::Sensitive);
        assert_eq!(restored.source, PermissionDecisionSource::PolicyRule);
    }
}
