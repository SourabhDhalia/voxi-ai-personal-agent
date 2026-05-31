//! voxi-cli: CLI tool for interacting with Voxi daemon.
//!
//! Usage:
//!   voxi-cli "What is the battery level?"
//!   voxi-cli -s my_session "Run a skill"
//!   voxi-cli --stream "Tell me about something"
//!   voxi-cli dashboard start
//!   voxi-cli dashboard start --port 9091
//!   voxi-cli dashboard stop
//!   voxi-cli dashboard status
//!   voxi-cli   (interactive mode)

use serde_json::{Map, Value, json};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use voxi::api::Voxi;

static CLI_SESSION_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn create_client() -> Result<Voxi, String> {
    let mut client = Voxi::new();
    client.initialize()?;
    Ok(client)
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

fn print_error_and_exit(error: &str) -> ! {
    eprintln!("Error: {}", error);
    std::process::exit(1);
}

fn generate_session_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = CLI_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cli_{}_{}", ts, seq)
}

fn parse_usage_baseline(raw: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|err| format!("Invalid usage baseline JSON: {}", err))
}

fn setup_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("VOXI_DATA_DIR") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".voxi")
}

fn setup_config_dir() -> PathBuf {
    setup_data_dir().join("config")
}

fn channel_config_path() -> PathBuf {
    setup_config_dir().join("channel_config.json")
}

fn default_dashboard_port() -> u16 {
    9091
}

fn dashboard_port_from_doc(doc: &Value) -> u16 {
    doc.get("channels")
        .and_then(Value::as_array)
        .and_then(|channels| {
            channels.iter().find_map(|channel| {
                if channel.get("name").and_then(Value::as_str) == Some("web_dashboard") {
                    channel
                        .get("settings")
                        .and_then(|settings| settings.get("port"))
                        .and_then(Value::as_u64)
                        .and_then(|port| u16::try_from(port).ok())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(default_dashboard_port)
}

fn dashboard_url() -> String {
    let port = fs::read_to_string(channel_config_path())
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .map(|doc| dashboard_port_from_doc(&doc))
        .unwrap_or_else(default_dashboard_port);
    format!("http://localhost:{}", port)
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("object just initialized")
}

fn set_path_value(doc: &mut Value, path: &[&str], new_value: Value) {
    let mut cursor = doc;
    for part in &path[..path.len().saturating_sub(1)] {
        let object = ensure_object(cursor);
        cursor = object
            .entry((*part).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    let object = ensure_object(cursor);
    object.insert(path[path.len() - 1].to_string(), new_value);
}

fn default_llm_config() -> Value {
    json!({
        "active_backend": "gemini",
        "fallback_backends": ["anthropic", "openai", "ollama"],
        "benchmark": {
            "pinchbench": {
                "actual_tokens": {
                    "prompt": 0,
                    "completion": 0,
                    "total": 0
                },
                "target": {
                    "score": 0.8,
                    "summary": "Match the target PinchBench run.",
                    "suite": "all"
                }
            }
        },
        "backends": {
            "gemini": {
                "api_key": "",
                "model": "gemini-2.5-flash",
                "temperature": 0.7,
                "max_tokens": 4096
            },
            "openai": {
                "api_key": "",
                "model": "gpt-4o",
                "endpoint": "https://api.openai.com/v1"
            },
            "anthropic": {
                "api_key": "",
                "model": "claude-sonnet-4-20250514",
                "endpoint": "https://api.anthropic.com/v1",
                "temperature": 0.7,
                "max_tokens": 4096
            },
            "xai": {
                "api_key": "",
                "model": "grok-3",
                "endpoint": "https://api.x.ai/v1"
            },
            "ollama": {
                "model": "llama3",
                "endpoint": "http://localhost:11434"
            }
        },
        "features": {
            "image_generation": {
                "provider": "openai",
                "api_key": "",
                "model": "gpt-image-1",
                "endpoint": "https://api.openai.com/v1",
                "size": "1024x1024",
                "background": "auto"
            }
        }
    })
}

fn default_telegram_config() -> Value {
    let default_workdir = std::env::current_dir()
        .unwrap_or_else(|_| setup_data_dir())
        .display()
        .to_string();
    json!({
        "bot_token": "",
        "allowed_chat_ids": [],
        "cli_workdir": default_workdir,
        "cli_backends": {}
    })
}

fn load_json_or_default(path: &Path, default_value: Value) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .unwrap_or(default_value)
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create config directory '{}': {}",
                parent.display(),
                err
            )
        })?;
    }
    let serialized = serde_json::to_string_pretty(value)
        .map_err(|err| format!("Failed to serialize JSON for '{}': {}", path.display(), err))?;
    fs::write(path, serialized)
        .map_err(|err| format!("Failed to write '{}': {}", path.display(), err))
}

fn prompt_line(prompt: &str) -> Result<String, String> {
    print!("{}", prompt);
    io::stdout()
        .flush()
        .map_err(|err| format!("Failed to flush stdout: {}", err))?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|err| format!("Failed to read user input: {}", err))?;
    Ok(line.trim().to_string())
}

fn prompt_with_default(prompt: &str, default: Option<&str>) -> Result<String, String> {
    let prompt_text = match default {
        Some(value) if !value.is_empty() => format!("{} [{}]: ", prompt, value),
        _ => format!("{}: ", prompt),
    };
    let value = prompt_line(&prompt_text)?;
    if value.is_empty() {
        Ok(default.unwrap_or("").to_string())
    } else {
        Ok(value)
    }
}

fn prompt_secret(prompt: &str, has_existing: bool) -> Result<Option<String>, String> {
    let suffix = if has_existing {
        " [press Enter to keep the saved value]"
    } else {
        " [press Enter to skip for now]"
    };
    let value = prompt_line(&format!("{}{}: ", prompt, suffix))?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn prompt_choice(prompt: &str, options: &[&str], default_index: usize) -> Result<usize, String> {
    println!("\n{}", prompt);
    for (index, option) in options.iter().enumerate() {
        println!("  {}. {}", index + 1, option);
    }

    loop {
        let default_value = (default_index + 1).to_string();
        let raw = prompt_with_default("Select an option", Some(&default_value))?;
        match raw.parse::<usize>() {
            Ok(value) if value >= 1 && value <= options.len() => return Ok(value - 1),
            _ => println!("Please enter a number between 1 and {}.", options.len()),
        }
    }
}

fn parse_chat_ids(raw: &str) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    for token in raw.split(',').map(str::trim).filter(|part| !part.is_empty()) {
        let value = token
            .parse::<i64>()
            .map_err(|_| format!("Invalid chat id '{}'", token))?;
        ids.push(value);
    }
    Ok(ids)
}

fn find_in_path(candidates: &[&str]) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for candidate in candidates {
            let candidate_path = dir.join(candidate);
            if candidate_path.is_file() {
                return Some(candidate_path);
            }
        }
    }
    None
}

fn detect_backend_path(backend: &str) -> Option<String> {
    let candidate_lists: &[&[&str]] = match backend {
        "codex" => &[&["codex"]],
        "gemini" => &[&["gemini"], &["/snap/bin/gemini"]],
        "claude" => &[&["claude"], &["claude-code"]],
        _ => return None,
    };

    for candidates in candidate_lists {
        if candidates.len() == 1 && candidates[0].starts_with('/') {
            let path = Path::new(candidates[0]);
            if path.is_file() {
                return Some(path.display().to_string());
            }
            continue;
        }
        if let Some(path) = find_in_path(candidates) {
            return Some(path.display().to_string());
        }
    }
    None
}

fn detected_cli_backends() -> Map<String, Value> {
    let mut map = Map::new();
    for backend in ["codex", "gemini", "claude"] {
        if let Some(path) = detect_backend_path(backend) {
            map.insert(backend.to_string(), Value::String(path));
        }
    }
    map
}

fn configure_llm(doc: &mut Value) -> Result<(), String> {
    let current_backend = doc
        .get("active_backend")
        .and_then(Value::as_str)
        .unwrap_or("gemini");
    let backends = ["gemini", "openai", "anthropic", "xai", "ollama"];
    let default_index = backends
        .iter()
        .position(|backend| *backend == current_backend)
        .unwrap_or(0);
    let labels = [
        "Gemini",
        "OpenAI",
        "Anthropic (Claude API)",
        "xAI",
        "Ollama",
    ];
    let choice = prompt_choice("Choose an LLM backend to configure", &labels, default_index)?;
    let backend = backends[choice];

    set_path_value(doc, &["active_backend"], Value::String(backend.to_string()));

    let backend_model_path = ["backends", backend, "model"];
    let current_model = doc
        .get("backends")
        .and_then(|value| value.get(backend))
        .and_then(|value| value.get("model"))
        .and_then(Value::as_str)
        .unwrap_or(match backend {
            "gemini" => "gemini-2.5-flash",
            "openai" => "gpt-4o",
            "anthropic" => "claude-sonnet-4-20250514",
            "xai" => "grok-3",
            "ollama" => "llama3",
            _ => "",
        });
    let model = prompt_with_default("Model name", Some(current_model))?;
    set_path_value(doc, &backend_model_path, Value::String(model));

    match backend {
        "ollama" => {
            let current_endpoint = doc
                .get("backends")
                .and_then(|value| value.get("ollama"))
                .and_then(|value| value.get("endpoint"))
                .and_then(Value::as_str)
                .unwrap_or("http://localhost:11434");
            let endpoint = prompt_with_default("Ollama endpoint", Some(current_endpoint))?;
            set_path_value(
                doc,
                &["backends", "ollama", "endpoint"],
                Value::String(endpoint),
            );
        }
        _ => {
            let has_existing_key = doc
                .get("backends")
                .and_then(|value| value.get(backend))
                .and_then(|value| value.get("api_key"))
                .and_then(Value::as_str)
                .map(|value| !value.is_empty())
                .unwrap_or(false);
            if let Some(api_key) = prompt_secret("API key", has_existing_key)? {
                set_path_value(
                    doc,
                    &["backends", backend, "api_key"],
                    Value::String(api_key),
                );
            }

            if backend == "openai" || backend == "anthropic" || backend == "xai" {
                let current_endpoint = doc
                    .get("backends")
                    .and_then(|value| value.get(backend))
                    .and_then(|value| value.get("endpoint"))
                    .and_then(Value::as_str)
                    .unwrap_or(match backend {
                        "openai" => "https://api.openai.com/v1",
                        "anthropic" => "https://api.anthropic.com/v1",
                        "xai" => "https://api.x.ai/v1",
                        _ => "",
                    });
                let endpoint = prompt_with_default("API endpoint", Some(current_endpoint))?;
                set_path_value(
                    doc,
                    &["backends", backend, "endpoint"],
                    Value::String(endpoint),
                );
            }
        }
    }

    Ok(())
}

fn print_botfather_guide() {
    println!("\nTelegram setup guide:");
    println!("  1. Open Telegram and search for @BotFather.");
    println!("  2. Run /newbot and follow the prompts.");
    println!("  3. Copy the bot token that BotFather gives you.");
    println!("  4. Send at least one message to your bot from the account you want to use.");
    println!("  5. Optionally restrict access with allowed_chat_ids after you know your chat id.");
}

fn configure_telegram(doc: &mut Value) -> Result<bool, String> {
    print_botfather_guide();

    let has_existing_token = doc
        .get("bot_token")
        .and_then(Value::as_str)
        .map(|value| !value.is_empty() && value != "YOUR_TELEGRAM_BOT_TOKEN_HERE")
        .unwrap_or(false);
    if let Some(token) = prompt_secret("Telegram bot token", has_existing_token)? {
        doc["bot_token"] = Value::String(token);
    }

    let token = doc
        .get("bot_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if token.is_empty() || token == "YOUR_TELEGRAM_BOT_TOKEN_HERE" {
        println!("Telegram setup skipped because no bot token was provided.");
        return Ok(false);
    }

    let existing_ids = doc
        .get("allowed_chat_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let allowlist_default = if existing_ids.is_empty() { 0 } else { 1 };
    let allowlist_choice = prompt_choice(
        "How should Telegram access be handled?",
        &[
            "Keep it open for now (empty allowlist, easier for first-time testing)",
            "Enter allowed chat IDs now",
        ],
        allowlist_default,
    )?;
    if allowlist_choice == 0 {
        doc["allowed_chat_ids"] = Value::Array(vec![]);
        println!("Note: an empty allowlist means any chat that reaches the bot can talk to it.");
    } else {
        let existing = existing_ids
            .iter()
            .filter_map(Value::as_i64)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let prompt_default = if existing.is_empty() {
            None
        } else {
            Some(existing.as_str())
        };
        loop {
            let raw = prompt_with_default("Comma-separated allowed chat IDs", prompt_default)?;
            match parse_chat_ids(&raw) {
                Ok(ids) => {
                    doc["allowed_chat_ids"] = Value::Array(
                        ids.into_iter().map(|id| Value::Number(id.into())).collect(),
                    );
                    break;
                }
                Err(err) => println!("{}", err),
            }
        }
    }

    let current_workdir = doc
        .get("cli_workdir")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| setup_data_dir())
                .display()
                .to_string()
        });
    let cli_workdir = prompt_with_default(
        "Default project directory for Telegram coding mode",
        Some(&current_workdir),
    )?;
    doc["cli_workdir"] = Value::String(cli_workdir);

    let detected = detected_cli_backends();
    if !detected.is_empty() {
        println!("\nDetected coding-agent CLIs:");
        for (name, value) in &detected {
            if let Some(path) = value.as_str() {
                println!("  - {}: {}", name, path);
            }
        }
    } else {
        println!("\nNo coding-agent CLI binaries were auto-detected in PATH.");
    }

    let existing_paths = doc
        .get("cli_backends")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let backend_path_choice = prompt_choice(
        "How should Telegram CLI backend paths be configured?",
        &[
            "Use detected paths where available",
            "Review and edit paths now",
            "Keep the existing file values",
        ],
        if existing_paths.is_empty() { 0 } else { 2 },
    )?;

    match backend_path_choice {
        0 => {
            let merged = if detected.is_empty() {
                existing_paths
            } else {
                detected
            };
            doc["cli_backends"] = Value::Object(merged);
        }
        1 => {
            let mut manual = Map::new();
            for backend in ["codex", "gemini", "claude"] {
                let fallback = existing_paths
                    .get(backend)
                    .and_then(Value::as_str)
                    .or_else(|| detected.get(backend).and_then(Value::as_str));
                let value = prompt_with_default(
                    &format!("Path for the {} CLI binary", backend),
                    fallback,
                )?;
                if !value.trim().is_empty() {
                    manual.insert(backend.to_string(), Value::String(value));
                }
            }
            doc["cli_backends"] = Value::Object(manual);
        }
        _ => {
            if doc.get("cli_backends").is_none() {
                doc["cli_backends"] = Value::Object(existing_paths);
            }
        }
    }

    Ok(true)
}

fn print_setup_summary(config_dir: &Path, configured_now: bool) {
    println!("\nSetup summary:");
    println!("  Dashboard: {}", dashboard_url());
    println!("  Config directory: {}", config_dir.display());
    println!("  Open the dashboard in your browser with the URL above.");
    println!("  Start the dashboard manually: voxi-cli dashboard start");
    println!("  Dashboard status command: voxi-cli dashboard status");
    if configured_now {
        println!("  To rerun setup later: voxi-cli setup");
        println!("  Telegram changes need a daemon restart to become active.");
    } else {
        println!("  Setup was postponed. You can continue with the dashboard now.");
        println!("  To configure later: voxi-cli setup");
    }
}

fn cmd_setup() {
    let config_dir = setup_config_dir();
    let llm_path = config_dir.join("llm_config.json");
    let telegram_path = config_dir.join("telegram_config.json");

    println!("Voxi setup wizard");
    println!("This wizard prepares host-side LLM and Telegram settings.");

    let start_choice = prompt_choice(
        "How would you like to continue?",
        &[
            "Configure now",
            "Configure later and use the dashboard first",
        ],
        0,
    )
    .unwrap_or_else(|err| print_error_and_exit(&err));

    if start_choice == 1 {
        print_setup_summary(&config_dir, false);
        return;
    }

    let mut llm_doc = load_json_or_default(&llm_path, default_llm_config());
    configure_llm(&mut llm_doc).unwrap_or_else(|err| print_error_and_exit(&err));
    write_pretty_json(&llm_path, &llm_doc).unwrap_or_else(|err| print_error_and_exit(&err));

    let telegram_choice = prompt_choice(
        "Do you want to configure Telegram coding mode now?",
        &[
            "Yes, configure Telegram now",
            "No, I will set up Telegram later",
        ],
        1,
    )
    .unwrap_or_else(|err| print_error_and_exit(&err));

    if telegram_choice == 0 {
        let mut telegram_doc = load_json_or_default(&telegram_path, default_telegram_config());
        if configure_telegram(&mut telegram_doc).unwrap_or_else(|err| print_error_and_exit(&err)) {
            write_pretty_json(&telegram_path, &telegram_doc)
                .unwrap_or_else(|err| print_error_and_exit(&err));
        }
    }

    print_setup_summary(&config_dir, true);
}

fn show_usage(client: &Voxi, session_id: Option<&str>, baseline: Option<&Value>) {
    match client.get_usage(session_id, baseline) {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn send_prompt(
    client: &Voxi,
    session_id: &str,
    prompt: &str,
    stream: bool,
) -> Result<String, String> {
    let response = if stream {
        client.process_prompt_streaming(session_id, prompt, |chunk| {
            print!("{}", chunk);
            io::stdout().flush().ok();
        })?
    } else {
        let text = client.process_prompt(session_id, prompt)?;
        voxi::api::PromptResponse {
            session_id: session_id.to_string(),
            text,
            stream_received: false,
        }
    };

    if !response.stream_received {
        println!("{}", response.text);
    } else {
        println!();
    }

    Ok(response.session_id)
}

fn parse_dashboard_command(input: &str) -> (String, Option<u16>) {
    let mut parts = input.split_whitespace();
    let action = parts.next().unwrap_or("").to_string();
    let mut port = None;

    while let Some(part) = parts.next() {
        if part == "--port" {
            let value = parts.next().unwrap_or("");
            match value.parse::<u16>() {
                Ok(parsed) if parsed > 0 => port = Some(parsed),
                _ => {
                    eprintln!("Error: invalid dashboard port '{}'", value);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!(
                "Unknown dashboard option '{}'. Use: start [--port N] | stop | status",
                part
            );
            std::process::exit(1);
        }
    }

    (action, port)
}

/// Handle `voxi-cli dashboard <action> [--port N]`.
fn cmd_dashboard(client: &Voxi, command: &str) {
    let (action, port) = parse_dashboard_command(command);

    match action.as_str() {
        "start" => match client.start_dashboard(port) {
            Ok(_) => {
                if let Some(port) = port {
                    println!("Dashboard started on port {}.", port);
                } else {
                    println!("Dashboard started.");
                }
            }
            Err(error) => print_error_and_exit(&error),
        },
        "stop" => match client.stop_dashboard() {
            Ok(_) => println!("Dashboard stopped."),
            Err(error) => print_error_and_exit(&error),
        },
        "status" => match client.dashboard_status() {
            Ok(result) => {
                let running = result
                    .get("running")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                println!("Dashboard: {}", if running { "running" } else { "stopped" });
            }
            Err(error) => print_error_and_exit(&error),
        },
        _ => {
            eprintln!(
                "Unknown dashboard action '{}'. Use: start [--port N] | stop | status",
                action
            );
            std::process::exit(1);
        }
    }
}

// ─── ANSI colour helpers ─────────────────────────────────────────────────────

fn ansi(code: &str, text: &str) -> String {
    if is_tty() {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

fn is_tty() -> bool {
    // Simple heuristic: check if stdout is a terminal via libc
    // Falls back to true so colours show by default.
    #[cfg(unix)]
    {
        unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn dim(text: &str) -> String { ansi("2", text) }
fn bold(text: &str) -> String { ansi("1", text) }
fn green(text: &str) -> String { ansi("32", text) }
fn cyan(text: &str) -> String { ansi("36", text) }
fn yellow(text: &str) -> String { ansi("33", text) }
fn magenta(text: &str) -> String { ansi("35", text) }

// ─── Chat REPL ───────────────────────────────────────────────────────────────

fn print_chat_help() {
    println!("{}", bold("Voxi Chat — available commands:"));
    println!("  {}   Start a brand-new conversation", cyan("/new"));
    println!("  {}   List recent sessions", cyan("/sessions"));
    println!("  {} {}  Switch to a previous session", cyan("/switch"), dim("<id>"));
    println!("  {}      Show current session ID", cyan("/session"));
    println!("  {}        Show token usage", cyan("/usage"));
    println!("  {}         Show this help", cyan("/help"));
    println!("  {} {}        Start/stop web dashboard", cyan("/dashboard"), dim("<start|stop|status>"));
    println!("  {}   Exit chat", cyan("/exit  quit  exit"));
    println!();
    println!("  {} {}   Send prompt with streaming (default on)", dim("--no-stream"), dim("flag"));
    println!("  {}   Anything else is sent as a message to the agent", dim("<message>"));
}

fn list_sessions_compact(client: &Voxi) {
    match client.list_sessions() {
        Ok(sessions) => {
            if sessions.is_empty() {
                println!("{}", dim("No saved sessions found."));
                return;
            }
            println!("{}", bold("Recent sessions:"));
            for s in sessions.iter().take(20) {
                let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)");
                let modified = s.get("modified").and_then(|v| v.as_i64())
                    .map(|ts| {
                        // Format as HH:MM or date depending on age
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        let age = now - ts;
                        if age < 86400 {
                            format!("{}h ago", age / 3600)
                        } else {
                            format!("{}d ago", age / 86400)
                        }
                    })
                    .unwrap_or_default();
                let msgs = s.get("message_count").and_then(|v| v.as_i64()).unwrap_or(0);
                println!("  {} {} {} {}",
                    cyan(id),
                    dim(&format!("{} msgs", msgs)),
                    dim(&modified),
                    dim(title)
                );
            }
        }
        Err(e) => eprintln!("{} {}", yellow("[sessions error]"), e),
    }
}

/// Full-featured interactive chat REPL.
fn cmd_chat(client: &Voxi, start_session: Option<String>, stream: bool) {
    let mut session_id = start_session.unwrap_or_else(generate_session_id);

    // Banner
    println!();
    println!("  {} {}", bold(&green("Voxi")), bold("Chat"));
    println!("  {}", dim("Type /help for commands. Press Ctrl-C or type /exit to quit."));
    println!("  Session: {}", cyan(&session_id));
    println!();

    let stdin = io::stdin();
    loop {
        // Prompt
        print!("{}  ", bold(&magenta("you›")));
        io::stdout().flush().ok();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,  // EOF / Ctrl-D
            Err(_) => break,
            Ok(_) => {}
        }
        let input = line.trim();
        if input.is_empty() { continue; }

        match input {
            "exit" | "quit" | "/exit" | "/quit" => {
                println!("{}", dim("Goodbye."));
                break;
            }
            "/help" => print_chat_help(),

            "/new" => {
                session_id = generate_session_id();
                println!("{} {}", green("✓ New session:"), cyan(&session_id));
            }

            "/session" => {
                println!("Session: {}", cyan(&session_id));
            }

            "/sessions" => list_sessions_compact(client),

            cmd if cmd.starts_with("/switch ") => {
                let new_id = cmd.trim_start_matches("/switch ").trim();
                if new_id.is_empty() {
                    println!("{}", yellow("Usage: /switch <session-id>"));
                } else {
                    session_id = new_id.to_string();
                    println!("{} {}", green("✓ Switched to:"), cyan(&session_id));
                }
            }

            "/usage" => show_usage(client, Some(&session_id), None),

            cmd if cmd.starts_with("/dashboard ") => {
                let action = cmd.trim_start_matches("/dashboard ").trim();
                cmd_dashboard(client, action);
            }

            prompt => {
                // Print the agent label before streaming starts
                print!("\n{}  ", bold(&cyan("voxi›")));
                io::stdout().flush().ok();

                let result = if stream {
                    client.process_prompt_streaming(&session_id, prompt, |chunk| {
                        print!("{}", chunk);
                        io::stdout().flush().ok();
                    })
                } else {
                    let text_result = client.process_prompt(&session_id, prompt);
                    text_result.map(|text| {
                        print!("{}", text);
                        voxi::api::PromptResponse {
                            session_id: session_id.clone(),
                            text,
                            stream_received: false,
                        }
                    })
                };

                match result {
                    Ok(resp) => {
                        println!("\n");
                        // Keep session_id in sync (agent may have assigned one)
                        if !resp.session_id.is_empty() {
                            session_id = resp.session_id;
                        }
                    }
                    Err(e) => {
                        println!();
                        eprintln!("{} {}", yellow("[error]"), e);
                    }
                }
            }
        }
    }
}

/// Legacy interactive REPL mode (kept for --no-chat compat, used by bare `voxi-cli`).
fn interactive_mode(client: &Voxi, explicit_session_id: Option<&str>, stream: bool) {
    match explicit_session_id {
        Some(session_id) => println!("Voxi Interactive CLI (session: {})", session_id),
        None => println!("Voxi Interactive CLI (new session per prompt)"),
    }
    println!("Type 'quit' or 'exit' to leave. Type '/help' for commands.\n");

    let stdin = io::stdin();
    loop {
        print!("voxi> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "quit" | "exit" => break,
            "/help" => {
                println!("Commands:");
                println!("  /usage            Show token usage");
                println!("  /dashboard start [--port N] Start web dashboard");
                println!("  /dashboard stop   Stop web dashboard");
                println!("  /dashboard status Show dashboard status");
                println!("  /chat             Enter full chat mode (persistent session)");
                println!("  -s <id>           Re-run CLI with a fixed session");
                println!("  quit, exit        Exit");
                println!("  <text>            Send prompt");
            }
            "/chat" => {
                cmd_chat(client, None, stream);
                return;
            }
            cmd if cmd.starts_with("/usage") => {
                show_usage(client, explicit_session_id, None);
            }
            cmd if cmd.starts_with("/dashboard ") => {
                let action = cmd.trim_start_matches("/dashboard ").trim();
                cmd_dashboard(client, action);
            }
            prompt => {
                let session_id = explicit_session_id
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(generate_session_id);
                if let Err(error) = send_prompt(client, &session_id, prompt, stream) {
                    eprintln!("Error: {}", error);
                }
            }
        }
    }
}

fn cmd_config_get(client: &Voxi, path: Option<&str>) {
    match client.get_llm_config(path) {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn cmd_config_set(client: &Voxi, path: &str, raw_value: &str, strict_json: bool) {
    let value = if strict_json {
        match serde_json::from_str::<Value>(raw_value) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Error: invalid JSON value: {}", err);
                std::process::exit(1);
            }
        }
    } else {
        Value::String(raw_value.to_string())
    };

    match client.set_llm_config(path, value) {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn cmd_config_unset(client: &Voxi, path: &str) {
    match client.unset_llm_config(path) {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn cmd_config_reload(client: &Voxi) {
    match client.reload_llm_backends() {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn cmd_config(client: &Voxi, args: &[String]) {
    match args.first().map(String::as_str) {
        Some("get") => {
            cmd_config_get(client, args.get(1).map(String::as_str));
        }
        Some("set") => {
            if args.len() < 3 {
                eprintln!("Usage: voxi-cli config set <path> <value> [--strict-json]");
                std::process::exit(1);
            }
            let strict_json = args[3..]
                .iter()
                .any(|arg| arg == "--strict-json" || arg == "--json");
            cmd_config_set(client, &args[1], &args[2], strict_json);
        }
        Some("unset") => {
            if args.len() < 2 {
                eprintln!("Usage: voxi-cli config unset <path>");
                std::process::exit(1);
            }
            cmd_config_unset(client, &args[1]);
        }
        Some("reload") => {
            cmd_config_reload(client);
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  voxi-cli config get [path]");
            eprintln!("  voxi-cli config set <path> <value> [--strict-json]");
            eprintln!("  voxi-cli config unset <path>");
            eprintln!("  voxi-cli config reload");
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("voxi-cli — Voxi CLI\n");
    eprintln!("Usage:");
    eprintln!("  voxi-cli [options] [prompt]\n");
    eprintln!("Chat commands:");
    eprintln!("  voxi-cli chat              Start persistent interactive chat");
    eprintln!("  voxi-cli chat -s <id>      Resume a specific session\n");
    eprintln!("Options:");
    eprintln!("  -s <id>           Reuse a fixed session ID");
    eprintln!("  --no-stream       Disable real-time streaming");
    eprintln!("  --usage           Show token usage");
    eprintln!("  --usage-baseline  JSON baseline for usage delta");
    eprintln!("  -h, --help        Show this help\n");
    eprintln!("Dashboard commands:");
    eprintln!("  voxi-cli dashboard start [--port N]");
    eprintln!("                                   Start the web dashboard");
    eprintln!("  voxi-cli dashboard stop    Stop the web dashboard");
    eprintln!("  voxi-cli dashboard status  Show dashboard status\n");
    eprintln!("Registration commands:");
    eprintln!("  voxi-cli register tool <path>");
    eprintln!("  voxi-cli register skill <path>");
    eprintln!("  voxi-cli unregister tool <path>");
    eprintln!("  voxi-cli unregister skill <path>");
    eprintln!("  voxi-cli list registrations\n");
    eprintln!("LLM config commands:");
    eprintln!("  voxi-cli config get [path]");
    eprintln!("  voxi-cli config set <path> <value> [--strict-json]");
    eprintln!("  voxi-cli config unset <path>");
    eprintln!("  voxi-cli config reload\n");
    eprintln!("Setup commands:");
    eprintln!("  voxi-cli setup         Interactive host setup wizard\n");
    eprintln!("Voice model commands:");
    eprintln!("  voxi-cli model list");
    eprintln!("  voxi-cli model install <model_id>");
    eprintln!("  voxi-cli model verify <model_id>");
    eprintln!("  voxi-cli model remove <model_id>");
    eprintln!("  voxi-cli model switch <task> <model_id>");
    eprintln!("  voxi-cli model doctor\n");
    eprintln!("If no prompt given, starts interactive mode.");
    eprintln!("Tip: use 'voxi-cli chat' for the full persistent chat experience.");
}

fn cmd_register(client: &Voxi, kind: &str, path: &str) {
    match client.register_path(kind, path) {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn cmd_unregister(client: &Voxi, kind: &str, path: &str) {
    match client.unregister_path(kind, path) {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

fn cmd_list_registrations(client: &Voxi) {
    match client.list_registered_paths() {
        Ok(result) => print_json(&result),
        Err(error) => print_error_and_exit(&error),
    }
}

/// Downloads model files by shelling out to `curl` (no unvendored HTTP crate
/// is available offline). Fails clearly when `curl` is missing.
struct CurlDownloader;

impl voxi_voice::model_store::Downloader for CurlDownloader {
    fn fetch(&self, url: &str, dest: &Path) -> Result<(), String> {
        let status = std::process::Command::new("curl")
            .arg("-fsSL")
            .arg("--create-dirs")
            .arg("-o")
            .arg(dest)
            .arg(url)
            .status()
            .map_err(|e| format!("failed to launch curl: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("curl exited with status {status}"))
        }
    }
}

/// Resolve the voice model directory (`~/.voxi/models/voice/` by default,
/// override with `VOXI_VOICE_MODEL_DIR`).
fn voice_model_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("VOXI_VOICE_MODEL_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    voxi_voice::config::default_model_dir()
}

/// Locate `models.voice.json`. Honors `VOXI_VOICE_REGISTRY`, then checks common
/// install/source locations.
fn voice_registry_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VOXI_VOICE_REGISTRY") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let candidates = [
        PathBuf::from("data/config/models.voice.json"),
        voice_model_dir().join("models.voice.json"),
        PathBuf::from("/usr/share/voxi/models.voice.json"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn load_voice_registry() -> voxi_voice::model_store::ModelRegistry {
    let path = voice_registry_path().unwrap_or_else(|| {
        print_error_and_exit(
            "voice model registry not found; set VOXI_VOICE_REGISTRY or place \
             data/config/models.voice.json",
        )
    });
    match voxi_voice::model_store::ModelRegistry::load(&path, voice_model_dir()) {
        Ok(r) => r,
        Err(e) => print_error_and_exit(&format!("failed to load registry {}: {e}", path.display())),
    }
}

fn cmd_model(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("list") => {
            let registry = load_voice_registry();
            for s in registry.list() {
                let mark = if s.installed { "[installed]" } else { "[       - ]" };
                println!(
                    "{mark} {:<28} task={:<12} backend={:<10} {} MB  {}",
                    s.entry.model_id,
                    s.entry.task,
                    s.entry.backend,
                    s.entry.size_mb,
                    s.entry.language
                );
            }
        }
        Some("install") => {
            let id = require_model_id(args, "install");
            let registry = load_voice_registry();
            println!("Installing '{id}'...");
            match registry.download(&id, &CurlDownloader) {
                Ok(report) => print_verify_report(&report),
                Err(e) => print_error_and_exit(&format!("install failed: {e}")),
            }
        }
        Some("verify") => {
            let id = require_model_id(args, "verify");
            let registry = load_voice_registry();
            match registry.verify(&id) {
                Ok(report) => print_verify_report(&report),
                Err(e) => print_error_and_exit(&format!("verify failed: {e}")),
            }
        }
        Some("remove") => {
            let id = require_model_id(args, "remove");
            let registry = load_voice_registry();
            match registry.remove(&id) {
                Ok(()) => println!("Removed '{id}'"),
                Err(e) => print_error_and_exit(&format!("remove failed: {e}")),
            }
        }
        Some("switch") => {
            if args.len() < 3 {
                print_error_and_exit("Usage: voxi-cli model switch <task> <model_id>");
            }
            let task = &args[1];
            let model_id = &args[2];
            let registry = load_voice_registry();
            if registry.find(model_id).is_none() {
                print_error_and_exit(&format!("unknown model id: {model_id}"));
            }
            persist_selection(task, model_id);
            println!("Selected '{model_id}' for task '{task}'");
        }
        Some("doctor") => cmd_model_doctor(&load_voice_registry()),
        _ => {
            eprintln!("Usage:");
            eprintln!("  voxi-cli model list");
            eprintln!("  voxi-cli model install <model_id>");
            eprintln!("  voxi-cli model verify <model_id>");
            eprintln!("  voxi-cli model remove <model_id>");
            eprintln!("  voxi-cli model switch <task> <model_id>");
            eprintln!("  voxi-cli model doctor");
            std::process::exit(1);
        }
    }
}

fn require_model_id(args: &[String], sub: &str) -> String {
    match args.get(1) {
        Some(id) if !id.is_empty() => id.clone(),
        _ => print_error_and_exit(&format!("Usage: voxi-cli model {sub} <model_id>")),
    }
}

fn print_verify_report(report: &voxi_voice::model_store::VerifyReport) {
    let checksum = match report.checksum_ok {
        Some(true) => "checksum=OK",
        Some(false) => "checksum=MISMATCH",
        None => "checksum=skipped",
    };
    println!(
        "{}: files_present={} {} — {}",
        report.model_id, report.files_present, checksum, report.detail
    );
}

/// Persist the selected model per task to `<model_dir>/selection.json`.
fn persist_selection(task: &str, model_id: &str) {
    let dir = voice_model_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        print_error_and_exit(&format!("cannot create model dir: {e}"));
    }
    let path = dir.join("selection.json");
    let mut map: Map<String, Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Map<String, Value>>(&s).ok())
        .unwrap_or_default();
    map.insert(task.to_string(), Value::String(model_id.to_string()));
    let body = serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default();
    if let Err(e) = std::fs::write(&path, body) {
        print_error_and_exit(&format!("cannot write selection: {e}"));
    }
}

fn cmd_model_doctor(registry: &voxi_voice::model_store::ModelRegistry) {
    println!("Voice model directory: {}", voice_model_dir().display());
    let mut installed = 0usize;
    let mut issues = 0usize;
    for status in registry.list() {
        if !status.installed {
            continue;
        }
        installed += 1;
        match registry.verify(&status.entry.model_id) {
            Ok(report) => {
                print_verify_report(&report);
                if report.checksum_ok == Some(false) || !report.files_present {
                    issues += 1;
                }
            }
            Err(e) => {
                println!("{}: ERROR {e}", status.entry.model_id);
                issues += 1;
            }
        }
    }
    let sel = voice_model_dir().join("selection.json");
    if sel.is_file() {
        println!("Active selection: {}", sel.display());
    } else {
        println!("Active selection: (none; using config defaults)");
    }
    println!("Doctor summary: {installed} installed, {issues} issue(s)");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut session_id: Option<String> = None;
    let mut explicit_session_id = false;
    let mut stream = true;
    let mut usage_requested = false;
    let mut usage_baseline: Option<Value> = None;
    let mut prompt_parts: Vec<String> = vec![];
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "-s" if i + 1 < args.len() => {
                i += 1;
                session_id = Some(args[i].clone());
                explicit_session_id = true;
            }
            "--no-stream" => stream = false,
            "--usage" => {
                usage_requested = true;
            }
            "--usage-baseline" if i + 1 < args.len() => {
                i += 1;
                usage_baseline = Some(parse_usage_baseline(&args[i]).unwrap_or_else(|err| {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }));
            }
            "--usage-baseline" => {
                eprintln!("Usage: voxi-cli --usage-baseline '<json>'");
                std::process::exit(1);
            }
            "dashboard" if i + 1 < args.len() => {
                let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));
                i += 1;
                let mut command = args[i].clone();
                i += 1;
                while i < args.len() {
                    command.push(' ');
                    command.push_str(&args[i]);
                    i += 1;
                }
                cmd_dashboard(&client, &command);
                return;
            }
            "register" if i + 2 < args.len() => {
                let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));
                cmd_register(&client, &args[i + 1], &args[i + 2]);
                return;
            }
            "unregister" if i + 2 < args.len() => {
                let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));
                cmd_unregister(&client, &args[i + 1], &args[i + 2]);
                return;
            }
            "list" if i + 1 < args.len() && args[i + 1] == "registrations" => {
                let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));
                cmd_list_registrations(&client);
                return;
            }
            "config" => {
                let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));
                cmd_config(&client, &args[i + 1..]);
                return;
            }
            "setup" => {
                cmd_setup();
                return;
            }
            "model" => {
                cmd_model(&args[i + 1..]);
                return;
            }
            "chat" => {
                // Collect remaining args to parse -s <id> and --no-stream
                let mut chat_session: Option<String> = session_id.clone();
                let mut chat_stream = stream;
                let mut j = i + 1;
                while j < args.len() {
                    match args[j].as_str() {
                        "-s" if j + 1 < args.len() => {
                            j += 1;
                            chat_session = Some(args[j].clone());
                        }
                        "--no-stream" => chat_stream = false,
                        _ => {}
                    }
                    j += 1;
                }
                let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));
                cmd_chat(&client, chat_session, chat_stream);
                return;
            }
            "dashboard" => {
                eprintln!("Usage: voxi-cli dashboard <start [--port N]|stop|status>");
                std::process::exit(1);
            }
            "register" => {
                eprintln!("Usage: voxi-cli register <tool|skill> <path>");
                std::process::exit(1);
            }
            "unregister" => {
                eprintln!("Usage: voxi-cli unregister <tool|skill> <path>");
                std::process::exit(1);
            }
            _ => {
                for arg in args.iter().skip(i) {
                    prompt_parts.push(arg.clone());
                }
                break;
            }
        }
        i += 1;
    }

    let client = create_client().unwrap_or_else(|err| print_error_and_exit(&err));

    if usage_requested {
        show_usage(&client, session_id.as_deref(), usage_baseline.as_ref());
        return;
    }

    let prompt = prompt_parts.join(" ");

    if !prompt.is_empty() {
        let resolved_session_id = session_id.unwrap_or_else(generate_session_id);
        if let Err(error) = send_prompt(&client, &resolved_session_id, &prompt, stream) {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    } else {
        let explicit = if explicit_session_id {
            session_id.as_deref()
        } else {
            None
        };
        interactive_mode(&client, explicit, stream);
    }
}

#[cfg(test)]
mod tests {
    use super::{dashboard_port_from_doc, parse_chat_ids};
    use serde_json::json;

    #[test]
    fn parse_chat_ids_accepts_comma_separated_ids() {
        assert_eq!(parse_chat_ids("123, 456,789").unwrap(), vec![123, 456, 789]);
    }

    #[test]
    fn parse_chat_ids_rejects_invalid_tokens() {
        assert!(parse_chat_ids("123, nope").is_err());
    }

    #[test]
    fn dashboard_port_from_doc_reads_web_dashboard_port() {
        let doc = json!({
            "channels": [
                {
                    "name": "web_dashboard",
                    "settings": {
                        "port": 9191
                    }
                }
            ]
        });
        assert_eq!(dashboard_port_from_doc(&doc), 9191);
    }
}
