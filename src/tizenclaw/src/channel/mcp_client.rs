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
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
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

/// A single MCP client connected to a server process.
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
}

impl McpClient {
    pub fn new(server_name: &str, command: &str, args: &[String], timeout_ms: u64) -> Self {
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
        }
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

    /// Spawn the server process and perform the MCP handshake.
    pub fn connect(&mut self) -> bool {
        if self.connected {
            return true;
        }
        self.update_last_used();

        let mut child = match Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("MCP Client: Failed to spawn '{}': {}", self.command, e);
                return false;
            }
        };

        let pid = child.id();
        let stdout = child.stdout.take().unwrap();
        let stdin = child.stdin.take().unwrap();

        self.reader = Some(Mutex::new(BufReader::new(stdout)));
        self.writer = Some(Mutex::new(stdin));
        self.child = Some(child);
        self.connected = true;

        log::debug!("MCP Client: '{}' started (PID: {})", self.server_name, pid);

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

    fn resolve_remote_tool_name(&self, full_name: &str) -> Option<&str> {
        self.tool_infos
            .iter()
            .find(|tool| tool.safe_name == full_name || tool.legacy_name == full_name)
            .map(|tool| tool.original_name.as_str())
            .or_else(|| {
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

                let mcp_type = s["type"].as_str().unwrap_or("stdio");
                if mcp_type == "http" {
                    if let Some(url) = s["url"].as_str() {
                        command = "npx".to_string();
                        args = vec!["-y", "mcp-remote", url]
                            .into_iter()
                            .map(String::from)
                            .collect();
                    }
                }

                if name.is_empty() || command.is_empty() {
                    continue;
                }

                let mut client = McpClient::new(name, &command, &args, timeout);
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

                let mcp_type = s["type"].as_str().unwrap_or("stdio");
                if mcp_type == "http" {
                    if let Some(url) = s["url"].as_str() {
                        command = "npx".to_string();
                        args = vec!["-y", "mcp-remote", url]
                            .into_iter()
                            .map(String::from)
                            .collect();
                    }
                }

                if name.is_empty() || command.is_empty() {
                    continue;
                }

                let mut client = McpClient::new(&name, &command, &args, timeout);
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
