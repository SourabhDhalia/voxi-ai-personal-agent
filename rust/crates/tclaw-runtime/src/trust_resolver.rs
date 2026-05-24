use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    Unknown,
    Restricted,
    Trusted,
    Privileged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustSubjectKind {
    Worker,
    Task,
    Lane,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustSubject {
    pub kind: TrustSubjectKind,
    pub id: String,
}

impl TrustSubject {
    pub fn new(kind: TrustSubjectKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustRequirement {
    pub minimum_level: TrustLevel,
}

impl TrustRequirement {
    pub fn at_least(minimum_level: TrustLevel) -> Self {
        Self { minimum_level }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustFailureReason {
    InsufficientLevel {
        required: TrustLevel,
        actual: TrustLevel,
    },
    ExplicitDeny {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustDecision {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustResolution {
    pub subject: TrustSubject,
    pub requirement: TrustRequirement,
    pub actual_level: TrustLevel,
    pub decision: TrustDecision,
    pub failure: Option<TrustFailureReason>,
    pub reason: String,
}

impl TrustResolution {
    pub fn is_allowed(&self) -> bool {
        self.decision == TrustDecision::Allowed
    }
}

#[derive(Debug, Clone, Default)]
pub struct TrustResolver;

impl TrustResolver {
    pub fn resolve(
        subject: TrustSubject,
        requirement: TrustRequirement,
        actual_level: TrustLevel,
    ) -> TrustResolution {
        if actual_level >= requirement.minimum_level {
            TrustResolution {
                subject,
                requirement,
                actual_level,
                decision: TrustDecision::Allowed,
                failure: None,
                reason: "trust requirement satisfied".to_string(),
            }
        } else {
            Self::deny(
                subject,
                requirement.clone(),
                actual_level.clone(),
                TrustFailureReason::InsufficientLevel {
                    required: requirement.minimum_level,
                    actual: actual_level,
                },
                "trust requirement not satisfied",
            )
        }
    }

    pub fn deny(
        subject: TrustSubject,
        requirement: TrustRequirement,
        actual_level: TrustLevel,
        failure: TrustFailureReason,
        reason: impl Into<String>,
    ) -> TrustResolution {
        TrustResolution {
            subject,
            requirement,
            actual_level,
            decision: TrustDecision::Denied,
            failure: Some(failure),
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_resolver_accepts_when_requirement_is_met() {
        let resolution = TrustResolver::resolve(
            TrustSubject::new(TrustSubjectKind::Worker, "worker-1"),
            TrustRequirement::at_least(TrustLevel::Restricted),
            TrustLevel::Trusted,
        );

        assert!(resolution.is_allowed());
        assert_eq!(resolution.failure, None);
        assert_eq!(resolution.reason, "trust requirement satisfied");
    }

    #[test]
    fn trust_resolver_reports_explicit_failure_reason() {
        let resolution = TrustResolver::resolve(
            TrustSubject::new(TrustSubjectKind::Task, "task-9"),
            TrustRequirement::at_least(TrustLevel::Privileged),
            TrustLevel::Restricted,
        );

        assert_eq!(resolution.decision, TrustDecision::Denied);
        assert_eq!(
            resolution.failure,
            Some(TrustFailureReason::InsufficientLevel {
                required: TrustLevel::Privileged,
                actual: TrustLevel::Restricted,
            })
        );
        assert_eq!(resolution.reason, "trust requirement not satisfied");
    }

    #[test]
    fn trust_resolution_round_trips_through_json() {
        let resolution = TrustResolver::deny(
            TrustSubject::new(TrustSubjectKind::Lane, "lane-a"),
            TrustRequirement::at_least(TrustLevel::Trusted),
            TrustLevel::Unknown,
            TrustFailureReason::ExplicitDeny {
                message: "manual approval required".to_string(),
            },
            "approval gate blocked execution",
        );

        let json = serde_json::to_string(&resolution).expect("serialize trust resolution");
        let restored: TrustResolution =
            serde_json::from_str(&json).expect("deserialize trust resolution");

        assert_eq!(restored.decision, TrustDecision::Denied);
        assert_eq!(restored.reason, "approval gate blocked execution");
    }
}
