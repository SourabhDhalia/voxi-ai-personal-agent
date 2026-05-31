//! MCP client — Model Context Protocol client for external tool servers.
//!
//! Connects to an MCP server via stdio transport:
//! - Spawns child process → pipes stdin/stdout for JSON-RPC 2.0
//! - Performs `initialize` handshake
//! - Discovers remote tools via `tools/list`
//! - Calls remote tools via `tools/call`

use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::llm::backend::LlmToolDecl;

const MCP_PROTOCOL_VERSION: &str = "2025-11-25";
const LEGACY_HTTP_SSE_PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpConnectionState {
    Disconnected,
    Connected,
    AuthRequired,
    Failed,
}

impl McpConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpConnectionState::Disconnected => "disconnected",
            McpConnectionState::Connected => "connected",
            McpConnectionState::AuthRequired => "auth_required",
            McpConnectionState::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug)]
pub struct McpServerStatus {
    pub name: String,
    pub transport: String,
    pub state: McpConnectionState,
    pub connected: bool,
    pub auth_required: bool,
    pub has_access_token: bool,
    pub tool_count: usize,
    pub endpoint: Option<String>,
    pub negotiated_protocol_version: Option<String>,
    pub message: Option<String>,
    pub suggested_command: Option<String>,
}

#[derive(Clone, Debug)]
pub struct McpToolInfo {
    pub server_name: String,
    pub original_name: String,
    pub safe_name: String,
    pub legacy_name: String,
    pub description: String,
    pub parameters: Value,
    searchable_text: String,
}

impl McpToolInfo {
    pub fn declaration(&self) -> LlmToolDecl {
        let description = if self.description.trim().is_empty() {
            format!(
                "MCP provider '{}' tool '{}'. Inspect this tool's schema before use.",
                self.server_name, self.original_name
            )
        } else {
            format!(
                "MCP provider '{}' tool '{}'. {}",
                self.server_name,
                self.original_name,
                self.description.trim()
            )
        };

        LlmToolDecl {
            name: self.safe_name.clone(),
            description,
            parameters: self.parameters.clone(),
        }
    }

    pub fn to_search_json(&self, score: usize) -> Value {
        json!({
            "name": self.safe_name.clone(),
            "legacy_name": self.legacy_name.clone(),
            "provider": self.server_name.clone(),
            "remote_tool": self.original_name.clone(),
            "description": self.description.clone(),
            "parameters": self.parameters.clone(),
            "score": score,
        })
    }
}

#[derive(Clone, Debug)]
pub struct McpToolSearchResult {
    pub score: usize,
    pub tool: McpToolInfo,
    pub behavior: McpToolBehavior,
}

#[derive(Clone, Debug)]
pub enum McpToolResolveError {
    NotFound,
    Ambiguous(Vec<McpToolInfo>),
}

impl McpToolSearchResult {
    pub fn to_json(&self) -> Value {
        let mut value = self.tool.to_search_json(self.score);
        if let Some(obj) = value.as_object_mut() {
            obj.insert("behavior".to_string(), self.behavior.to_json());
        }
        value
    }
}

#[derive(Clone, Debug)]
pub struct McpToolBehavior {
    pub provider: String,
    pub safe_name: String,
    pub original_name: String,
    pub description: String,
    pub required_inputs: Vec<String>,
    pub capability_tags: Vec<String>,
    pub risk_level: String,
    pub identifiers: Vec<String>,
    pub prerequisites: Vec<String>,
    pub verification_tools: Vec<String>,
    pub known_error_patterns: Vec<String>,
    pub behavior_doc: String,
}

impl McpToolBehavior {
    pub fn from_tool(tool: &McpToolInfo, provider_tools: &[McpToolInfo]) -> Self {
        let searchable = format!(
            "{} {} {} {}",
            tool.server_name,
            tool.original_name,
            tool.description,
            tool.parameters
        )
        .to_ascii_lowercase();
        let required_inputs = required_inputs_from_schema(&tool.parameters);
        let capability_tags = infer_capability_tags(&searchable);
        let risk_level = infer_risk_level(&searchable);
        let identifiers = infer_identifier_fields(&searchable, &tool.parameters);
        let prerequisites = infer_prerequisites(&capability_tags, &searchable);
        let verification_tools = infer_verification_tools(tool, provider_tools);
        let known_error_patterns = infer_known_error_patterns(&capability_tags, &searchable);

        let behavior_doc = format!(
            "provider: {}\nsafe_tool: {}\nremote_tool: {}\nrisk: {}\ncapabilities: {}\nrequired_inputs: {}\nidentifiers: {}\nprerequisites: {}\nverification_tools: {}\nknown_errors: {}\ndescription: {}\nschema: {}",
            tool.server_name,
            tool.safe_name,
            tool.original_name,
            risk_level,
            capability_tags.join(", "),
            required_inputs.join(", "),
            identifiers.join(", "),
            prerequisites.join("; "),
            verification_tools.join(", "),
            known_error_patterns.join("; "),
            tool.description,
            tool.parameters
        );

        Self {
            provider: tool.server_name.clone(),
            safe_name: tool.safe_name.clone(),
            original_name: tool.original_name.clone(),
            description: tool.description.clone(),
            required_inputs,
            capability_tags,
            risk_level,
            identifiers,
            prerequisites,
            verification_tools,
            known_error_patterns,
            behavior_doc,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "provider": self.provider.clone(),
            "safe_name": self.safe_name.clone(),
            "remote_tool": self.original_name.clone(),
            "capability_tags": self.capability_tags.clone(),
            "risk_level": self.risk_level.clone(),
            "required_inputs": self.required_inputs.clone(),
            "identifiers": self.identifiers.clone(),
            "prerequisites": self.prerequisites.clone(),
            "verification_tools": self.verification_tools.clone(),
            "known_error_patterns": self.known_error_patterns.clone(),
            "summary": self.behavior_doc.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct McpToolOutcome {
    pub status: String,
    pub message: Option<String>,
}

impl McpToolOutcome {
    pub fn normalize(result: &Value) -> Self {
        if result
            .get("isError")
            .or_else(|| result.get("is_error"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            return Self::from_message("fatal", first_error_message(result));
        }

        if let Some(error) = result.get("error") {
            return Self::from_message("fatal", Some(value_to_short_text(error)));
        }

        let combined = collect_string_values(result).join(" ").to_ascii_lowercase();
        if combined.contains("unauthorized")
            || combined.contains("forbidden")
            || combined.contains("oauth")
            || combined.contains("token expired")
            || combined.contains("login")
            || combined.contains("authentication")
        {
            return Self::from_message("auth_required", Some(extract_relevant_message(result)));
        }

        if combined.contains("no valid items in cart")
            || combined.contains("address id could not be found")
            || combined.contains("address id not found")
            || combined.contains("delivery address id could not be found")
            || combined.contains("invalid address")
            || combined.contains("invalid item")
            || combined.contains("out of stock")
            || combined.contains("not serviceable")
        {
            return Self::from_message("business_error", Some(extract_relevant_message(result)));
        }

        if combined.contains("please select")
            || combined.contains("please choose")
            || combined.contains("please specify")
            || combined.contains("requires confirmation")
            || combined.contains("otp")
        {
            return Self::from_message("user_action_required", Some(extract_relevant_message(result)));
        }

        if combined.contains("ambiguous") || combined.contains("multiple matches") {
            return Self::from_message("ambiguous", Some(extract_relevant_message(result)));
        }

        Self {
            status: "success".to_string(),
            message: None,
        }
    }

    fn from_message(status: &str, message: Option<String>) -> Self {
        Self {
            status: status.to_string(),
            message,
        }
    }

    pub fn is_failure(&self) -> bool {
        self.status != "success"
    }

    pub fn to_json(&self) -> Value {
        json!({
            "status": self.status.clone(),
            "message": self.message.clone(),
        })
    }
}

fn required_inputs_from_schema(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn infer_capability_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for (needle, tag) in [
        ("address", "address"),
        ("location", "address"),
        ("search", "search"),
        ("product", "product"),
        ("menu", "menu"),
        ("restaurant", "food"),
        ("food", "food"),
        ("grocery", "grocery"),
        ("instamart", "grocery"),
        ("cart", "cart"),
        ("checkout", "checkout"),
        ("payment", "payment"),
        ("order", "order"),
        ("book", "booking"),
        ("reserve", "booking"),
        ("table", "booking"),
        ("bill", "verification"),
        ("view", "read"),
        ("list", "read"),
        ("get", "read"),
    ] {
        if text.contains(needle) && !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_string());
        }
    }
    if tags.is_empty() {
        tags.push("generic".to_string());
    }
    tags
}

fn infer_risk_level(text: &str) -> String {
    if text.contains("checkout")
        || text.contains("payment")
        || text.contains("pay")
        || text.contains("place_order")
        || text.contains("book")
        || text.contains("reserve")
    {
        "irreversible_confirmation_required".to_string()
    } else if text.contains("update_cart")
        || text.contains("add_to_cart")
        || text.contains("remove_from_cart")
        || text.contains("clear_cart")
        || text.contains("cart")
    {
        "reversible_cart_mutation".to_string()
    } else {
        "read_only".to_string()
    }
}

fn infer_identifier_fields(text: &str, schema: &Value) -> Vec<String> {
    let mut fields = Vec::new();
    for key in [
        "spinId",
        "spin_id",
        "variantId",
        "variant_id",
        "skuId",
        "sku_id",
        "productId",
        "product_id",
        "itemId",
        "item_id",
        "storeId",
        "store_id",
        "addressId",
        "address_id",
        "selectedAddressId",
        "restaurantId",
        "restaurant_id",
    ] {
        if text.contains(&key.to_ascii_lowercase()) || schema.to_string().contains(key) {
            fields.push(key.to_string());
        }
    }
    fields.sort();
    fields.dedup();
    fields
}

fn infer_prerequisites(tags: &[String], text: &str) -> Vec<String> {
    let mut prereqs = Vec::new();
    let needs_address = tags.iter().any(|tag| tag == "search" || tag == "cart")
        && !tags.iter().any(|tag| tag == "address");
    if needs_address {
        prereqs.push("Resolve a live provider address/location before search or cart writes.".to_string());
    }
    if text.contains("update_cart") || text.contains("add_to_cart") || text.contains("cart") {
        prereqs.push("Use a provider-returned product/cart identifier; never invent IDs from display text.".to_string());
    }
    prereqs
}

fn infer_verification_tools(tool: &McpToolInfo, provider_tools: &[McpToolInfo]) -> Vec<String> {
    let name = tool.original_name.to_ascii_lowercase();
    if !(name.contains("update_cart")
        || name.contains("add_to_cart")
        || name.contains("remove_from_cart")
        || name.contains("clear_cart"))
    {
        return vec![];
    }

    provider_tools
        .iter()
        .filter(|candidate| candidate.server_name == tool.server_name)
        .filter(|candidate| {
            let candidate_name = candidate.original_name.to_ascii_lowercase();
            (candidate_name.contains("view_cart")
                || candidate_name.contains("get_cart")
                || candidate_name.contains("cart_details")
                || candidate_name.contains("bill"))
                && !candidate_name.contains("update_cart")
                && !candidate_name.contains("add_to_cart")
                && !candidate_name.contains("remove_from_cart")
                && !candidate_name.contains("clear_cart")
        })
        .map(|candidate| candidate.safe_name.clone())
        .collect()
}

fn infer_known_error_patterns(tags: &[String], text: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    if tags.iter().any(|tag| tag == "cart") || text.contains("cart") {
        patterns.push("No valid items in cart".to_string());
        patterns.push("Invalid or stale product identifier".to_string());
    }
    if tags.iter().any(|tag| tag == "address") || text.contains("address") {
        patterns.push("Address ID not found".to_string());
        patterns.push("Location not serviceable".to_string());
    }
    patterns.push("Auth token expired or login required".to_string());
    patterns
}

fn collect_string_values(value: &Value) -> Vec<String> {
    let mut strings = Vec::new();
    collect_string_values_into(value, &mut strings);
    strings
}

fn collect_string_values_into(value: &Value, strings: &mut Vec<String>) {
    match value {
        Value::String(text) => strings.push(text.clone()),
        Value::Array(items) => {
            for item in items {
                collect_string_values_into(item, strings);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_string_values_into(value, strings);
            }
        }
        _ => {}
    }
}

fn first_error_message(value: &Value) -> Option<String> {
    value
        .get("error")
        .map(value_to_short_text)
        .or_else(|| Some(extract_relevant_message(value)))
}

fn value_to_short_text(value: &Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string())
        .chars()
        .take(500)
        .collect()
}

fn extract_relevant_message(value: &Value) -> String {
    collect_string_values(value)
        .into_iter()
        .find(|text| {
            let lower = text.to_ascii_lowercase();
            lower.contains("error")
                || lower.contains("failed")
                || lower.contains("invalid")
                || lower.contains("not found")
                || lower.contains("no valid")
                || lower.contains("please")
                || lower.contains("auth")
        })
        .unwrap_or_else(|| value.to_string().chars().take(500).collect())
}

fn sanitize_tool_fragment(input: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;

    for ch in input.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_was_underscore = false;
            Some(ch.to_ascii_lowercase())
        } else if !last_was_underscore {
            last_was_underscore = true;
            Some('_')
        } else {
            None
        };

        if let Some(ch) = next {
            out.push(ch);
        }
    }

    let trimmed = out.trim_matches('_');
    let mut sanitized = if trimmed.is_empty() {
        "tool".to_string()
    } else {
        trimmed.to_string()
    };

    if sanitized
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        sanitized.insert_str(0, "x_");
    }

    sanitized
}

fn safe_mcp_tool_name(server_name: &str, tool_name: &str) -> String {
    format!(
        "mcp_{}_{}",
        sanitize_tool_fragment(server_name),
        sanitize_tool_fragment(tool_name)
    )
}

fn legacy_mcp_tool_name(server_name: &str, tool_name: &str) -> String {
    format!("mcp_{}_{}", server_name, tool_name)
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|token| {
            let token = token.trim().to_ascii_lowercase();
            (token.len() >= 2).then_some(token)
        })
        .collect()
}

fn expanded_query_tokens(query: &str) -> Vec<String> {
    let mut tokens = tokenize(query);
    let mut extras = Vec::new();

    for token in &tokens {
        match token.as_str() {
            "buy" | "bought" | "purchase" | "shop" | "shopping" | "get" | "need" => {
                extras.extend([
                    "shopping",
                    "grocery",
                    "groceries",
                    "product",
                    "cart",
                    "order",
                    "checkout",
                ]);
            }
            "order" | "reorder" => {
                extras.extend([
                    "order",
                    "cart",
                    "checkout",
                    "food",
                    "restaurant",
                    "grocery",
                    "product",
                ]);
            }
            "table" | "reserve" | "reservation" | "book" | "booking" => {
                extras.extend(["booking", "reservation", "restaurant", "table", "dine"]);
            }
            "eat" | "meal" | "lunch" | "dinner" | "breakfast" | "snack" => {
                extras.extend(["food", "restaurant", "menu", "order"]);
            }
            _ => {}
        }
    }

    tokens.extend(extras.into_iter().map(String::from));
    tokens.sort();
    tokens.dedup();
    tokens
}

fn edit_distance(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut prev = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut curr = vec![0; right_chars.len() + 1];

    for (i, left_ch) in left_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, right_ch) in right_chars.iter().enumerate() {
            let cost = usize::from(left_ch != right_ch);
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[right_chars.len()]
}

fn fuzzy_score(query: &str, target: &str) -> usize {
    let query_tokens = expanded_query_tokens(query);
    if query_tokens.is_empty() {
        return 0;
    }

    let target_lower = target.to_ascii_lowercase();
    let target_tokens = tokenize(target);
    let mut score = 0usize;

    for query_token in query_tokens {
        if target_lower.contains(&query_token) {
            score += 20 + query_token.len();
            continue;
        }

        let typo_match = target_tokens.iter().any(|target_token| {
            let max_distance = (query_token.len().max(target_token.len()) / 3).clamp(1, 2);
            edit_distance(&query_token, target_token) <= max_distance
        });
        if typo_match {
            score += 8 + query_token.len();
        }
    }

    score
}

fn build_searchable_text(
    server_name: &str,
    original_name: &str,
    safe_name: &str,
    description: &str,
    parameters: &Value,
) -> String {
    format!(
        "{} {} {} {} {}",
        server_name, original_name, safe_name, description, parameters
    )
}

/// A single MCP client connected to a server process or an HTTP server.
pub struct McpClient {
    pub server_name: String,
    command: String,
    args: Vec<String>,
    timeout_ms: u64,
    child: Option<Child>,
    reader: Option<Mutex<BufReader<std::process::ChildStdout>>>,
    writer: Option<Mutex<std::process::ChildStdin>>,
    connected: bool,
    tools: Vec<LlmToolDecl>,
    tool_infos: Vec<McpToolInfo>,
    next_req_id: AtomicI32,
    last_used_ms: u64,
    envs: Option<std::collections::HashMap<String, String>>,

    // HTTP transport state.
    is_http: bool,
    http_url: String,
    http_client: Option<reqwest::blocking::Client>,
    http_headers: std::collections::HashMap<String, String>,
    mcp_session_id: Arc<RwLock<Option<String>>>,
    endpoint_url: Arc<RwLock<Option<String>>>,
    received_responses: Arc<Mutex<std::collections::HashMap<i32, Value>>>,
    stop_signal: Arc<AtomicBool>,
    sse_thread: Option<std::thread::JoinHandle<()>>,
    connection_state: RwLock<McpConnectionState>,
    status_message: RwLock<Option<String>>,
    auth_challenge: RwLock<Option<String>>,
    negotiated_protocol_version: RwLock<String>,
}

fn run_sse_listener(
    client: reqwest::blocking::Client,
    url: String,
    session_id: Option<String>,
    endpoint_url: Arc<RwLock<Option<String>>>,
    received_responses: Arc<Mutex<std::collections::HashMap<i32, Value>>>,
    stop_signal: Arc<AtomicBool>,
) {
    while !stop_signal.load(Ordering::SeqCst) {
        let target_url = url.clone();

        let mut req = client
            .get(&target_url)
            .header("Accept", "text/event-stream");

        if let Some(ref sid) = session_id {
            req = req.header("Mcp-Session-Id", sid);
        }

        let mut resp = match req.send() {
            Ok(r) => r,
            Err(e) => {
                log::error!("SSE connection error to '{}': {}", target_url, e);
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        if !resp.status().is_success() {
            log::warn!(
                "Legacy HTTP+SSE listener for '{}' ended with status: {}",
                url,
                resp.status()
            );
            let status = resp.status();
            if status == reqwest::StatusCode::METHOD_NOT_ALLOWED
                || status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::NOT_FOUND
            {
                log::warn!(
                    "SSE transport not supported or unauthorized (status: {}) for '{}'. Terminating SSE listener thread.",
                    status,
                    url
                );
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }

        let mut reader = BufReader::new(resp);
        let mut line = String::new();
        let mut current_event = String::new();

        loop {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // Connection closed
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        current_event.clear();
                        continue;
                    }

                    if trimmed.starts_with("event:") {
                        current_event = trimmed["event:".len()..].trim().to_string();
                    } else if trimmed.starts_with("data:") {
                        let data = trimmed["data:".len()..].trim().to_string();

                        if current_event == "endpoint" {
                            let mut ep = endpoint_url.write().unwrap();
                            let resolved =
                                if data.starts_with("http://") || data.starts_with("https://") {
                                    data
                                } else {
                                    let base = url.trim_end_matches('/');
                                    if data.starts_with('/') {
                                        format!("{}{}", base, data)
                                    } else {
                                        format!("{}/{}", base, data)
                                    }
                                };
                            *ep = Some(resolved);
                        } else if current_event == "message" || current_event.is_empty() {
                            if let Ok(v) = serde_json::from_str::<Value>(&data) {
                                if let Some(id) = v.get("id").and_then(|id_val| id_val.as_i64()) {
                                    if let Ok(mut map) = received_responses.lock() {
                                        map.insert(id as i32, v);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("SSE read error: {}", e);
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

impl McpClient {
    pub fn new(server_name: &str, command: &str, args: &[String], timeout_ms: u64) -> Self {
        let is_http = command.starts_with("http://") || command.starts_with("https://");
        let http_url = if is_http {
            command.to_string()
        } else {
            String::new()
        };

        McpClient {
            server_name: server_name.into(),
            command: command.into(),
            args: args.to_vec(),
            timeout_ms,
            child: None,
            reader: None,
            writer: None,
            connected: false,
            tools: Vec::new(),
            tool_infos: Vec::new(),
            next_req_id: AtomicI32::new(1),
            last_used_ms: Self::now_ms(),
            envs: None,

            is_http,
            http_url,
            http_client: None,
            http_headers: std::collections::HashMap::new(),
            mcp_session_id: Arc::new(RwLock::new(None)),
            endpoint_url: Arc::new(RwLock::new(None)),
            received_responses: Arc::new(Mutex::new(std::collections::HashMap::new())),
            stop_signal: Arc::new(AtomicBool::new(false)),
            sse_thread: None,
            connection_state: RwLock::new(McpConnectionState::Disconnected),
            status_message: RwLock::new(None),
            auth_challenge: RwLock::new(None),
            negotiated_protocol_version: RwLock::new(MCP_PROTOCOL_VERSION.to_string()),
        }
    }

    pub fn with_env(mut self, envs: Option<std::collections::HashMap<String, String>>) -> Self {
        self.envs = envs;
        self
    }

    pub fn with_http_headers(
        mut self,
        headers: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        self.http_headers = headers.unwrap_or_default();
        self
    }

    fn set_connection_state(&self, state: McpConnectionState, message: Option<String>) {
        if let Ok(mut guard) = self.connection_state.write() {
            *guard = state;
        }
        if let Ok(mut guard) = self.status_message.write() {
            *guard = message;
        }
    }

    fn set_auth_challenge(&self, challenge: Option<String>) {
        if let Ok(mut guard) = self.auth_challenge.write() {
            *guard = challenge.filter(|value| !value.trim().is_empty());
        }
    }

    fn set_negotiated_protocol_version(&self, version: &str) {
        let version = version.trim();
        if version.is_empty() {
            return;
        }
        if let Ok(mut guard) = self.negotiated_protocol_version.write() {
            *guard = version.to_string();
        }
    }

    fn negotiated_protocol_version(&self) -> String {
        self.negotiated_protocol_version
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| MCP_PROTOCOL_VERSION.to_string())
    }

    fn reset_http_transport(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Some(handle) = self.sse_thread.take() {
            let _ = handle.join();
        }
        self.http_client = None;
        if let Ok(mut ep) = self.endpoint_url.write() {
            *ep = None;
        }
        if let Ok(mut session) = self.mcp_session_id.write() {
            *session = None;
        }
        if let Ok(mut map) = self.received_responses.lock() {
            map.clear();
        }
    }

    fn status_message(&self) -> Option<String> {
        self.status_message
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn auth_challenge(&self) -> Option<String> {
        self.auth_challenge
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn update_last_used(&mut self) {
        self.last_used_ms = Self::now_ms();
    }

    pub fn last_used_ms(&self) -> u64 {
        self.last_used_ms
    }

    fn normalized_server_env_key(&self) -> String {
        self.server_name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn read_secret_token(path: &std::path::Path) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                for key in ["access_token", "token", "bearer_token"] {
                    if let Some(token) = value.get(key).and_then(|v| v.as_str()) {
                        let token = token.trim();
                        if !token.is_empty() {
                            return Some(token.to_string());
                        }
                    }
                }
            }
        }

        Some(trimmed.to_string())
    }

    fn find_arg_value(&self, flags: &[&str]) -> Option<String> {
        let mut i = 0;
        while i < self.args.len() {
            if let Some(arg) = self.args.get(i) {
                if flags.contains(&arg.as_str()) && i + 1 < self.args.len() {
                    return self.args.get(i + 1).cloned();
                }
                for flag in flags {
                    let prefix = format!("{}=", flag);
                    if arg.starts_with(&prefix) {
                        return Some(arg[prefix.len()..].to_string());
                    }
                }
            }
            i += 1;
        }
        None
    }

    fn find_mcp_session_id(&self) -> Option<String> {
        let configured = self.find_arg_value(&["--mcp-session-id", "mcp_session_id"]);
        if configured.is_some() {
            return configured;
        }

        if let Some(ref envs) = self.envs {
            let server_key = self.normalized_server_env_key();
            let keys = [
                "MCP_SESSION_ID".to_string(),
                "SESSION_ID".to_string(),
                format!("{}_MCP_SESSION_ID", server_key),
                format!("{}_SESSION", server_key),
            ];
            for key in keys {
                if let Some(val) = envs.get(&key) {
                    let val = val.trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }

        None
    }

    fn find_access_token(&self) -> Option<String> {
        let token_flags = [
            "--access-token",
            "--bearer-token",
            "--token",
            "access_token",
            "oauth_token",
            "bearer_token",
            "token",
        ];

        if let Some(token) = self.find_arg_value(&token_flags) {
            return Some(token);
        }

        // 2. Look for raw direct token in args for backwards compatibility.
        // If there's an argument that is alphanumeric, length >= 6, doesn't start with '-',
        // and is not a known flag, a URL (starts with http), or a file path (contains / or \),
        // we treat it as a direct token.
        for arg in &self.args {
            let trimmed = arg.trim();
            if trimmed.len() >= 6
                && !trimmed.starts_with('-')
                && !trimmed.starts_with("http://")
                && !trimmed.starts_with("https://")
                && !trimmed.contains('/')
                && !trimmed.contains('\\')
                && !token_flags.contains(&trimmed)
            {
                return Some(trimmed.to_string());
            }
        }

        // 3. Look in env variables.
        if let Some(ref envs) = self.envs {
            let server_key = self.normalized_server_env_key();
            let keys = [
                "MCP_ACCESS_TOKEN".to_string(),
                "ACCESS_TOKEN".to_string(),
                "OAUTH_ACCESS_TOKEN".to_string(),
                "BEARER_TOKEN".to_string(),
                "SWIGGY_MCP_TOKEN".to_string(),
                format!("{}_ACCESS_TOKEN", server_key),
                format!("{}_TOKEN", server_key),
            ];
            for key in keys {
                if let Some(val) = envs.get(&key) {
                    let val = val.trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }

        // 4. Look in secrets file under VOXI_DATA_DIR/secrets/ or ~/.voxi/secrets/
        let data_dir = std::env::var("VOXI_DATA_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".voxi"))
                    .unwrap_or_else(|_| std::path::PathBuf::from("/root/.voxi"))
            });

        let secrets_dir = data_dir.join("secrets");
        let name_lower = self.server_name.to_ascii_lowercase();
        let name_safe = name_lower.replace('-', "_");

        let mut variants = vec![
            format!("{}_access_token", name_lower),
            format!("{}_access_token", name_safe),
            format!("{}_token", name_lower),
            format!("{}_token", name_safe),
            format!("{}_token", name_lower.replace('_', "-")),
            format!("oauth_{}.json", name_lower),
            format!("oauth_{}.json", name_safe),
        ];

        if name_lower.contains("swiggy") {
            variants.push("swiggy_access_token".to_string());
            variants.push("swiggy_token".to_string());
            variants.push("swiggy_food_token".to_string());
        }

        for var in variants {
            let file_path = secrets_dir.join(&var);
            if file_path.exists() {
                if let Some(token) = Self::read_secret_token(&file_path) {
                    log::info!(
                        "MCP Client: Loaded access token for '{}' from secrets file '{}'",
                        self.server_name,
                        var
                    );
                    return Some(token);
                }
            }
        }
        None
    }

    fn capture_mcp_session_id_from_headers(&self, resp: &reqwest::blocking::Response) {
        for name in ["mcp-session-id", "Mcp-Session-Id", "MCP-Session-Id"] {
            if let Some(value) = resp.headers().get(name).and_then(|v| v.to_str().ok()) {
                let value = value.trim();
                if !value.is_empty() {
                    if let Ok(mut session) = self.mcp_session_id.write() {
                        if session.as_deref() != Some(value) {
                            log::debug!(
                                "MCP Client: captured Mcp-Session-Id for '{}'",
                                self.server_name
                            );
                            *session = Some(value.to_string());
                        }
                    }
                    break;
                }
            }
        }
    }

    fn parse_http_rpc_body(body: &str) -> Result<Option<Value>, String> {
        let raw = body.trim();
        if raw.is_empty() {
            return Ok(None);
        }

        if raw.starts_with('{') || raw.starts_with('[') {
            return serde_json::from_str::<Value>(raw)
                .map(Some)
                .map_err(|e| e.to_string());
        }

        if raw.contains("data:") {
            let mut data_lines = Vec::new();
            for line in raw.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if !data.is_empty() && data != "[DONE]" {
                        data_lines.push(data.to_string());
                    }
                }
            }
            let merged = data_lines.join("\n");
            if !merged.trim().is_empty() {
                return serde_json::from_str::<Value>(&merged)
                    .map(Some)
                    .map_err(|e| e.to_string());
            }
        }
        Err(format!(
            "Unsupported HTTP MCP response format: {}",
            &raw[..raw.len().min(500)]
        ))
    }

    fn is_auth_error(message: &str) -> bool {
        message.contains("401 Unauthorized")
            || message.contains("403 Forbidden")
            || message.contains("HTTP 401")
            || message.contains("HTTP 403")
    }

    fn should_try_legacy_http_sse(message: &str) -> bool {
        message.contains("400 Bad Request")
            || message.contains("404 Not Found")
            || message.contains("405 Method Not Allowed")
            || message.contains("HTTP 400")
            || message.contains("HTTP 404")
            || message.contains("HTTP 405")
    }

    fn record_protocol_from_initialize(&self, resp: &Value) {
        if let Some(version) = resp
            .get("result")
            .and_then(|result| result.get("protocolVersion"))
            .and_then(|value| value.as_str())
        {
            self.set_negotiated_protocol_version(version);
        }
    }

    fn legacy_endpoint_from_event(base_url: &str, data: &str) -> String {
        let data = data.trim();
        if data.starts_with("http://") || data.starts_with("https://") {
            return data.to_string();
        }
        let base = base_url.trim_end_matches('/');
        if data.starts_with('/') {
            format!("{}{}", base, data)
        } else {
            format!("{}/{}", base, data)
        }
    }

    fn discover_legacy_sse_endpoint(
        &self,
        client: &reqwest::blocking::Client,
    ) -> Result<String, String> {
        let resp = client
            .get(&self.http_url)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .map_err(|err| err.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("legacy SSE GET failed with {}", status));
        }

        let mut reader = BufReader::new(resp);
        let mut line = String::new();
        let mut current_event = String::new();
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(self.timeout_ms.min(10_000)) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if let Some(event) = trimmed.strip_prefix("event:") {
                        current_event = event.trim().to_string();
                    } else if current_event == "endpoint" {
                        if let Some(data) = trimmed.strip_prefix("data:") {
                            let endpoint = data.trim();
                            if !endpoint.is_empty() {
                                return Ok(Self::legacy_endpoint_from_event(
                                    &self.http_url,
                                    endpoint,
                                ));
                            }
                        }
                    }
                }
                Err(err) => return Err(err.to_string()),
            }
        }

        Err("legacy SSE endpoint event was not received".to_string())
    }

    fn start_legacy_sse_listener(&mut self, client: reqwest::blocking::Client) {
        self.stop_signal.store(false, Ordering::SeqCst);
        let url = self.http_url.clone();
        let session_id = self
            .mcp_session_id
            .read()
            .ok()
            .and_then(|session| session.clone());
        let endpoint_url = Arc::clone(&self.endpoint_url);
        let received_responses = Arc::clone(&self.received_responses);
        let stop_signal = Arc::clone(&self.stop_signal);
        self.sse_thread = Some(std::thread::spawn(move || {
            run_sse_listener(
                client,
                url,
                session_id,
                endpoint_url,
                received_responses,
                stop_signal,
            )
        }));
    }

    fn remember_http_response(&self, value: Value) {
        if value.get("jsonrpc").is_some()
            && (value.get("result").is_some() || value.get("error").is_some())
        {
            if let Some(id) = value.get("id").and_then(|id_val| id_val.as_i64()) {
                if let Ok(mut map) = self.received_responses.lock() {
                    map.insert(id as i32, value);
                }
            }
        } else if let Some(items) = value.as_array() {
            for item in items {
                self.remember_http_response(item.clone());
            }
        }
    }

    fn apply_configured_http_headers(
        &self,
        mut req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        for (name, value) in &self.http_headers {
            let Ok(header_name) = reqwest::header::HeaderName::from_bytes(name.as_bytes()) else {
                log::warn!(
                    "MCP Client: ignoring invalid configured HTTP header '{}' for '{}'",
                    name,
                    self.server_name
                );
                continue;
            };
            let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) else {
                log::warn!(
                    "MCP Client: ignoring invalid configured HTTP header value for '{}' on '{}'",
                    name,
                    self.server_name
                );
                continue;
            };
            req = req.header(header_name, header_value);
        }
        req
    }

    /// Spawn the server process or start the HTTP client and perform the MCP handshake.
    pub fn connect(&mut self) -> bool {
        if self.connected {
            return true;
        }
        self.update_last_used();

        if self.is_http {
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(self.timeout_ms))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    self.set_connection_state(
                        McpConnectionState::Failed,
                        Some(format!("failed to build HTTP client: {}", e)),
                    );
                    log::error!(
                        "MCP Client: Failed to build HTTP client for '{}': {}",
                        self.server_name,
                        e
                    );
                    return false;
                }
            };

            self.http_client = Some(client);
            self.stop_signal.store(false, Ordering::SeqCst);
            self.set_negotiated_protocol_version(MCP_PROTOCOL_VERSION);
            self.set_connection_state(McpConnectionState::Disconnected, None);
            self.set_auth_challenge(None);

            if let Ok(mut ep) = self.endpoint_url.write() {
                *ep = Some(self.http_url.clone());
            }
            if let Ok(mut session) = self.mcp_session_id.write() {
                *session = self.find_mcp_session_id();
            }
            if let Ok(mut map) = self.received_responses.lock() {
                map.clear();
            }
            self.connected = true;

            log::debug!(
                "MCP Client: Streamable HTTP transport prepared for '{}' (protocol {})",
                self.server_name,
                MCP_PROTOCOL_VERSION
            );

            // Perform initialize handshake
            let init_params = json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "clientInfo": {"name": "voxi-mcp-client", "version": "1.0.0"}
            });

            match self.send_request_sync("initialize", &init_params, 10000) {
                Ok(resp) => {
                    if let Some(error) = resp.get("error") {
                        let message = format!("initialize returned JSON-RPC error: {}", error);
                        self.connected = false;
                        self.reset_http_transport();
                        self.set_connection_state(McpConnectionState::Failed, Some(message));
                        log::error!("MCP Client: Handshake failed for '{}'", self.server_name);
                        return false;
                    }
                    self.record_protocol_from_initialize(&resp);
                }
                Err(e) => {
                    if Self::is_auth_error(&e) {
                        self.connected = false;
                        self.reset_http_transport();
                        self.set_connection_state(
                            McpConnectionState::AuthRequired,
                            Some("OAuth bearer token required or expired".to_string()),
                        );
                        log::warn!(
                            "MCP Client: '{}' requires OAuth; run `/mcp login {}` or provide a bearer token.",
                            self.server_name,
                            self.server_name
                        );
                        return false;
                    }

                    if Self::should_try_legacy_http_sse(&e) {
                        log::debug!(
                            "MCP Client: '{}' did not accept Streamable HTTP initialize; trying legacy HTTP+SSE fallback",
                            self.server_name
                        );
                        if let Some(legacy_client) = self.http_client.as_ref().cloned() {
                            match self.discover_legacy_sse_endpoint(&legacy_client) {
                                Ok(endpoint) => {
                                    if let Ok(mut ep) = self.endpoint_url.write() {
                                        *ep = Some(endpoint);
                                    }
                                    self.set_negotiated_protocol_version(
                                        LEGACY_HTTP_SSE_PROTOCOL_VERSION,
                                    );
                                    self.start_legacy_sse_listener(legacy_client);
                                    let legacy_init = json!({
                                        "protocolVersion": LEGACY_HTTP_SSE_PROTOCOL_VERSION,
                                        "capabilities": {"tools": {}},
                                        "clientInfo": {"name": "voxi-mcp-client", "version": "1.0.0"}
                                    });
                                    match self.send_request_sync("initialize", &legacy_init, 10000)
                                    {
                                        Ok(resp) if resp.get("error").is_none() => {
                                            self.record_protocol_from_initialize(&resp);
                                        }
                                        Ok(resp) => {
                                            let message = format!(
                                                "legacy initialize returned JSON-RPC error: {}",
                                                resp.get("error").unwrap_or(&Value::Null)
                                            );
                                            self.connected = false;
                                            self.reset_http_transport();
                                            self.set_connection_state(
                                                McpConnectionState::Failed,
                                                Some(message),
                                            );
                                            return false;
                                        }
                                        Err(legacy_err) if Self::is_auth_error(&legacy_err) => {
                                            self.connected = false;
                                            self.reset_http_transport();
                                            self.set_connection_state(
                                                McpConnectionState::AuthRequired,
                                                Some(
                                                    "OAuth bearer token required or expired"
                                                        .to_string(),
                                                ),
                                            );
                                            return false;
                                        }
                                        Err(legacy_err) => {
                                            self.connected = false;
                                            self.reset_http_transport();
                                            self.set_connection_state(
                                                McpConnectionState::Failed,
                                                Some(format!(
                                                    "legacy HTTP+SSE initialize failed: {}",
                                                    legacy_err
                                                )),
                                            );
                                            return false;
                                        }
                                    }
                                }
                                Err(legacy_err) => {
                                    self.connected = false;
                                    self.reset_http_transport();
                                    self.set_connection_state(
                                        McpConnectionState::Failed,
                                        Some(format!(
                                            "Streamable HTTP initialize failed: {}; legacy HTTP+SSE fallback failed: {}",
                                            e, legacy_err
                                        )),
                                    );
                                    return false;
                                }
                            }
                        }
                    } else {
                        self.connected = false;
                        self.reset_http_transport();
                        self.set_connection_state(
                            McpConnectionState::Failed,
                            Some(format!("initialize failed: {}", e)),
                        );
                        log::error!("MCP Client: Init error for '{}': {}", self.server_name, e);
                        return false;
                    }
                    if !self.connected {
                        return false;
                    }
                    if self
                        .connection_state
                        .read()
                        .map(|state| *state == McpConnectionState::Failed)
                        .unwrap_or(false)
                    {
                        return false;
                    }
                    if self.http_client.is_none() {
                        self.set_connection_state(
                            McpConnectionState::Failed,
                            Some(format!("initialize failed: {}", e)),
                        );
                        return false;
                    }
                    // Legacy fallback succeeded.
                }
            }

            if !self.connected {
                return false;
            }

            // Send notifications/initialized
            let notif = json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            });
            let _ = self.send_rpc_message(&notif);
            self.set_connection_state(McpConnectionState::Connected, None);
            self.set_auth_challenge(None);

            log::debug!(
                "MCP Client: Handshake succeeded for HTTP server '{}' (negotiated protocol {})",
                self.server_name,
                self.negotiated_protocol_version()
            );
            return true;
        }

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref envs) = self.envs {
            cmd.envs(envs);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.set_connection_state(
                    McpConnectionState::Failed,
                    Some(format!("failed to spawn '{}': {}", self.command, e)),
                );
                log::error!("MCP Client: Failed to spawn '{}': {}", self.command, e);
                return false;
            }
        };

        let pid = child.id();
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                self.set_connection_state(
                    McpConnectionState::Failed,
                    Some("failed to capture stdout stream".to_string()),
                );
                log::error!("MCP Client: Failed to capture stdout stream");
                return false;
            }
        };
        let stdin = match child.stdin.take() {
            Some(s) => s,
            None => {
                self.set_connection_state(
                    McpConnectionState::Failed,
                    Some("failed to capture stdin stream".to_string()),
                );
                log::error!("MCP Client: Failed to capture stdin stream");
                return false;
            }
        };
        let stderr = match child.stderr.take() {
            Some(s) => s,
            None => {
                self.set_connection_state(
                    McpConnectionState::Failed,
                    Some("failed to capture stderr stream".to_string()),
                );
                log::error!("MCP Client: Failed to capture stderr stream");
                return false;
            }
        };

        self.reader = Some(Mutex::new(BufReader::new(stdout)));
        self.writer = Some(Mutex::new(stdin));
        self.child = Some(child);
        self.connected = true;
        self.set_negotiated_protocol_version(MCP_PROTOCOL_VERSION);
        self.set_connection_state(McpConnectionState::Disconnected, None);

        log::debug!("MCP Client: '{}' started (PID: {})", self.server_name, pid);

        // Spawn a background thread to read stderr and look for links/prompts
        let server_name_cloned = self.server_name.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        let trimmed = l.trim();
                        if !trimmed.is_empty() {
                            log::info!("MCP Server [{}] stderr: {}", server_name_cloned, trimmed);
                            if (trimmed.contains("http://") || trimmed.contains("https://"))
                                && !trimmed.contains("registry.npmjs.org")
                            {
                                log::warn!("**************************************************");
                                log::warn!(
                                    "MCP Server [{}] AUTHENTICATION LINK DETECTED!",
                                    server_name_cloned
                                );
                                log::warn!("Please open this link in any browser to authenticate:");
                                log::warn!("  {}", trimmed);
                                log::warn!("**************************************************");
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Perform initialize handshake
        let init_params = json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "clientInfo": {"name": "voxi-mcp-client", "version": "1.0.0"}
        });

        match self.send_request_sync("initialize", &init_params, 10000) {
            Ok(resp) => {
                if let Some(error) = resp.get("error") {
                    let message = format!("initialize returned JSON-RPC error: {}", error);
                    log::error!("MCP Client: Handshake failed for '{}'", self.server_name);
                    self.disconnect();
                    self.set_connection_state(McpConnectionState::Failed, Some(message));
                    return false;
                }
                self.record_protocol_from_initialize(&resp);
            }
            Err(e) => {
                log::error!("MCP Client: Init error for '{}': {}", self.server_name, e);
                self.disconnect();
                self.set_connection_state(
                    McpConnectionState::Failed,
                    Some(format!("initialize failed: {}", e)),
                );
                return false;
            }
        }

        // Send notifications/initialized
        let notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let _ = self.send_rpc_message(&notif);
        self.set_connection_state(McpConnectionState::Connected, None);

        log::debug!(
            "MCP Client: Handshake succeeded for '{}' (negotiated protocol {})",
            self.server_name,
            self.negotiated_protocol_version()
        );
        true
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.reader = None;
        self.writer = None;

        if self.is_http {
            self.reset_http_transport();
            self.set_connection_state(McpConnectionState::Disconnected, None);
            return;
        }

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.set_connection_state(McpConnectionState::Disconnected, None);
    }

    /// Discover tools from the remote server.
    pub fn discover_tools(&mut self) -> &[LlmToolDecl] {
        if !self.connected {
            return &self.tools;
        }

        match self.send_request_sync("tools/list", &json!({}), 5000) {
            Ok(resp) => {
                if let Some(tools_arr) = resp
                    .get("result")
                    .and_then(|r| r.get("tools"))
                    .and_then(|t| t.as_array())
                {
                    let mut used_names = HashSet::new();
                    self.tool_infos = tools_arr
                        .iter()
                        .filter_map(|t| {
                            let original_name = t["name"].as_str()?.to_string();
                            let description = t["description"].as_str().unwrap_or("").to_string();
                            let parameters = t
                                .get("inputSchema")
                                .cloned()
                                .unwrap_or_else(|| json!({"type": "object"}));
                            let base_safe_name =
                                safe_mcp_tool_name(&self.server_name, &original_name);
                            let mut safe_name = base_safe_name.clone();
                            let mut suffix = 2usize;
                            while !used_names.insert(safe_name.clone()) {
                                safe_name = format!("{}_{}", base_safe_name, suffix);
                                suffix += 1;
                            }

                            let legacy_name =
                                legacy_mcp_tool_name(&self.server_name, &original_name);
                            let searchable_text = build_searchable_text(
                                &self.server_name,
                                &original_name,
                                &safe_name,
                                &description,
                                &parameters,
                            );
                            Some(McpToolInfo {
                                server_name: self.server_name.clone(),
                                original_name,
                                safe_name,
                                legacy_name,
                                description,
                                parameters,
                                searchable_text,
                            })
                        })
                        .collect();
                    self.tools = self
                        .tool_infos
                        .iter()
                        .map(McpToolInfo::declaration)
                        .collect();
                }
            }
            Err(e) => {
                log::error!(
                    "MCP Client: tools/list error for '{}': {}",
                    self.server_name,
                    e
                );
            }
        }
        &self.tools
    }

    pub fn get_tools(&self) -> &[LlmToolDecl] {
        &self.tools
    }

    pub fn get_tool_infos(&self) -> &[McpToolInfo] {
        &self.tool_infos
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn status(&self) -> McpServerStatus {
        let state = self
            .connection_state
            .read()
            .map(|guard| guard.clone())
            .unwrap_or(McpConnectionState::Disconnected);
        let endpoint = if self.is_http {
            self.endpoint_url
                .read()
                .ok()
                .and_then(|guard| guard.clone())
        } else {
            Some(self.command.clone())
        };
        let negotiated_protocol_version = self
            .negotiated_protocol_version
            .read()
            .ok()
            .map(|guard| guard.clone())
            .filter(|value| !value.trim().is_empty());
        let auth_required = state == McpConnectionState::AuthRequired;
        McpServerStatus {
            name: self.server_name.clone(),
            transport: if self.is_http { "http" } else { "stdio" }.to_string(),
            state,
            connected: self.connected,
            auth_required,
            has_access_token: self.is_http && self.find_access_token().is_some(),
            tool_count: self.tools.len(),
            endpoint,
            negotiated_protocol_version,
            message: self.status_message().or_else(|| self.auth_challenge()),
            suggested_command: (self.is_http && auth_required)
                .then(|| format!("/mcp login {}", self.server_name)),
        }
    }

    fn resolve_remote_tool_name<'a>(&'a self, full_name: &'a str) -> Option<&'a str> {
        self.tool_infos
            .iter()
            .find(|tool| {
                tool.safe_name == full_name
                    || tool.legacy_name == full_name
                    || tool.original_name == full_name
            })
            .map(|tool| tool.original_name.as_str())
            .or_else(move || {
                let prefix = format!("mcp_{}_", self.server_name);
                full_name.strip_prefix(&prefix)
            })
    }

    pub fn get_expected_parameter_keys(&self) -> std::collections::HashSet<String> {
        let mut keys = std::collections::HashSet::new();
        for tool in &self.tool_infos {
            if let Some(properties) = tool.parameters.get("properties").and_then(|p| p.as_object()) {
                for key in properties.keys() {
                    keys.insert(key.clone());
                }
            }
        }
        keys
    }

    /// Call a tool on the remote server.
    pub fn call_tool(&mut self, tool_name: &str, arguments: &Value) -> Value {
        if !self.connected {
            return json!({"error": "Not connected"});
        }
        self.update_last_used();

        let params = json!({"name": tool_name, "arguments": arguments});
        match self.send_request_sync("tools/call", &params, self.timeout_ms as i64) {
            Ok(resp) => {
                if let Some(result) = resp.get("result") {
                    result.clone()
                } else if let Some(error) = resp.get("error") {
                    json!({"isError": true, "error": error})
                } else {
                    json!({"isError": true, "error": "Invalid response"})
                }
            }
            Err(e) => json!({"isError": true, "error": e.to_string()}),
        }
    }

    fn send_rpc_message(&self, message: &Value) -> Result<(), String> {
        if self.is_http {
            let client = self.http_client.as_ref().ok_or("No HTTP client")?;
            let ep = self
                .endpoint_url
                .read()
                .unwrap()
                .clone()
                .ok_or("No message endpoint URL")?;

            let mut req = client
                .post(&ep)
                .header(
                    reqwest::header::ACCEPT,
                    "application/json, text/event-stream",
                )
                .header("MCP-Protocol-Version", self.negotiated_protocol_version());
            req = self.apply_configured_http_headers(req);

            if let Some(access_token) = self.find_access_token() {
                req = req.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", access_token),
                );
            }

            let current_session = self
                .mcp_session_id
                .read()
                .ok()
                .and_then(|session| session.clone())
                .or_else(|| self.find_mcp_session_id());
            if let Some(ref sid) = current_session {
                req = req.header("Mcp-Session-Id", sid);
            }

            let resp = req.json(message).send().map_err(|e| e.to_string())?;

            let status = resp.status();
            let auth_challenge = resp
                .headers()
                .get(reqwest::header::WWW_AUTHENTICATE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.trim().to_string())
                .unwrap_or_default();
            self.capture_mcp_session_id_from_headers(&resp);
            let body = resp.text().map_err(|e| e.to_string())?;

            if !status.is_success() {
                if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                {
                    self.set_auth_challenge(
                        (!auth_challenge.is_empty()).then_some(auth_challenge.clone()),
                    );
                    self.set_connection_state(
                        McpConnectionState::AuthRequired,
                        Some("OAuth bearer token required or expired".to_string()),
                    );
                }
                if status == reqwest::StatusCode::NOT_FOUND && current_session.is_some() {
                    if let Ok(mut session) = self.mcp_session_id.write() {
                        *session = None;
                    }
                    self.set_connection_state(
                        McpConnectionState::Disconnected,
                        Some("MCP HTTP session expired; reinitialize required".to_string()),
                    );
                }
                let auth_hint = if auth_challenge.is_empty() {
                    String::new()
                } else {
                    format!("; WWW-Authenticate: {}", auth_challenge)
                };
                let body_hint = if body.trim().is_empty() {
                    String::new()
                } else {
                    format!(
                        "; body: {}",
                        body.trim().chars().take(500).collect::<String>()
                    )
                };
                return Err(format!(
                    "HTTP POST failed: {}{}{}",
                    status, auth_hint, body_hint
                ));
            }

            if status == reqwest::StatusCode::ACCEPTED || body.trim().is_empty() {
                return Ok(());
            }

            if let Some(value) = Self::parse_http_rpc_body(&body)? {
                self.remember_http_response(value);
            }
            return Ok(());
        }

        let writer = self.writer.as_ref().ok_or("No writer")?;
        let mut writer = writer.lock().map_err(|e| e.to_string())?;
        let data = format!("{}\n", message);
        writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn read_rpc_message(&self, timeout_ms: i64) -> Result<Value, String> {
        let reader = self.reader.as_ref().ok_or("No reader")?;
        let mut reader = reader.lock().map_err(|e| e.to_string())?;

        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms as u64);
        let mut line = String::new();

        loop {
            if start.elapsed() >= timeout {
                return Err("Timeout".into());
            }

            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => return Err("EOF".into()),
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    return serde_json::from_str(trimmed).map_err(|e| e.to_string());
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    fn send_request_sync(
        &self,
        method: &str,
        params: &Value,
        timeout_ms: i64,
    ) -> Result<Value, String> {
        let req_id = self.next_req_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params
        });

        self.send_rpc_message(&request)?;

        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms as u64);

        if self.is_http {
            loop {
                if start.elapsed() >= timeout {
                    return Err(format!(
                        "Timeout after {}ms waiting for HTTP response",
                        timeout_ms
                    ));
                }

                if let Ok(mut map) = self.received_responses.lock() {
                    if let Some(resp) = map.remove(&req_id) {
                        return Ok(resp);
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        loop {
            if start.elapsed() >= timeout {
                return Err(format!("Timeout after {}ms", timeout_ms));
            }

            let remaining = timeout
                .checked_sub(start.elapsed())
                .unwrap_or(Duration::from_millis(1));
            let resp = self.read_rpc_message(remaining.as_millis() as i64)?;

            // Check for matching ID
            if resp.get("id").and_then(|v| v.as_i64()) == Some(req_id as i64) {
                return Ok(resp);
            }

            // Handle notifications
            if let Some(m) = resp.get("method").and_then(|v| v.as_str()) {
                log::debug!(
                    "MCP Client: notification from '{}': {}",
                    self.server_name,
                    m
                );
            }
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Manages multiple MCP client connections.
pub struct McpClientManager {
    clients: Vec<McpClient>,
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClientManager {
    pub fn new() -> Self {
        McpClientManager {
            clients: Vec::new(),
        }
    }

    fn disconnect_all(&mut self) {
        for client in &mut self.clients {
            client.disconnect();
        }
        self.clients.clear();
    }

    /// Load MCP server configs from JSON and connect.
    ///
    /// Config format:
    /// ```json
    /// { "servers": [{"name": "x", "command": "/usr/bin/x", "args": ["--stdio"]}] }
    /// ```
    pub fn load_config_and_connect(&mut self, path: &str) -> bool {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let config: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return false,
        };

        self.disconnect_all();
        let mut connected_count = 0usize;

        if let Some(servers_map) = config["mcpServers"].as_object() {
            for (name, s) in servers_map {
                let mut command = s["command"].as_str().unwrap_or("").to_string();
                let mut args: Vec<String> = s["args"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let timeout = s["timeout_ms"].as_u64().unwrap_or(30000);

                let env_map: Option<std::collections::HashMap<String, String>> =
                    s.get("env").and_then(|v| v.as_object()).map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    });
                let header_map: Option<std::collections::HashMap<String, String>> =
                    s.get("headers").and_then(|v| v.as_object()).map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    });

                let mcp_type = s["type"].as_str().unwrap_or("stdio");
                if mcp_type == "http" {
                    if let Some(url) = s["url"].as_str() {
                        command = url.to_string();
                    }
                }

                if name.is_empty() || command.is_empty() {
                    continue;
                }

                let mut client = McpClient::new(name, &command, &args, timeout)
                    .with_env(env_map)
                    .with_http_headers(header_map);
                if client.connect() {
                    client.discover_tools();
                    connected_count += 1;
                    log::debug!(
                        "MCP Client: '{}' connected ({} tools)",
                        name,
                        client.get_tools().len()
                    );
                } else {
                    let status = client.status();
                    if status.auth_required {
                        log::warn!(
                            "MCP Client: '{}' is auth-required; run `{}`",
                            name,
                            status
                                .suggested_command
                                .as_deref()
                                .unwrap_or("/mcp login <server>")
                        );
                    } else if let Some(message) = status.message.as_deref() {
                        log::warn!("MCP Client: '{}' not connected: {}", name, message);
                    }
                }
                self.clients.push(client);
            }
        } else if let Some(servers) = config["servers"].as_array() {
            for s in servers {
                let name = s["name"].as_str().unwrap_or("").to_string();
                let mut command = s["command"].as_str().unwrap_or("").to_string();
                let mut args: Vec<String> = s["args"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let timeout = s["timeout_ms"].as_u64().unwrap_or(30000);

                let env_map: Option<std::collections::HashMap<String, String>> =
                    s.get("env").and_then(|v| v.as_object()).map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    });
                let header_map: Option<std::collections::HashMap<String, String>> =
                    s.get("headers").and_then(|v| v.as_object()).map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    });

                let mcp_type = s["type"].as_str().unwrap_or("stdio");
                if mcp_type == "http" {
                    if let Some(url) = s["url"].as_str() {
                        command = url.to_string();
                    }
                }

                if name.is_empty() || command.is_empty() {
                    continue;
                }

                let mut client = McpClient::new(&name, &command, &args, timeout)
                    .with_env(env_map)
                    .with_http_headers(header_map);
                if client.connect() {
                    client.discover_tools();
                    connected_count += 1;
                    log::debug!(
                        "MCP Client: '{}' connected ({} tools)",
                        name,
                        client.get_tools().len()
                    );
                } else {
                    let status = client.status();
                    if status.auth_required {
                        log::warn!(
                            "MCP Client: '{}' is auth-required; run `{}`",
                            name,
                            status
                                .suggested_command
                                .as_deref()
                                .unwrap_or("/mcp login <server>")
                        );
                    } else if let Some(message) = status.message.as_deref() {
                        log::warn!("MCP Client: '{}' not connected: {}", name, message);
                    }
                }
                self.clients.push(client);
            }
        }

        connected_count > 0
    }

    /// Get all tools from all connected clients.
    pub fn get_all_tools(&self) -> Vec<LlmToolDecl> {
        self.clients
            .iter()
            .flat_map(|c| c.get_tools().to_vec())
            .collect()
    }

    pub fn get_all_tool_infos(&self) -> Vec<McpToolInfo> {
        self.clients
            .iter()
            .flat_map(|client| client.get_tool_infos().to_vec())
            .collect()
    }

    pub fn statuses(&self) -> Vec<McpServerStatus> {
        self.clients.iter().map(McpClient::status).collect()
    }

    pub fn search_tools(&self, query: &str, limit: usize) -> Vec<McpToolSearchResult> {
        let query = query.trim();
        let include_all = query.is_empty() || query.eq_ignore_ascii_case("ALL");
        let all_tools = self.get_all_tool_infos();
        let mut results = self
            .clients
            .iter()
            .flat_map(|client| client.get_tool_infos())
            .filter_map(|tool| {
                let score = if include_all {
                    1
                } else {
                    fuzzy_score(query, &tool.searchable_text)
                };
                (score > 0).then_some(McpToolSearchResult {
                    score,
                    tool: tool.clone(),
                    behavior: McpToolBehavior::from_tool(tool, &all_tools),
                })
            })
            .collect::<Vec<_>>();

        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.tool.safe_name.cmp(&right.tool.safe_name))
        });
        results.truncate(limit.max(1));
        results
    }

    pub fn get_all_tool_behaviors(&self) -> Vec<McpToolBehavior> {
        let all_tools = self.get_all_tool_infos();
        all_tools
            .iter()
            .map(|tool| McpToolBehavior::from_tool(tool, &all_tools))
            .collect()
    }

    pub fn requires_confirmation(&self, full_name: &str, keywords: &[String]) -> bool {
        let name_lower = full_name.to_ascii_lowercase();

        // Check if this is a high-risk tool name explicitly
        let is_high_risk = name_lower.contains("checkout")
            || name_lower.contains("pay")
            || name_lower.contains("payment")
            || name_lower.contains("place_order")
            || name_lower.contains("book")
            || name_lower.contains("reserve");

        // Check if this is a low-risk/query tool name
        let is_low_risk = name_lower.contains("search")
            || name_lower.contains("list")
            || name_lower.contains("view")
            || name_lower.contains("get")
            || name_lower.contains("cart");

        if is_low_risk && !is_high_risk {
            return false;
        }

        let matches_name = keywords.iter().any(|keyword| {
            let keyword = keyword.trim().to_ascii_lowercase();
            !keyword.is_empty() && name_lower.contains(&keyword)
        });
        if matches_name {
            return true;
        }

        let original_name = self
            .clients
            .iter()
            .flat_map(|client| client.get_tool_infos())
            .find(|tool| {
                tool.safe_name == full_name
                    || tool.legacy_name == full_name
                    || tool.original_name == full_name
            })
            .map(|tool| tool.original_name.to_ascii_lowercase());

        if let Some(orig) = original_name {
            // Also apply the low-risk bypass to the original name
            let is_orig_high_risk = orig.contains("checkout")
                || orig.contains("pay")
                || orig.contains("payment")
                || orig.contains("place_order")
                || orig.contains("book")
                || orig.contains("reserve");
            let is_orig_low_risk = orig.contains("search")
                || orig.contains("list")
                || orig.contains("view")
                || orig.contains("get")
                || orig.contains("cart");
            if is_orig_low_risk && !is_orig_high_risk {
                return false;
            }

            return keywords.iter().any(|keyword| {
                let keyword = keyword.trim().to_ascii_lowercase();
                !keyword.is_empty() && orig.contains(&keyword)
            });
        }

        false
    }

    fn resolve_tool_alias_with_client(
        &self,
        requested_name: &str,
    ) -> Result<(usize, McpToolInfo), McpToolResolveError> {
        let matchers: [fn(&McpToolInfo, &str) -> bool; 3] = [
            |tool: &McpToolInfo, name: &str| tool.safe_name == name,
            |tool: &McpToolInfo, name: &str| tool.legacy_name == name,
            |tool: &McpToolInfo, name: &str| tool.original_name == name,
        ];
        for matcher in matchers {
            let matches = self
                .clients
                .iter()
                .enumerate()
                .filter(|(_, client)| client.is_connected())
                .flat_map(|(client_index, client)| {
                    client
                        .get_tool_infos()
                        .iter()
                        .filter(move |tool| matcher(tool, requested_name))
                        .cloned()
                        .map(move |tool| (client_index, tool))
                })
                .collect::<Vec<_>>();

            match matches.len() {
                0 => continue,
                1 => return Ok(matches.into_iter().next().unwrap()),
                _ => {
                    return Err(McpToolResolveError::Ambiguous(
                        matches.into_iter().map(|(_, tool)| tool).collect(),
                    ));
                }
            }
        }

        Err(McpToolResolveError::NotFound)
    }

    fn get_or_create_zepto_device_id(&self) -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let dir = std::path::PathBuf::from(home).join(".voxi");
        let file_path = dir.join("zepto_device_id");
        
        if let Ok(id) = std::fs::read_to_string(&file_path) {
            let trimmed = id.trim().to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
        
        let _ = std::fs::create_dir_all(&dir);
        let new_id = uuid::Uuid::new_v4().to_string();
        let _ = std::fs::write(&file_path, &new_id);
        new_id
    }

    pub fn resolve_tool_alias(
        &self,
        requested_name: &str,
    ) -> Result<McpToolInfo, McpToolResolveError> {
        self.resolve_tool_alias_with_client(requested_name)
            .map(|(_, tool)| tool)
    }

    pub fn call_tool_resolved(
        &mut self,
        requested_name: &str,
        args: &Value,
    ) -> Result<Value, McpToolResolveError> {
        let (client_index, tool_info) = self.resolve_tool_alias_with_client(requested_name)?;
        
        let mut final_args = args.clone();
        if tool_info.server_name == "zepto" {
            if let Some(obj) = final_args.as_object_mut() {
                if !obj.contains_key("deviceId") && !obj.contains_key("device_id") {
                    let dev_id = self.get_or_create_zepto_device_id();
                    obj.insert("deviceId".to_string(), Value::String(dev_id.clone()));
                    obj.insert("device_id".to_string(), Value::String(dev_id));
                }
            }
        }
        
        Ok(self.clients[client_index].call_tool(&tool_info.original_name, &final_args))
    }

    /// Route a tool call to the appropriate client.
    pub fn call_tool(&mut self, full_name: &str, args: &Value) -> Option<Value> {
        if let Ok(result) = self.call_tool_resolved(full_name, args) {
            return Some(result);
        }

        None
    }

    pub fn get_client(&self, name: &str) -> Option<&McpClient> {
        self.clients.iter().find(|c| c.server_name == name)
    }

    pub fn get_server_parameter_keys(&self, server_name: &str) -> std::collections::HashSet<String> {
        if let Some(client) = self.get_client(server_name) {
            client.get_expected_parameter_keys()
        } else {
            std::collections::HashSet::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tool(server: &str, original: &str) -> McpToolInfo {
        let safe_name = safe_mcp_tool_name(server, original);
        McpToolInfo {
            server_name: server.to_string(),
            original_name: original.to_string(),
            safe_name: safe_name.clone(),
            legacy_name: legacy_mcp_tool_name(server, original),
            description: String::new(),
            parameters: json!({"type": "object"}),
            searchable_text: build_searchable_text(
                server,
                original,
                &safe_name,
                "",
                &json!({"type": "object"}),
            ),
        }
    }

    fn test_client(server: &str, originals: &[&str]) -> McpClient {
        let mut client = McpClient::new(server, "test-command", &[], 30000);
        client.connected = true;
        client.tool_infos = originals
            .iter()
            .map(|original| test_tool(server, original))
            .collect();
        client.tools = client
            .tool_infos
            .iter()
            .map(McpToolInfo::declaration)
            .collect();
        client
    }

    #[test]
    fn resolve_tool_alias_matches_safe_legacy_and_original_names() {
        let manager = McpClientManager {
            clients: vec![test_client("swiggy-instamart", &["list_saved_addresses"])],
        };

        let safe = manager
            .resolve_tool_alias("mcp_swiggy_instamart_list_saved_addresses")
            .unwrap();
        assert_eq!(safe.original_name, "list_saved_addresses");

        let legacy = manager
            .resolve_tool_alias("mcp_swiggy-instamart_list_saved_addresses")
            .unwrap();
        assert_eq!(
            legacy.safe_name,
            "mcp_swiggy_instamart_list_saved_addresses"
        );

        let original = manager.resolve_tool_alias("list_saved_addresses").unwrap();
        assert_eq!(
            original.safe_name,
            "mcp_swiggy_instamart_list_saved_addresses"
        );
    }

    #[test]
    fn resolve_tool_alias_fails_closed_on_ambiguous_original_name() {
        let manager = McpClientManager {
            clients: vec![
                test_client("zepto", &["list_saved_addresses"]),
                test_client("swiggy", &["list_saved_addresses"]),
            ],
        };

        match manager.resolve_tool_alias("list_saved_addresses") {
            Err(McpToolResolveError::Ambiguous(tools)) => {
                assert_eq!(tools.len(), 2);
                assert!(tools
                    .iter()
                    .any(|tool| tool.safe_name == "mcp_zepto_list_saved_addresses"));
                assert!(tools
                    .iter()
                    .any(|tool| tool.safe_name == "mcp_swiggy_list_saved_addresses"));
            }
            other => panic!("expected ambiguous original-name match, got {:?}", other),
        }
    }

    #[test]
    fn requires_confirmation_checks_original_tool_alias() {
        let manager = McpClientManager {
            clients: vec![test_client("zepto", &["checkout"])],
        };
        let keywords = vec!["checkout".to_string()];

        assert!(manager.requires_confirmation("checkout", &keywords));
        assert!(manager.requires_confirmation("mcp_zepto_checkout", &keywords));
    }

    #[test]
    fn test_find_access_token() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("VOXI_DATA_DIR", temp_dir.path());

        // Test with flag-value pair
        let client_flag = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &[
                "--access-token".to_string(),
                "my_test_session_token_123".to_string(),
            ],
            30000,
        );
        assert_eq!(
            client_flag.find_access_token(),
            Some("my_test_session_token_123".to_string())
        );

        // Test with inline flag parameter
        let client_inline = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &["--access-token=my_inline_session_token_456".to_string()],
            30000,
        );
        assert_eq!(
            client_inline.find_access_token(),
            Some("my_inline_session_token_456".to_string())
        );

        // Test with direct token argument
        let client_direct = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &["my_raw_direct_token_value_789".to_string()],
            30000,
        );
        assert_eq!(
            client_direct.find_access_token(),
            Some("my_raw_direct_token_value_789".to_string())
        );

        // Test with direct token argument when URL is also in args
        let client_direct_with_url = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &[
                "https://mcp.zepto.co.in/mcp".to_string(),
                "token_directly_appended_abc".to_string(),
            ],
            30000,
        );
        assert_eq!(
            client_direct_with_url.find_access_token(),
            Some("token_directly_appended_abc".to_string())
        );

        // Test with no token
        let client_empty = McpClient::new("zepto", "https://mcp.zepto.co.in/mcp", &[], 30000);
        assert_eq!(client_empty.find_access_token(), None);

        std::env::remove_var("VOXI_DATA_DIR");
    }

    #[test]
    fn test_find_access_token_secrets() {
        let temp_dir = tempfile::tempdir().unwrap();
        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();

        let token_file = secrets_dir.join("swiggy_food_token");
        std::fs::write(&token_file, "secret_swiggy_token_val").unwrap();

        std::env::set_var("VOXI_DATA_DIR", temp_dir.path());

        let client = McpClient::new("swiggy-instamart", "https://mcp.swiggy.com/im", &[], 30000);

        assert_eq!(
            client.find_access_token(),
            Some("secret_swiggy_token_val".to_string())
        );

        std::env::remove_var("VOXI_DATA_DIR");
    }
}
