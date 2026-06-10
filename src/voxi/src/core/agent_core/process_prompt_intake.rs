impl AgentCore {
    fn parse_safety_confirmation_reply(prompt: &str) -> Option<bool> {
        let input_lower = prompt.trim().to_lowercase();
        if input_lower.is_empty() {
            return None;
        }

        if matches!(input_lower.as_str(), "no" | "n" | "cancel" | "stop" | "abort") {
            return Some(false);
        }

        let denies = ["no,", "no ", "cancel", "stop", "do not", "don't", "dont", "abort"];
        if denies.iter().any(|word| input_lower.contains(word)) {
            return Some(false);
        }

        if matches!(input_lower.as_str(), "yes" | "y" | "confirm" | "/confirm") {
            return Some(true);
        }

        let confirms = [
            "yes ", " confirm", "confirm ", "/confirm", "proceed", "go ahead",
            "approve", "run it", "continue",
        ];
        if confirms.iter().any(|word| input_lower.contains(word)) {
            return Some(true);
        }

        None
    }

    fn is_nonterminal_progress_response(text: &str) -> bool {
        let lower = text.to_lowercase();
        let promises_future_work = lower.contains("i am currently")
            || lower.contains("i'm currently")
            || lower.contains("i will ")
            || lower.contains("i’ll ")
            || lower.contains("shortly")
            || lower.contains("in progress")
            || lower.contains("working on");
        let mentions_pending_action = lower.contains("search")
            || lower.contains("present")
            || lower.contains("show")
            || lower.contains("fetch")
            || lower.contains("get ");

        promises_future_work && mentions_pending_action
    }

    fn summarize_confirmed_tool_result(tool_name: &str, result: &Value) -> String {
        if let Some(error) = result.get("error").and_then(Value::as_str) {
            return format!(
                "I could not complete `{}` after confirmation: {}",
                tool_name, error
            );
        }

        let result_text = serde_json::to_string_pretty(result)
            .unwrap_or_else(|_| result.to_string());
        let clipped = Self::truncate_chars(&result_text, 2400);
        format!(
            "Confirmed and ran `{}`.\n\n```json\n{}\n```",
            tool_name, clipped
        )
    }

    fn truncate_chars(s: &str, max: usize) -> String {
        if s.len() > max {
            let mut truncated = s.chars().take(max).collect::<String>();
            truncated.push_str("...");
            truncated
        } else {
            s.to_string()
        }
    }

    fn parse_numbered_options(text: &str) -> Vec<(usize, String)> {
        static OPTION_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            match regex::Regex::new(r"(?m)^\s*(\d+)[\.\)]\s*(.*)$") {
                Ok(re) => re,
                Err(err) => {
                    log::error!("Failed to compile option regex: {}", err);
                    match regex::Regex::new("") {
                        Ok(fallback) => fallback,
                        Err(_) => unreachable!("Empty regex is guaranteed to compile"),
                    }
                }
            }
        });

        let mut options = Vec::new();
        for line in text.lines() {
            if let Some(caps) = OPTION_RE.captures(line) {
                if let (Some(num_match), Some(text_match)) = (caps.get(1), caps.get(2)) {
                    if let Ok(num) = num_match.as_str().parse::<usize>() {
                        let cleaned_text = text_match.as_str().trim().trim_matches('`').trim().to_string();
                        if !cleaned_text.is_empty() {
                            options.push((num, cleaned_text));
                        }
                    }
                }
            }
        }
        options
    }

    async fn resolve_confirmed_option(&self, user_input: &str, options: &[(usize, String)]) -> Option<String> {
        let input_lower = user_input.trim().to_lowercase();
        if input_lower.is_empty() || options.is_empty() {
            return None;
        }

        // Get all available tool names dynamically
        let mut available_tools = Vec::new();
        {
            let td = self.tool_dispatcher.read().await;
            for tool in td.get_tool_declarations() {
                available_tools.push(tool.name.clone());
            }
        }
        {
            let mcp = self.mcp_client_manager.read().await;
            for tool in mcp.get_all_tools() {
                available_tools.push(tool.name.clone());
            }
        }
        // Add meta tools that are always loaded
        available_tools.push("request_user_clarification".to_string());
        available_tools.push("send_outbound_message".to_string());
        available_tools.push("reload_mcp_servers".to_string());

        // Helper to extract a valid tool name from a descriptive option string
        let extract_tool_name = |opt_text: &str| -> Option<String> {
            let opt_lower = opt_text.to_lowercase();
            // Try to find an exact match first
            for tool in &available_tools {
                if tool.to_lowercase() == opt_lower {
                    return Some(tool.clone());
                }
            }
            // Check if any tool name is a substring of the option text
            for tool in &available_tools {
                if opt_lower.contains(&tool.to_lowercase()) {
                    return Some(tool.clone());
                }
            }
            None
        };

        // 1. Check if the user input is a direct index number (e.g. "1", "1st", "option 1", "choice 1")
        let mut chosen_index = None;
        if let Some(digit_char) = input_lower.chars().find(|c| c.is_ascii_digit()) {
            if let Some(digit) = digit_char.to_digit(10) {
                chosen_index = Some(digit as usize);
            }
        } else if input_lower.contains("first") || input_lower.contains("1st") {
            chosen_index = Some(1);
        } else if input_lower.contains("second") || input_lower.contains("2nd") {
            chosen_index = Some(2);
        } else if input_lower.contains("third") || input_lower.contains("3rd") {
            chosen_index = Some(3);
        }

        if let Some(idx) = chosen_index {
            for &(num, ref text) in options {
                if num == idx {
                    if let Some(tool_name) = extract_tool_name(text) {
                        return Some(tool_name);
                    }
                    return Some(text.clone());
                }
            }
        }

        // 2. Check if user input is a positive confirmation
        let is_confirmation = input_lower == "yes"
            || input_lower == "y"
            || input_lower == "confirm"
            || input_lower == "proceed"
            || input_lower.contains("go ahead")
            || input_lower.contains("yes run")
            || input_lower.contains("run it")
            || input_lower.contains("run");

        if is_confirmation {
            if let Some(&(_, ref text)) = options.first() {
                if let Some(tool_name) = extract_tool_name(text) {
                    return Some(tool_name);
                }
                return Some(text.clone());
            }
        }

        // 3. Check if user input contains the name of any option
        for &(_, ref text) in options {
            if input_lower.contains(&text.to_lowercase()) {
                if let Some(tool_name) = extract_tool_name(text) {
                    return Some(tool_name);
                }
                return Some(text.clone());
            }
        }

        None
    }

    #[allow(dead_code)]
    fn is_zepto_address_selected(messages: &[LlmMessage]) -> bool {
        let mut selected = false;
        for msg in messages {
            for tc in &msg.tool_calls {
                if tc.name == "mcp_zepto_select_saved_address" || tc.name == "select_saved_address" {
                    let tc_id = &tc.id;
                    for res_msg in messages {
                        if res_msg.role == "tool" && &res_msg.tool_call_id == tc_id {
                            let res_str = res_msg.tool_result.to_string();
                            if !res_str.contains("error") && !res_str.contains("failed") {
                                selected = true;
                            }
                        }
                    }
                }
            }
        }
        selected
    }

    fn is_shopping_intent(&self, session_id: &str, prompt: &str) -> bool {
        if let Ok(ss) = self.session_store.lock() {
            if let Some(store) = ss.as_ref() {
                let session_workdir = store.session_workdir(session_id);
                let state_path = shopping_state_path(&session_workdir, session_id);
                if state_path.exists() {
                    return true;
                }
            }
        }

        let prompt_lower = prompt.to_lowercase();
        let mut found = prompt_lower.contains("zepto")
            || prompt_lower.contains("swiggy")
            || prompt_lower.contains("instamart")
            || prompt_lower.contains("cart")
            || prompt_lower.contains("checkout")
            || prompt_lower.contains("order")
            || prompt_lower.contains("groceries")
            || prompt_lower.contains("grocery")
            || prompt_lower.contains("food")
            || prompt_lower.contains("buy")
            || prompt_lower.contains("purchase")
            || prompt_lower.contains("shop")
            || prompt_lower.contains("add ")
            || prompt_lower.starts_with("add")
            || prompt_lower.contains("choose")
            || prompt_lower.contains("select");

        if !found {
            if let Ok(ms_guard) = self.memory_store.lock() {
                if let Some(ms) = ms_guard.as_ref() {
                    if let Some(prompt_emb) = ms.encode_text_embedding(prompt) {
                        if let Some(ref_emb) = ms.encode_text_embedding("shopping, grocery search, adding items to cart, checkout, store selection, address selection") {
                            let similarity: f32 = prompt_emb.iter().zip(ref_emb.iter()).map(|(a, b)| a * b).sum();
                            if similarity > 0.30 {
                                found = true;
                            }
                        }
                    }
                }
            }
        }
        found
    }

    fn get_session_lock(&self, session_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.session_locks.lock().unwrap_or_else(|err| err.into_inner());
        locks.entry(session_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    pub fn is_request_cancelled(&self, request_id: &str) -> bool {
        if let Ok(active) = self.active_requests.lock() {
            if let Some(req) = active.get(request_id) {
                return req.cancelled.load(std::sync::atomic::Ordering::SeqCst);
            }
        }
        false
    }

    fn handle_cancellation(&self, session_id: &str, request_id: &str) -> String {
        log::info!("Request {} for session {} cancelled at checkpoint", request_id, session_id);
        let msg = "Request stopped by user.";
        if let Ok(ss) = self.session_store.lock() {
            if let Some(store) = ss.as_ref() {
                store.add_message(session_id, "assistant", msg);
                store.add_structured_assistant_text_message(session_id, msg);
            }
        }
        if let Ok(mut active) = self.active_requests.lock() {
            active.remove(request_id);
        }
        msg.to_string()
    }

    pub fn cancel_request(&self, session_id: &str, request_id: &str) -> Result<(), String> {
        let active = self.active_requests.lock().map_err(|e| e.to_string())?;
        if let Some(req) = active.get(request_id) {
            if req.session_id == session_id || session_id == "new" {
                req.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
                log::info!("Cancelled request {} for session {}", request_id, session_id);
                return Ok(());
            } else {
                return Err("Session ID mismatch".to_string());
            }
        }
        Err("Request not found".to_string())
    }

    pub fn get_active_requests(&self) -> Vec<RequestState> {
        if let Ok(active) = self.active_requests.lock() {
            active.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub async fn process_prompt(
        &self,
        session_id: &str,
        prompt: &str,
        on_chunk: Option<&(dyn Fn(&str) + Send + Sync)>,
    ) -> String {
        self.process_prompt_with_request(session_id, prompt, None, on_chunk).await
    }

    pub async fn process_prompt_with_request(
        &self,
        session_id: &str,
        prompt: &str,
        request_id: Option<String>,
        on_chunk: Option<&(dyn Fn(&str) + Send + Sync)>,
    ) -> String {
        let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let req_state = RequestState {
            session_id: session_id.to_string(),
            request_id: request_id.clone(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        {
            let mut active = self.active_requests.lock().unwrap_or_else(|err| err.into_inner());
            if active.contains_key(&request_id) {
                return format!("Error: Duplicate active request ID {}", request_id);
            }
            active.insert(request_id.clone(), req_state.clone());
        }

        if req_state.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return self.handle_cancellation(session_id, &request_id);
        }

        let lock = self.get_session_lock(session_id);
        let _guard = lock.lock().await;

        if req_state.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return self.handle_cancellation(session_id, &request_id);
        }

        // Surface a structured warning when the incoming prompt looks like a
        // prompt-injection attempt. Detection-only (does not block legitimate
        // prompts): the tool-call SafetyGuard remains the enforcement boundary.
        if let Ok(guard) = self.safety_guard.lock() {
            if guard.check_prompt_injection(prompt) {
                log::warn!(
                    "[Safety] Possible prompt-injection phrasing in session '{}' request '{}'",
                    session_id,
                    request_id
                );
            }
        }

        let result = self
            .process_prompt_internal(session_id, prompt, &request_id, &req_state, on_chunk)
            .await;

        if let Ok(mut active) = self.active_requests.lock() {
            active.remove(&request_id);
        }
        result
    }
}
