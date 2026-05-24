use std::sync::{Arc, Mutex};

use tclaw_api::{
    ApiClient, ApiError, ChatMessage, ChatRequest, ChatResponse, EventStream, FinishReason,
    ProviderClient, ProviderKind, ResponseMetadata, StreamEvent,
};

#[derive(Default)]
struct RecordingProvider {
    sent_models: Mutex<Vec<String>>,
}

impl ProviderClient for RecordingProvider {
    fn provider(&self) -> ProviderKind {
        ProviderKind::OpenAiCompat
    }

    fn send(&self, request: &ChatRequest) -> Result<ChatResponse, ApiError> {
        self.sent_models
            .lock()
            .expect("mutex")
            .push(request.model.clone());
        Ok(ChatResponse {
            metadata: ResponseMetadata {
                provider: ProviderKind::OpenAiCompat,
                model: request.model.clone(),
                id: Some("resp-1".to_string()),
            },
            content: vec![],
            finish_reason: FinishReason::stop(),
            usage: None,
            stop_sequence: None,
            raw: None,
        })
    }

    fn stream(&self, request: &ChatRequest) -> Result<EventStream, ApiError> {
        self.sent_models
            .lock()
            .expect("mutex")
            .push(format!("stream:{}", request.model));
        Ok(Box::new(
            vec![Ok(StreamEvent::MessageStop {
                finish_reason: FinishReason::stop(),
            })]
            .into_iter(),
        ))
    }
}

fn sample_request() -> ChatRequest {
    ChatRequest {
        model: "gpt-test".to_string(),
        system: None,
        messages: vec![ChatMessage::user_text("hello")],
        tools: vec![],
        stream: false,
        temperature: None,
        max_output_tokens: None,
        response_format: None,
        prompt_cache: None,
        metadata: None,
    }
}

#[test]
fn api_client_delegates_send_and_stream_to_the_provider() {
    let provider = Arc::new(RecordingProvider::default());
    let client = ApiClient::new(provider.clone());
    let request = sample_request();

    let response = client.send(&request).expect("send");
    let events = client.stream(&request).expect("stream").count();

    assert_eq!(client.provider(), ProviderKind::OpenAiCompat);
    assert_eq!(response.metadata.model, "gpt-test");
    assert_eq!(events, 1);
    assert_eq!(
        provider.sent_models.lock().expect("mutex").clone(),
        vec!["gpt-test".to_string(), "stream:gpt-test".to_string()]
    );
}
