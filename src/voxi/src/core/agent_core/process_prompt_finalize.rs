impl AgentCore {
    pub async fn verify_candidate_response(
        &self,
        prompt: &str,
        candidate_text: &str,
    ) -> Option<String> {
        let verification_system_prompt = "You are a rigorous response validator. Evaluate whether the candidate response completely and accurately answers the original prompt. If correct, output PASS. Otherwise, output corrections.";
        let user_msg = format!("Original User Query:\n{}\n\nCandidate Response:\n{}", prompt, candidate_text);
        let messages = vec![LlmMessage::user(&user_msg)];
        let response = self.chat_with_fallback(&messages, &[], None, verification_system_prompt, Some(1024)).await;
        if response.success {
            let text = response.text.trim();
            if text.to_ascii_lowercase().starts_with("pass") {
                None
            } else {
                Some(text.to_string())
            }
        } else {
            log::warn!("Verification call failed: {}", response.error_message);
            None
        }
    }

    pub async fn summarize_and_store_pruned_messages(
        &self,
        session_id: &str,
        original_messages: &[LlmMessage],
        compacted_messages: &[LlmMessage],
    ) {
        let before_len = original_messages.len();
        let after_len = compacted_messages.len();
        if after_len >= before_len {
            return;
        }

        // Find which messages were dropped
        let mut dropped_messages = Vec::new();
        let mut comp_idx = 0;
        for msg in original_messages {
            if comp_idx < compacted_messages.len()
                && msg.role == compacted_messages[comp_idx].role
                && msg.text == compacted_messages[comp_idx].text
                && msg.tool_call_id == compacted_messages[comp_idx].tool_call_id
            {
                comp_idx += 1;
            } else {
                dropped_messages.push(msg.clone());
            }
        }

        if dropped_messages.is_empty() {
            return;
        }

        let mut snippet = String::new();
        for msg in &dropped_messages {
            if msg.role == "system" {
                continue;
            }
            snippet.push_str(&format!("{}: {}\n", msg.role, msg.text));
            if !msg.tool_calls.is_empty() {
                snippet.push_str(&format!("  Tool Calls: {:?}\n", msg.tool_calls));
            }
            if msg.role == "tool" {
                snippet.push_str(&format!("  Tool Result: {}\n", msg.tool_result));
            }
        }

        if snippet.is_empty() {
            return;
        }

        let summarizer_system_prompt = "You are an episodic memory summarizer. Extract key facts, accomplishments, and decisions from the following conversation snippet in a bulleted format.";
        let user_msg = format!("Conversation Snippet:\n{}", snippet);
        let response = self
            .chat_with_fallback(
                &[LlmMessage::user(&user_msg)],
                &[],
                None,
                summarizer_system_prompt,
                Some(1024),
            )
            .await;

        if response.success {
            let summary = response.text.trim().to_string();
            log::info!(
                "[Compaction] Generated episodic memory summary:\n{}",
                summary
            );
            if let Ok(mut ms_guard) = self.memory_store.lock() {
                if let Some(store) = ms_guard.as_mut() {
                    let _ = store.insert_episodic_memory(session_id, &summary);
                }
            }
        }
    }
}

fn was_shopping_tool_executed(messages: &[LlmMessage], history_len: usize) -> bool {
    let start_idx = history_len.min(messages.len());
    for msg in &messages[start_idx..] {
        if msg.role == "tool" {
            let name = msg.tool_name.to_lowercase();
            if name.contains("zepto") || name.contains("swiggy") || name.contains("instamart") || name.contains("cart") || name.contains("search") {
                return true;
            }
        }
    }
    false
}

fn normalize_mcp_tool_result(tool_name: &str, result: Value) -> Value {
    let outcome = crate::channel::mcp_client::McpToolOutcome::normalize(&result);
    if outcome.is_failure() {
        return json!({
            "error": outcome
                .message
                .clone()
                .unwrap_or_else(|| format!("MCP tool returned {}", outcome.status)),
            "mcp_outcome": outcome.to_json(),
            "raw_result": result,
        });
    }

    if is_cart_mutation_tool_name(tool_name) {
        match result {
            Value::Object(mut obj) => {
                obj.insert("mcp_outcome".to_string(), outcome.to_json());
                obj.insert("verification_required".to_string(), Value::Bool(true));
                obj.insert(
                    "verification_hint".to_string(),
                    Value::String(
                        "Call a provider cart, bill, or cart-details read tool before final response."
                            .to_string(),
                    ),
                );
                Value::Object(obj)
            }
            other => json!({
                "result": other,
                "mcp_outcome": outcome.to_json(),
                "verification_required": true,
                "verification_hint": "Call a provider cart, bill, or cart-details read tool before final response."
            }),
        }
    } else {
        result
    }
}

fn is_cart_mutation_tool_name(tool_name: &str) -> bool {
    let name = tool_name.to_ascii_lowercase();
    name.contains("update_cart")
        || name.contains("add_to_cart")
        || name.contains("remove_from_cart")
        || name.contains("clear_cart")
}

fn is_cart_verification_tool_name(tool_name: &str) -> bool {
    let name = tool_name.to_ascii_lowercase();
    if is_cart_mutation_tool_name(&name) {
        return false;
    }
    name.contains("view_cart")
        || name.contains("get_cart")
        || name.contains("cart_details")
        || name.contains("bill")
}

fn shopping_cart_mutation_unverified(messages: &[LlmMessage]) -> bool {
    let mut pending_mutation = false;
    for msg in messages {
        if msg.role != "tool" {
            continue;
        }
        let name = msg.tool_name.to_ascii_lowercase();
        if is_cart_mutation_tool_name(&name) && msg.tool_result.get("error").is_none() {
            pending_mutation = true;
        } else if pending_mutation && is_cart_verification_tool_name(&name) {
            pending_mutation = false;
        }
    }
    pending_mutation
}

fn last_cart_mutation_failed(messages: &[LlmMessage]) -> Option<(String, Value, String)> {
    for msg in messages.iter().rev() {
        if msg.role == "tool" {
            let name = msg.tool_name.to_ascii_lowercase();
            if is_cart_mutation_tool_name(&name) {
                if let Some(err) = msg.tool_result.get("error") {
                    let mut args = Value::Null;
                    for prev_msg in messages.iter().rev() {
                        if prev_msg.role == "assistant" {
                            if let Some(tc) = prev_msg.tool_calls.iter().find(|tc| tc.id == msg.tool_call_id) {
                                args = tc.args.clone();
                                break;
                            }
                        }
                    }
                    return Some((msg.tool_name.clone(), args, err.as_str().unwrap_or("unknown error").to_string()));
                } else {
                    return None;
                }
            }
        }
    }
    None
}

fn last_payment_options_failed(messages: &[LlmMessage]) -> Option<(String, Value, String)> {
    for msg in messages.iter().rev() {
        if msg.role == "tool" {
            let name = msg.tool_name.to_ascii_lowercase();
            if is_payment_read_tool_name(&name) {
                if let Some(err) = msg.tool_result.get("error") {
                    let mut args = Value::Null;
                    for prev_msg in messages.iter().rev() {
                        if prev_msg.role == "assistant" {
                            if let Some(tc) = prev_msg.tool_calls.iter().find(|tc| tc.id == msg.tool_call_id) {
                                args = tc.args.clone();
                                break;
                            }
                        }
                    }
                    return Some((
                        msg.tool_name.clone(),
                        args,
                        err.as_str().unwrap_or("unknown error").to_string(),
                    ));
                }
                return None;
            }
        }
    }
    None
}

fn effective_agent_task_intent(prompt: &str, messages: &[LlmMessage]) -> AgentTaskIntent {
    let direct = classify_agent_task_intent(prompt);
    if direct != AgentTaskIntent::Unknown {
        return direct;
    }

    if is_ambiguous_retry_prompt(prompt) && recent_context_mentions_payment_options(messages) {
        return AgentTaskIntent::ShowPaymentOptions;
    }

    direct
}

fn is_ambiguous_retry_prompt(prompt: &str) -> bool {
    let lower = prompt.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "yes" | "ok" | "okay" | "again" | "try again" | "check again" | "proceed"
    ) || lower.contains("one more")
}

fn recent_context_mentions_payment_options(messages: &[LlmMessage]) -> bool {
    messages.iter().rev().take(8).any(|msg| {
        let text = msg.text.to_ascii_lowercase();
        text.contains("payment option")
            || text.contains("payment method")
            || text.contains("payment api")
            || text.contains("retrieve the payment")
            || text.contains("fetching the payment")
    })
}
