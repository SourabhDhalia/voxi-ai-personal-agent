use super::*;

pub(super) struct PromptLoopPreparation {
    pub messages: Vec<LlmMessage>,
    pub context_engine: SizedContextEngine,
}

impl AgentCore {
    pub(super) async fn prepare_prompt_loop(
        &self,
        session_id: &str,
        prompt: &str,
        loop_state: &mut AgentLoopState,
        history: &[crate::storage::session_store::SessionMessage],
        skill_context: Option<&str>,
        dynamic_context: Option<&str>,
        memory_context_for_log: Option<&str>,
        system_prompt: &str,
        tools: &[crate::llm::backend::LlmToolDecl],
        mut messages: Vec<LlmMessage>,
        literal_json_output: bool,
    ) -> PromptLoopPreparation {
        loop_state.transition(AgentPhase::Planning);
        let context_engine = SizedContextEngine::new().with_threshold(loop_state.compact_threshold);

        log_payload_breakdown(
            session_id,
            prompt,
            history,
            skill_context,
            dynamic_context,
            memory_context_for_log,
            system_prompt,
            tools,
            &messages,
            &context_engine,
        );

        let mut matched_workflow_id = None;
        {
            let workflow_engine = self.workflow_engine.read().await;
            for workflow_value in workflow_engine.list_workflows() {
                if let (Some(workflow_id), Some(trigger)) = (
                    workflow_value.get("id").and_then(|value| value.as_str()),
                    workflow_value
                        .get("trigger")
                        .and_then(|value| value.as_str()),
                ) {
                    if trigger != "manual" && (prompt.contains(trigger) || trigger == prompt) {
                        matched_workflow_id = Some(workflow_id.to_string());
                        break;
                    }
                }
            }
        }

        if let Some(workflow_id) = matched_workflow_id {
            log::info!(
                "[Planning] Matched workflow trigger '{}', entering Workflow Mode.",
                workflow_id
            );
            loop_state.active_workflow_id = Some(workflow_id);
        } else if crate::core::intent_analyzer::IntentAnalyzer::is_complex_task(prompt)
            && !literal_json_output
        {
            log::debug!(
                "[AgentLoop] Complex prompt detected. Triggering explicit Plan-and-Solve..."
            );
            let plan_sys = "You are a precise planner. Outline the distinct steps to solve the user's request. Output only a list of concise steps.";

            // Use chat_with_fallback so the planning preflight participates in
            // the full provider-selection loop.  A transient failure on the first
            // available provider no longer silently disables planning when a later
            // provider is healthy.
            let plan_response = Some(
                self.chat_with_fallback(
                    &sanitize_messages_for_transport(vec![LlmMessage::user(prompt)]),
                    &[],
                    None,
                    plan_sys,
                    Some(1024),
                )
                .await,
            );

            if let Some(plan_response) = plan_response {
                if plan_response.success {
                    let steps = plan_response.text.trim().to_string();
                    loop_state.plan_steps.push(steps.clone());
                    messages.push(LlmMessage {
                        role: "system".into(),
                        text: format!("## Active Plan (Follow these steps):\n{}", steps),
                        ..Default::default()
                    });
                    log::info!("[Planning] Extracted plan steps into context.");
                }
            }
        }

        loop_state.token_used = context_engine.estimate_tokens(&messages);
        if context_engine.should_compact(&messages, loop_state.token_budget)
            || loop_state.needs_compaction()
        {
            log::debug!(
                "[AgentLoop] Pre-loop compaction triggered ({}% used)",
                (loop_state.token_used as f32 / loop_state.token_budget as f32 * 100.0) as u32
            );
            messages = context_engine.compact(messages, loop_state.token_budget);
            loop_state.token_used = context_engine.estimate_tokens(&messages);
            self.persist_compacted_messages(session_id, &messages);
        }

        PromptLoopPreparation {
            messages,
            context_engine,
        }
    }
}
