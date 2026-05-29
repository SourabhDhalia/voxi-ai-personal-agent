# Design Document — TV Channel & System Prompt Admin Editor

This document details the architecture and design changes to register and launch the new `tv` channel (running on port 9092) and to expose the system prompt configuration file (`system_prompt.txt`) directly in the Admin Panel editor.

## 1. TV Channel Daemon Integration

### Default Channel Registration (`main.rs`)
In `src/voxi/src/main.rs`, we need to check if the `tv` channel is registered in the channel registry. If not, we will register it with default settings (port 9092, auto_start = true, and localhost_only = false):
```rust
if !reg.has_channel("tv") {
    let web_root = platform.paths.web_root.to_string_lossy().to_string();
    let tv_config = channel::ChannelConfig {
        name: "tv".into(),
        channel_type: "tv".into(),
        enabled: true,
        settings: serde_json::json!({
            "port": 9092,
            "localhost_only": false,
            "web_root": web_root
        }),
    };
    if let Some(ch) =
        channel::channel_factory::create_channel(&tv_config, Some(agent.clone()))
    {
        reg.register(ch, true);
        log::info!(
            "[Boot] TvChannel registered (port 9092, auto_start=true)"
        );
    }
}
```

## 2. Standalone Dashboard CLI and Outbound Queue Separation

### `--name` CLI Parameter
In `src/voxi-web-dashboard/src/main.rs`, we will introduce a `--name <NAME>` parameter. This will default to `"web_dashboard"`.
```rust
let mut channel_name = "web_dashboard".to_string();
...
"--name" if i + 1 < args.len() => {
    channel_name = args[i + 1].clone();
    i += 2;
}
```

### AppState Injection
We will add `channel_name: String` to `AppState` to dynamically resolve:
1. The outbound queue file path: `outbound/<channel_name>.jsonl` (e.g. `outbound/tv.jsonl` when name is `"tv"`).
2. The session ID prefix: `<channel_name>` (e.g. `tv_1779653216_0` instead of `web_1779653216_0`).

```rust
struct AppState {
    web_root: PathBuf,
    config_dir: PathBuf,
    data_dir: PathBuf,
    channel_name: String,
    admin_pw_hash: Arc<Mutex<String>>,
    active_tokens: Arc<Mutex<HashSet<String>>>,
    bridge_rate: Arc<Mutex<HashMap<String, Vec<u64>>>>,
}
```

### Outbound Path Resolution
Update `outbound_queue_path` and `load_outbound_messages` to accept and use the channel name:
```rust
fn outbound_queue_path(data_dir: &std::path::Path, channel_name: &str) -> PathBuf {
    let filename = format!("{}.jsonl", channel_name);
    data_dir.join("outbound").join(filename)
}
```

## 3. System Prompt Editor in Admin Panel

### Config Allowlist Expansion
We will add `"system_prompt.txt"` to `ALLOWED_CONFIGS` in `src/voxi-web-dashboard/src/main.rs`:
```rust
const ALLOWED_CONFIGS: &[&str] = &[
    "llm_config.json",
    "mcp_servers.json",
    "telegram_config.json",
    "slack_config.json",
    "discord_config.json",
    "webhook_config.json",
    "tool_policy.json",
    "agent_roles.json",
    "tunnel_config.json",
    "web_search_config.json",
    "system_prompt.txt",
];
```

### Frontend Card Metadata
We will update `CONFIG_LABELS` and `CONFIG_DESCRIPTIONS` in `data/web/app.js` to render a clean, human-readable card:
```javascript
CONFIG_LABELS['system_prompt.txt'] = 'System Prompt';
CONFIG_DESCRIPTIONS['system_prompt.txt'] = 'Edit the core system instructions and behavioral constraints of the agent.';
```
The raw/text modal editor handles files that are not valid JSON objects automatically, letting the user edit the plain text prompt file safely.
