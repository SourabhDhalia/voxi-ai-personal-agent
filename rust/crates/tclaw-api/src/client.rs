use std::sync::Arc;

use crate::error::ApiError;
use crate::types::{ChatRequest, ChatResponse, ProviderKind, StreamEvent};

pub type EventStream = Box<dyn Iterator<Item = Result<StreamEvent, ApiError>> + Send>;

pub trait ProviderClient: Send + Sync {
    fn provider(&self) -> ProviderKind;
    fn send(&self, request: &ChatRequest) -> Result<ChatResponse, ApiError>;
    fn stream(&self, request: &ChatRequest) -> Result<EventStream, ApiError>;
}

#[derive(Clone)]
pub struct ApiClient {
    provider: Arc<dyn ProviderClient>,
}

impl ApiClient {
    pub fn new(provider: Arc<dyn ProviderClient>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> ProviderKind {
        self.provider.provider()
    }

    pub fn send(&self, request: &ChatRequest) -> Result<ChatResponse, ApiError> {
        self.provider.send(request)
    }

    pub fn stream(&self, request: &ChatRequest) -> Result<EventStream, ApiError> {
        self.provider.stream(request)
    }
}
