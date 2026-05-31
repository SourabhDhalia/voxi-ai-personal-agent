# Voxi Usage Guide

## Purpose

This guide explains how to build, deploy, operate, and inspect Voxi using
the repository's supported workflow. It is written for developers and operators
who want to use the current Rust workspace as an embedded agent runtime rather
than treat it as a generic local Rust application.

## Operating Model

Voxi is designed to run as a long-lived daemon. The normal flow is:

1. build the Voxi packages through `deploy.sh`
2. deploy to an emulator or device
3. restart the service
4. interact through the CLI, dashboard, or configured channels

The repository workflow is intentionally deployment-oriented. For this project,
the important validation path is the target-oriented one, not a detached local
host loop.

## Prerequisites

You should have the following available before you begin:

- Voxi Studio tooling with `sdb`
- Voxi GBS build support
- a reachable Voxi emulator or physical device
- a shell environment that can run repository scripts

It also helps to know the target device serial if more than one emulator or
device is connected.

If your target is a Voxi TV / DTV device that uses `ssh` instead of `sdb`,
see [DTV_USAGE.md](DTV_USAGE.md) for the manual SSH-based deployment flow.

## Core Command: `deploy.sh`

The root `deploy.sh` script is the operational entry point for the
project. It works natively on **macOS**, **Ubuntu**, and **WSL**.

Common host-mode commands:

```bash
./deploy.sh                  # Release build + install + run
./deploy.sh -d               # Debug build + install + run
./deploy.sh -b               # Build only, do not install or run
./deploy.sh --test           # Run workspace tests (offline)
./deploy.sh --status         # Check daemon status
./deploy.sh --log            # Follow daemon logs
./deploy.sh -s               # Stop the running daemon
./deploy.sh --remove         # Stop and remove ~/.voxi install
./deploy.sh --restart-only   # Restart using already-installed binaries
./deploy.sh --dry-run        # Print commands without executing
```

> **Voxi DTV / armv7l note**: The GBS build and `sdb`-based device
> deployment workflow is a separate pipeline that requires Voxi Studio
> tooling. The host-mode `./deploy.sh` does not have `-a`/`--arch`
> flags and always targets the local host machine.

### What the script handles

- prerequisite checks
- architecture selection
- GBS build orchestration
- package deployment through `sdb`
- service restart steps

### Common flags

| Flag | Description |
|---|---|
| `-d, --debug` | Build in debug mode (default: release) |
| `-b, --build-only` | Build only, do not install or run |
| `--test` | Build + run workspace tests (offline) |
| `--restart-only` | Restart using installed binaries |
| `-s, --stop` | Stop the running daemon |
| `--remove` | Stop all processes and remove `~/.voxi` |
| `--status` | Show current daemon status |
| `--log` | Follow daemon log output |
| `--dry-run` | Print commands without executing |
| `--build-root <dir>` | Override host Cargo target directory |
| `--llm-config <path>` | Use a specific `llm_config.json` file |

## Standard Development Deployment Flow

For the normal host build-and-run:

```bash
./deploy.sh
```

For a debug build:

```bash
./deploy.sh -d
```

For a build-only pass without launching the daemon:

```bash
./deploy.sh -b
```

For Voxi DTV / emulator validation, use the separate GBS pipeline
(requires Voxi Studio tooling and a reachable target).

## Service Lifecycle

After deployment, Voxi runs as a device service. In practice, useful
checks usually include:

- verifying that the main daemon is active
- confirming the dashboard process is available
- checking that the tool executor socket is listening

The exact commands depend on your environment, but the project workflow and
internal notes regularly use device-side service inspection through `sdb shell`
plus `systemctl` or log inspection tools.

## Using the CLI

The CLI is the most direct operator surface for the daemon.

### Send a prompt

```bash
voxi-cli "What is the current system status?"
```

### Stream a response

```bash
voxi-cli --stream "Explain the active channels"
```

### Use interactive mode

```bash
voxi-cli
```

### Manage the dashboard channel

```bash
voxi-cli dashboard start
voxi-cli dashboard start --port 9091
voxi-cli dashboard stop
voxi-cli dashboard status
```

### Manage voice models

The voice channel is disabled by default and degrades to null STT/TTS when no
models are installed, so the daemon always boots. Use the `model` subcommand to
populate `data/config/models.voice.json`-listed models on demand:

```bash
voxi-cli model list                       # known models + install state
voxi-cli model install moonshine-tiny     # download via curl, then verify
voxi-cli model verify moonshine-tiny      # files present + checksum (if pinned)
voxi-cli model switch stt moonshine-tiny  # persist a per-task selection
voxi-cli model remove moonshine-tiny      # delete the installed model dir
voxi-cli model doctor                     # summarize installed models + issues
```

Override locations with `VOXI_VOICE_MODEL_DIR` (default
`~/.voxi/models/voice/`) and `VOXI_VOICE_REGISTRY` (path to
`models.voice.json`). Real on-device STT/TTS additionally requires building
`voxi-voice` with the `onnx` feature and a target-arch `libonnxruntime`.

## Using the Web Dashboard

The standalone dashboard binary serves both the UI and HTTP API.

Based on the current code:

- Host default port: **`9091`**

The dashboard binary accepts runtime options such as:

```bash
voxi-web-dashboard --port 9090
voxi-web-dashboard --web-root <path>
voxi-web-dashboard --config-dir <path>
voxi-web-dashboard --data-dir <path>
voxi-web-dashboard --localhost-only
```

In normal deployments the daemon or deployment flow is expected to manage the
dashboard lifecycle for you, but the flags are useful for debugging and custom
bring-up.

## Runtime Paths and Data

The codebase uses runtime path detection so the daemon can behave sensibly on
Voxi and non-Voxi environments.

Examples of what gets stored under runtime-managed directories include:

- logs
- sessions
- tasks
- outbound dashboard message queues
- web dashboard assets and app data

When debugging environment-specific issues, confirm which data and config
directories were resolved at startup.

## Configuration Touchpoints

The source tree and dashboard service indicate several configuration files and
surfaces, including:

- LLM configuration
- channel configuration
- tool policy configuration
- agent role configuration
- tunnel and web search configuration
- tool hooks configuration (`hooks.json`)
- skill toggle states configuration (`skills_state.json`)

Operators should treat those files as part of the deployed runtime contract,
especially when reproducing issues between emulator, host, and device setups.

## Advanced Features: Hooks, Skills, and Event Streaming

### Pre/Post-Tool Hooks & Approval Gates

Voxi supports executing configurable pre-tool and post-tool hooks defined in `hooks.json` under the config directory (`~/.voxi/config/hooks.json`).

#### Hook Configuration
A hook matches on tool name prefix patterns and defines actions:
- `allow`: Proceed with tool execution.
- `deny`: Block the tool and return a descriptive error immediately.
- `ask`: Pause execution and request user approval.

Example `hooks.json`:
```json
{
  "pre_hooks": [
    {
      "pattern": "fs.write_*",
      "action": "ask",
      "timeout_seconds": 30
    },
    {
      "pattern": "sys.shutdown",
      "action": "deny"
    }
  ],
  "post_hooks": []
}
```

#### Approval Gates (`ask` action)
When a hook evaluates to `ask`:
1. The execution blocks on a pending approval object.
2. A `hook_approval_request` event containing the approval ID, tool name, arguments, and timeout countdown is published to the daemon event bus.
3. The Web Dashboard displays an interactive modal with countdown rings, allowing operators to approve or deny the action.
4. If approved, the tool proceeds; if denied or timed out, it returns an approval rejection error.

### Dynamic Skills Catalog

Developer skills (defined as directories containing `SKILL.md`) are scanned dynamically from repository-level and home-level paths.

#### Toggle States
Skill activation states (enabled/disabled toggles) are stored and persisted in `skills_state.json` inside the config directory. Toggling a skill in the Web Dashboard UI immediately writes to this configuration and updates active agent capability sets.

#### Draft Skill Review & Approval
When new skills are drafted (created with `SKILL.md.draft`), they are placed in the review stream. The Web Dashboard provides:
- A side-by-side comparative diff display of draft vs original contents.
- Automatic verification flags for required dependencies.
- "Approve & Save" and "Discard Draft" action handlers.

### Real-Time SSE Event Broadcasting

The web dashboard mounts a token-authorized server-sent events stream endpoint at `/api/events`.
- **Axum SSE Hop**: Axum establishes a Unix IPC socket connection to subscribe to daemon events via the JSON-RPC `subscribe_events` handler, forwarding them directly to frontend SSE clients.
- **Timeline Visualizer**: Real-time event streams drive the Web Dashboard's timeline visualizer, showing live states (running, success, failure) of tool executions and pending hook approvals.

## Extension Model

Voxi supports extension through a mix of runtime modules and metadata
plugins.

Important extension paths include:

- built-in LLM backend modules
- plugin-managed LLM backend metadata
- skill metadata plugins
- CLI plugin metadata
- tool execution through the sidecar

This split keeps the daemon core responsible for orchestration while allowing
new behaviors to be described or loaded through narrower extension points.

## Troubleshooting Checklist

If the daemon does not behave as expected, start with these checks:

1. Confirm the daemon is running: `./deploy.sh --status`
2. Re-run `./deploy.sh` for a fresh build and restart.
3. Verify the main service restarted successfully.
4. Check the dashboard port (default **9091**) and whether the
   dashboard process is alive.
5. Use `voxi-cli dashboard status` to confirm dashboard state.
6. Inspect `~/.voxi/logs/voxi.log` for daemon boot failures or
   configuration issues.

## Recommended Reading Order

To go deeper after using the project:

1. [README.md](../README.md)
2. [STRUCTURE.md](STRUCTURE.md)
3. `deploy.sh`
4. `src/voxi/src/main.rs`
5. `src/voxi-cli/src/main.rs`
6. `src/voxi-web-dashboard/src/main.rs`
