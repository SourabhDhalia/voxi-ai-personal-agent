/// Thread-safe AgentCore with fine-grained internal locking.
///
/// Callers share `Arc<AgentCore>` — no outer Mutex needed.
/// Each field that requires mutation is individually protected:
/// - `provider_registry`: RwLock — owns all initialized LLM backends and
///   the configured preference order for request-time routing.
/// - `session_store`: Mutex (SQLite is not Sync)
/// - `tool_dispatcher`: RwLock (reads are frequent, writes are rare)
pub struct AgentCore {
    platform: Arc<libtizenclaw_core::framework::PlatformContext>,
    /// Provider registry — owns all initialized backends in preference order.
    /// Replaces the former `backend` + `fallback_backends` + `backend_name`
    /// flat fields.  `ProviderSelector` picks the first available provider at
    /// request time using the circuit-breaker state in `circuit_breakers`.
    provider_registry: tokio::sync::RwLock<crate::core::provider_selection::ProviderRegistry>,
    session_store: Mutex<Option<SessionStore>>,
    tool_dispatcher: tokio::sync::RwLock<ToolDispatcher>,
    safety_guard: Arc<Mutex<SafetyGuard>>,
    context_engine: Arc<SizedContextEngine>,
    event_bus: Arc<EventBus>,
    key_store: Mutex<KeyStore>,
    system_prompt: RwLock<String>,
    soul_content: RwLock<Option<String>>,
    llm_config: Mutex<LlmConfig>,
    circuit_breakers: RwLock<std::collections::HashMap<String, CircuitBreakerState>>,
    action_bridge: Mutex<crate::core::action_bridge::ActionBridge>,
    tool_policy: Mutex<crate::core::tool_policy::ToolPolicy>,
    memory_store: Mutex<Option<crate::storage::memory_store::MemoryStore>>,
    tool_embedding_store: Mutex<Option<crate::storage::embedding_store::EmbeddingStore>>,
    workflow_engine: tokio::sync::RwLock<crate::core::workflow_engine::WorkflowEngine>,
    agent_roles: RwLock<AgentRoleRegistry>,
    session_profiles: Mutex<HashMap<String, SessionPromptProfile>>,
    /// Hash of the last system_prompt sent to the backend.
    /// Used to detect when the prompt changes so that the server-side
    /// cached content can be refreshed (e.g. Gemini CachedContent API).
    prompt_hash: tokio::sync::RwLock<u64>,
    mcp_client_manager: tokio::sync::RwLock<crate::channel::mcp_client::McpClientManager>,
    pending_mcp_confirmations: Mutex<HashMap<String, PendingMcpConfirmation>>,
    llm_response_cache: LlmResponseCache,
    active_requests: Arc<Mutex<HashMap<String, RequestState>>>,
    session_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

#[derive(Clone, Debug)]
pub struct PendingMcpConfirmation {
    pub session_id: String,
    pub tool_name: String,
    pub args: Value,
    pub timestamp_ms: u64,
    pub tool_call_id: String,
}

#[derive(Clone, Debug)]
pub struct LlmResponseCacheEntry {
    pub response: LlmResponse,
    pub created_at_ms: u64,
    pub last_used_ms: u64,
}

pub type LlmResponseCache = Arc<RwLock<HashMap<u64, LlmResponseCacheEntry>>>;

#[derive(Clone, Debug)]
pub struct RequestState {
    pub session_id: String,
    pub request_id: String,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
}
