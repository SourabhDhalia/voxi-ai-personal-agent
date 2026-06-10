//! Agent factory — creates specialized agent instances from role definitions.



pub struct AgentFactory;

impl Default for AgentFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentFactory {
    pub fn new() -> Self {
        AgentFactory
    }

    /// Create a sub-agent session with the given role's system prompt.
    pub fn create_agent_session(role_name: &str, _system_prompt: &str) -> String {
        let session_id = format!(
            "agent_{}_{}",
            role_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                % 100000
        );
        log::debug!(
            "AgentFactory: created session '{}' for role '{}'",
            session_id,
            role_name
        );
        session_id
    }
}
