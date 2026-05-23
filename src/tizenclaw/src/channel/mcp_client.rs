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
}

impl McpToolSearchResult {
    pub fn to_json(&self) -> Value {
        self.tool.to_search_json(self.score)
    }
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

    // New fields for HTTP/SSE transport
    is_http: bool,
    http_url: String,
    http_client: Option<reqwest::blocking::Client>,
    endpoint_url: Arc<RwLock<Option<String>>>,
    received_responses: Arc<Mutex<std::collections::HashMap<i32, Value>>>,
    stop_signal: Arc<AtomicBool>,
    sse_thread: Option<std::thread::JoinHandle<()>>,
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
        let mut target_url = url.clone();
        if let Some(ref sid) = session_id {
            if !target_url.contains("session=") && !target_url.contains("session_id=") {
                let separator = if target_url.contains('?') { "&" } else { "?" };
                target_url = format!("{}{}session={}", target_url, separator, sid);
            }
        }

        let mut req = client.get(&target_url)
            .header("Accept", "text/event-stream");

        if let Some(ref sid) = session_id {
            req = req.header("x-session-id", sid)
                     .header("x-mcp-session-id", sid)
                     .header("Authorization", format!("Bearer {}", sid))
                     .header("Cookie", format!("session_id={}; session={}", sid, sid));
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
            log::error!("SSE connection failed with status: {}", resp.status());
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
                            let resolved = if data.starts_with("http://") || data.starts_with("https://") {
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
        let http_url = if is_http { command.to_string() } else { String::new() };

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
            endpoint_url: Arc::new(RwLock::new(None)),
            received_responses: Arc::new(Mutex::new(std::collections::HashMap::new())),
            stop_signal: Arc::new(AtomicBool::new(false)),
            sse_thread: None,
        }
    }

    pub fn with_env(mut self, envs: Option<std::collections::HashMap<String, String>>) -> Self {
        self.envs = envs;
        self
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

    fn find_session_id(&self) -> Option<String> {
        let token_flags = vec!["--session", "--mcp-session-id", "oauth_token", "--token", "token"];
        
        // 1. Look for standard flag-value pairs or inline parameters in args
        let mut i = 0;
        while i < self.args.len() {
            if let Some(arg) = self.args.get(i) {
                if token_flags.contains(&arg.as_str()) && i + 1 < self.args.len() {
                    return self.args.get(i + 1).cloned();
                }
                for flag in &token_flags {
                    let prefix = format!("{}=", flag);
                    if arg.starts_with(&prefix) {
                        return Some(arg[prefix.len()..].to_string());
                    }
                }
            }
            i += 1;
        }

        // 2. Look for raw direct token in args
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

        // 3. Look in env variables
        if let Some(ref envs) = self.envs {
            let env_session_key = format!("{}_SESSION", self.server_name.to_ascii_uppercase());
            let keys = vec![
                "SESSION_ID",
                "MCP_SESSION_ID",
                env_session_key.as_str(),
            ];
            for key in keys {
                if let Some(val) = envs.get(key) {
                    return Some(val.clone());
                }
            }
        }
        None
    }

    fn extract_and_save_session_from_headers(&self, resp: &reqwest::blocking::Response) {
        // Only run this if we have --session, --mcp-session-id, or if this is zepto
        let has_session_indicator = self.args.iter().any(|arg| arg == "--session" || arg == "--mcp-session-id") 
            || self.server_name.contains("zepto");
            
        if !has_session_indicator {
            return;
        }

        let mut new_session_id = None;

        // Try extracting from Cookie headers
        if let Some(cookie) = resp.headers().get("set-cookie").and_then(|v| v.to_str().ok()) {
            for cookie_part in cookie.split(';') {
                let cookie_part = cookie_part.trim();
                if let Some(pos) = cookie_part.find('=') {
                    let key = cookie_part[..pos].trim().to_ascii_lowercase();
                    let val = cookie_part[pos + 1..].trim();
                    if key == "session_id" || key == "session" || key == "token" || key == "oauth_token" || key == "mcp_session_id" || key == "mcp-session-id" {
                        new_session_id = Some(val.to_string());
                        break;
                    }
                }
            }
        }

        // Try extracting from other custom headers
        if new_session_id.is_none() {
            let header_names = vec![
                "x-session-id",
                "x-mcp-session-id",
                "session-id",
                "mcp-session-id",
                "session_id",
                "session",
                "token",
                "authorization"
            ];
            for name in header_names {
                if let Some(val) = resp.headers().get(name).and_then(|v| v.to_str().ok()) {
                    let val = val.trim();
                    let token_val = if name == "authorization" && val.to_ascii_lowercase().starts_with("bearer ") {
                        val["bearer ".len()..].trim().to_string()
                    } else {
                        val.to_string()
                    };
                    if !token_val.is_empty() {
                        new_session_id = Some(token_val);
                        break;
                    }
                }
            }
        }

        if let Some(sid) = new_session_id {
            let current = self.find_session_id();
            if current.as_ref() != Some(&sid) {
                log::warn!("Auto-detected updated session token from HTTP headers for server '{}': {}", self.server_name, sid);
                let data_dir = std::env::var("TIZENCLAW_DATA_DIR")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| {
                        std::env::var("HOME")
                            .map(|h| std::path::PathBuf::from(h).join(".tizenclaw"))
                            .unwrap_or_else(|_| std::path::PathBuf::from("/opt/usr/share/tizenclaw"))
                    });
                let config_path = data_dir.join("config").join("mcp_servers.json");
                if config_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(mut json_val) = serde_json::from_str::<Value>(&content) {
                            if let Some(servers) = json_val.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                                if let Some(server) = servers.get_mut(&self.server_name) {
                                    let mut updated = false;
                                    
                                    let token_flags = vec!["--session", "--mcp-session-id", "oauth_token", "--token", "token"];
                                    let mut target_flag = None;
                                    if let Some(args_arr) = server.get("args").and_then(|v| v.as_array()) {
                                        for flag in &token_flags {
                                            if args_arr.iter().any(|arg| arg.as_str() == Some(*flag)) {
                                                target_flag = Some(flag.to_string());
                                                break;
                                            }
                                        }
                                    }
                                    let target_flag = target_flag.unwrap_or_else(|| {
                                        if self.server_name.contains("zepto") {
                                            "--session".to_string()
                                        } else {
                                            "--mcp-session-id".to_string()
                                        }
                                    });

                                    if let Some(args_arr) = server.get_mut("args").and_then(|v| v.as_array_mut()) {
                                        let mut i = 0;
                                        while i < args_arr.len() {
                                            if let Some(arg_str) = args_arr[i].as_str() {
                                                if arg_str == target_flag && i + 1 < args_arr.len() {
                                                    args_arr[i + 1] = serde_json::json!(sid);
                                                    updated = true;
                                                    break;
                                                }
                                            }
                                            i += 1;
                                        }
                                        if !updated {
                                            if args_arr.len() == 1 && !args_arr[0].as_str().unwrap_or("").starts_with('-') {
                                                args_arr[0] = serde_json::json!(sid);
                                            } else {
                                                args_arr.push(serde_json::json!(target_flag));
                                                args_arr.push(serde_json::json!(sid));
                                            }
                                        }
                                    } else {
                                        let mut arr = Vec::new();
                                        arr.push(serde_json::json!(target_flag));
                                        arr.push(serde_json::json!(sid));
                                        server.as_object_mut().unwrap().insert("args".to_string(), Value::Array(arr));
                                    }
                                    
                                    let env_key = format!("{}_SESSION", self.server_name.to_ascii_uppercase());
                                    if let Some(env_obj) = server.get_mut("env").and_then(|v| v.as_object_mut()) {
                                        env_obj.insert(env_key, serde_json::json!(sid));
                                        env_obj.insert("SESSION_ID".to_string(), serde_json::json!(sid));
                                        env_obj.insert("MCP_SESSION_ID".to_string(), serde_json::json!(sid));
                                    } else {
                                        let mut env_map = serde_json::Map::new();
                                        env_map.insert(env_key, serde_json::json!(sid));
                                        env_map.insert("SESSION_ID".to_string(), serde_json::json!(sid));
                                        env_map.insert("MCP_SESSION_ID".to_string(), serde_json::json!(sid));
                                        server.as_object_mut().unwrap().insert("env".to_string(), Value::Object(env_map));
                                    }
                                    
                                    if let Ok(new_content) = serde_json::to_string_pretty(&json_val) {
                                        let _ = std::fs::write(&config_path, new_content);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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
                    log::error!(
                        "MCP Client: Failed to build HTTP client for '{}': {}",
                        self.server_name,
                        e
                    );
                    return false;
                }
            };

            self.http_client = Some(client.clone());
            self.stop_signal.store(false, Ordering::SeqCst);

            let url = self.http_url.clone();
            let session_id = self.find_session_id();
            let endpoint_url = self.endpoint_url.clone();
            let received_responses = self.received_responses.clone();
            let stop_signal = self.stop_signal.clone();

            let handle = std::thread::spawn(move || {
                run_sse_listener(client, url, session_id, endpoint_url, received_responses, stop_signal);
            });
            self.sse_thread = Some(handle);
            self.connected = true;

            // Wait up to 5s for endpoint to resolve via SSE
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                if self.endpoint_url.read().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            // Fallback if no SSE endpoint was resolved
            if self.endpoint_url.read().unwrap().is_none() {
                let mut ep = self.endpoint_url.write().unwrap();
                *ep = Some(self.http_url.clone());
            }

            log::debug!("MCP Client: Native HTTP transport established for '{}'", self.server_name);

            // Perform initialize handshake
            let init_params = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "tizenclaw-mcp-client", "version": "1.0.0"}
            });

            match self.send_request_sync("initialize", &init_params, 10000) {
                Ok(resp) => {
                    if resp.get("error").is_some() {
                        log::error!("MCP Client: Handshake failed for '{}'", self.server_name);
                        self.disconnect();
                        return false;
                    }
                }
                Err(e) => {
                    log::error!("MCP Client: Init error for '{}': {}", self.server_name, e);
                    self.disconnect();
                    return false;
                }
            }

            // Send notifications/initialized
            let notif = json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            });
            let _ = self.send_rpc_message(&notif);

            log::debug!("MCP Client: Handshake succeeded for HTTP server '{}'", self.server_name);
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
                log::error!("MCP Client: Failed to spawn '{}': {}", self.command, e);
                return false;
            }
        };

        let pid = child.id();
        let stdout = child.stdout.take().unwrap();
        let stdin = child.stdin.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        self.reader = Some(Mutex::new(BufReader::new(stdout)));
        self.writer = Some(Mutex::new(stdin));
        self.child = Some(child);
        self.connected = true;

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
                            if trimmed.contains("http://") || trimmed.contains("https://") {
                                log::warn!("**************************************************");
                                log::warn!("MCP Server [{}] AUTHENTICATION LINK DETECTED!", server_name_cloned);
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
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "tizenclaw-mcp-client", "version": "1.0.0"}
        });

        match self.send_request_sync("initialize", &init_params, 10000) {
            Ok(resp) => {
                if resp.get("error").is_some() {
                    log::error!("MCP Client: Handshake failed for '{}'", self.server_name);
                    self.disconnect();
                    return false;
                }
            }
            Err(e) => {
                log::error!("MCP Client: Init error for '{}': {}", self.server_name, e);
                self.disconnect();
                return false;
            }
        }

        // Send notifications/initialized
        let notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let _ = self.send_rpc_message(&notif);

        log::debug!("MCP Client: Handshake succeeded for '{}'", self.server_name);
        true
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.reader = None;
        self.writer = None;

        if self.is_http {
            self.stop_signal.store(true, Ordering::SeqCst);
            if let Some(handle) = self.sse_thread.take() {
                let _ = handle.join();
            }
            self.http_client = None;
            if let Ok(mut ep) = self.endpoint_url.write() {
                *ep = None;
            }
            if let Ok(mut map) = self.received_responses.lock() {
                map.clear();
            }
            return;
        }

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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

    fn resolve_remote_tool_name<'a>(&'a self, full_name: &'a str) -> Option<&'a str> {
        self.tool_infos
            .iter()
            .find(|tool| tool.safe_name == full_name || tool.legacy_name == full_name)
            .map(|tool| tool.original_name.as_str())
            .or_else(move || {
                let prefix = format!("mcp_{}_", self.server_name);
                full_name.strip_prefix(&prefix)
            })
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
            let ep = self.endpoint_url.read().unwrap().clone().ok_or("No message endpoint URL")?;

            let session_id = self.find_session_id();
            let mut target_ep = ep;
            if let Some(ref sid) = session_id {
                if !target_ep.contains("session=") && !target_ep.contains("session_id=") {
                    let separator = if target_ep.contains('?') { "&" } else { "?" };
                    target_ep = format!("{}{}session={}", target_ep, separator, sid);
                }
            }

            let mut req = client.post(&target_ep);
            if let Some(ref sid) = session_id {
                req = req.header("x-session-id", sid)
                         .header("x-mcp-session-id", sid)
                         .header("Authorization", format!("Bearer {}", sid))
                         .header("Cookie", format!("session_id={}; session={}", sid, sid));
            }

            let resp = req.json(message)
                .send()
                .map_err(|e| e.to_string())?;

            if !resp.status().is_success() {
                return Err(format!("HTTP POST failed: {}", resp.status()));
            }

            self.extract_and_save_session_from_headers(&resp);

            // Check if server returns synchronous response directly in body
            if let Ok(v) = resp.json::<Value>() {
                if v.get("jsonrpc").is_some() && (v.get("result").is_some() || v.get("error").is_some()) {
                    if let Some(id) = v.get("id").and_then(|id_val| id_val.as_i64()) {
                        if let Ok(mut map) = self.received_responses.lock() {
                            map.insert(id as i32, v);
                        }
                    }
                }
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
                    return Err(format!("Timeout after {}ms waiting for HTTP response", timeout_ms));
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

                let env_map: Option<std::collections::HashMap<String, String>> = s.get("env")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
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

                let mut client = McpClient::new(name, &command, &args, timeout).with_env(env_map);
                if client.connect() {
                    client.discover_tools();
                    connected_count += 1;
                    log::debug!(
                        "MCP Client: '{}' connected ({} tools)",
                        name,
                        client.get_tools().len()
                    );
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

                let env_map: Option<std::collections::HashMap<String, String>> = s.get("env")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
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

                let mut client = McpClient::new(&name, &command, &args, timeout).with_env(env_map);
                if client.connect() {
                    client.discover_tools();
                    connected_count += 1;
                    log::debug!(
                        "MCP Client: '{}' connected ({} tools)",
                        name,
                        client.get_tools().len()
                    );
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

    pub fn search_tools(&self, query: &str, limit: usize) -> Vec<McpToolSearchResult> {
        let query = query.trim();
        let include_all = query.is_empty() || query.eq_ignore_ascii_case("ALL");
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

    pub fn requires_confirmation(&self, full_name: &str, keywords: &[String]) -> bool {
        let fallback = full_name.to_ascii_lowercase();
        let searchable = self
            .clients
            .iter()
            .flat_map(|client| client.get_tool_infos())
            .find(|tool| tool.safe_name == full_name || tool.legacy_name == full_name)
            .map(|tool| tool.searchable_text.to_ascii_lowercase())
            .unwrap_or(fallback);

        keywords.iter().any(|keyword| {
            let keyword = keyword.trim().to_ascii_lowercase();
            !keyword.is_empty() && searchable.contains(&keyword)
        })
    }

    /// Route a tool call to the appropriate client.
    pub fn call_tool(&mut self, full_name: &str, args: &Value) -> Option<Value> {
        for client in &mut self.clients {
            if let Some(tool_name) = client
                .resolve_remote_tool_name(full_name)
                .map(|name| name.to_string())
            {
                return Some(client.call_tool(&tool_name, args));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_session_id() {
        // Test with flag-value pair
        let client_flag = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &["--session".to_string(), "my_test_session_token_123".to_string()],
            30000,
        );
        assert_eq!(client_flag.find_session_id(), Some("my_test_session_token_123".to_string()));

        // Test with inline flag parameter
        let client_inline = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &["--session=my_inline_session_token_456".to_string()],
            30000,
        );
        assert_eq!(client_inline.find_session_id(), Some("my_inline_session_token_456".to_string()));

        // Test with direct token argument
        let client_direct = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &["my_raw_direct_token_value_789".to_string()],
            30000,
        );
        assert_eq!(client_direct.find_session_id(), Some("my_raw_direct_token_value_789".to_string()));

        // Test with direct token argument when URL is also in args
        let client_direct_with_url = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &["https://mcp.zepto.co.in/mcp".to_string(), "token_directly_appended_abc".to_string()],
            30000,
        );
        assert_eq!(client_direct_with_url.find_session_id(), Some("token_directly_appended_abc".to_string()));

        // Test with no token
        let client_empty = McpClient::new(
            "zepto",
            "https://mcp.zepto.co.in/mcp",
            &[],
            30000,
        );
        assert_eq!(client_empty.find_session_id(), None);
    }
}
