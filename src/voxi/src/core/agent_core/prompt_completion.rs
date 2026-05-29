use super::*;

impl AgentCore {
    pub(super) fn store_assistant_text_message(&self, session_id: &str, text: &str) {
        if let Ok(ss) = self.session_store.lock() {
            if let Some(store) = ss.as_ref() {
                store.add_message(session_id, "assistant", text);
                store.add_structured_assistant_text_message(session_id, text);
            }
        }
    }

    pub(super) fn finalize_prompt_text(
        &self,
        session_id: &str,
        loop_state: &mut AgentLoopState,
        text: String,
    ) -> String {
        self.store_assistant_text_message(session_id, &text);
        loop_state.transition(AgentPhase::ResultReporting);
        loop_state.transition(AgentPhase::Complete);
        loop_state.log_self_inspection();
        self.persist_loop_snapshot(loop_state);
        log_conversation("Assistant", &text);
        text
    }

    pub(super) async fn finalize_prompt_text_with_memory(
        &self,
        session_id: &str,
        messages: &[LlmMessage],
        text: String,
        skip_memory_extraction: bool,
        loop_state: &mut AgentLoopState,
    ) -> String {
        self.store_assistant_text_message(session_id, &text);
        if !skip_memory_extraction {
            self.extract_and_save_memory(messages, &text).await;
        }
        loop_state.transition(AgentPhase::ResultReporting);
        loop_state.transition(AgentPhase::Complete);
        loop_state.log_self_inspection();
        self.persist_loop_snapshot(loop_state);
        log_conversation("Assistant", &text);
        text
    }

    pub(super) fn finalize_prompt_file_targets(
        &self,
        session_id: &str,
        prompt: &str,
        session_workdir: &Path,
        completed_targets: &[String],
        notice: Option<&str>,
        loop_state: &mut AgentLoopState,
    ) -> String {
        if let Ok(ss) = self.session_store.lock() {
            if let Some(store) = ss.as_ref() {
                maybe_record_completed_file_preview_interactions(
                    store,
                    session_id,
                    prompt,
                    session_workdir,
                    completed_targets,
                );
            }
        }
        let text = prepend_completion_notice(
            completion_message_for_prompt_file_targets(prompt, session_workdir, completed_targets),
            notice,
        );
        self.finalize_prompt_text(session_id, loop_state, text)
    }

    pub(super) async fn finalize_prompt_file_targets_with_memory(
        &self,
        session_id: &str,
        prompt: &str,
        session_workdir: &Path,
        completed_targets: &[String],
        notice: Option<&str>,
        messages: &[LlmMessage],
        skip_memory_extraction: bool,
        loop_state: &mut AgentLoopState,
    ) -> String {
        if let Ok(ss) = self.session_store.lock() {
            if let Some(store) = ss.as_ref() {
                maybe_record_completed_file_preview_interactions(
                    store,
                    session_id,
                    prompt,
                    session_workdir,
                    completed_targets,
                );
            }
        }
        let text = prepend_completion_notice(
            completion_message_for_prompt_file_targets(prompt, session_workdir, completed_targets),
            notice,
        );
        self.finalize_prompt_text_with_memory(
            session_id,
            messages,
            text,
            skip_memory_extraction,
            loop_state,
        )
        .await
    }
}
