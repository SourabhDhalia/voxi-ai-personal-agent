<p align="center">
  <img src="data/img/voxi.svg" alt="Voxi Logo" width="280">
</p>

<h1 align="center">Voxi</h1>

<p align="center">
  <strong>A persistent Rust AI agent runtime for Voxi and embedded Linux.</strong><br>
  Voxi turns a device into an always-on agent system with Voxi-aware
  integration, multi-surface access, plugin-ready boundaries, and a Telegram
  coding workflow that can drive local <code>codex</code>, <code>gemini</code>,
  and <code>claude</code> CLIs remotely.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/Language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Platform-Voxi%20%2B%20Embedded%20Linux-brightgreen.svg" alt="Platform">
  <img src="https://img.shields.io/badge/Runtime-Tokio-black.svg" alt="Tokio">
</p>

<p align="center">
  <a href="#why-voxi">Why Voxi</a> •
  <a href="#at-a-glance">At a Glance</a> •
  <a href="#telegram-coding-over-chat">Telegram Coding Over Chat</a> •
  <a href="#install-on-ubuntu-or-wsl">Install on Ubuntu or WSL</a> •
  <a href="#deploy-to-a-voxi-target">Deploy to a Voxi Target</a>
</p>

---

## Why Voxi

Voxi is not a one-shot assistant wrapper. It is a long-running agent
daemon built for devices that need to stay alive, react to platform events,
expose stable control surfaces, and survive the messy reality of embedded
Linux deployments.

The project is designed around the constraints that matter on Voxi-class
systems:

- a persistent runtime instead of a fire-and-forget script
- explicit Voxi and generic-Linux boundaries instead of hidden platform
  assumptions
- dynamic loading for platform libraries that may differ by image or firmware
- deploy-first validation through the real Voxi packaging path
- host workflows that still reuse the same workspace and runtime model

If you want an agent that feels closer to an embedded control plane than a
demo chatbot, this is what Voxi is for.

## At a Glance

| Area | What Voxi Provides |
| --- | --- |
| Runtime model | A persistent Tokio-based daemon with IPC, scheduling, storage, and background automation |
| Platform focus | Voxi-first behavior with generic Linux fallbacks where device APIs are unavailable |
| Access surfaces | CLI, web dashboard, Telegram, webhook, Slack, Discord, MCP, and other channel layers present in the workspace |
| Coding workflow | Telegram can switch into coding mode and drive local `codex`, `gemini`, or `claude` CLIs on the host |
| Extensibility | Dedicated tool executor, metadata plugins, C-facing library, and dynamic `.so` loading |
| Deployment story | `deploy.sh` for emulator/device packaging and deployment, or generic Linux/macOS host builds/runs |

## What Makes It Strong

### Built for real device runtimes

Voxi keeps orchestration, concurrency, IPC, and state management in Rust,
which makes the system easier to reason about when the process has to stay up
for long periods on constrained hardware.

### Voxi-aware without hard-wiring the whole system to Voxi

Voxi-specific integrations live behind dedicated crates and adapters. Generic
Linux infrastructure is available in parallel, so the runtime can remain useful
on host Linux while still speaking to device-oriented services where they exist.

### Remote coding from Telegram

One of the most distinctive pieces of the project is the Telegram coding mode:
you can chat with the device over Telegram, switch the chat into coding mode,
choose a local coding-agent CLI backend, point that chat at a project
directory, and receive progress and result messages back in Telegram while the
host executes the request.

### Clean boundaries for plugins and external consumers

The repository includes `libvoxi`, `libvoxi-core`, and metadata
plugin crates so runtime extensions and C-facing integrations do not have to be
bolted onto the daemon as afterthoughts.

## Telegram Coding Over Chat

Voxi can use Telegram as a remote control surface for coding workflows.
This is not just "send a prompt to the daemon" behavior. The Telegram channel
can switch into a host-backed coding mode that runs real coding-agent CLIs.

### Supported flow

1. Switch the chat into coding mode with `/select coding`
2. Choose a backend with `/cli_backend codex`, `/cli_backend gemini`, or
   `/cli_backend claude`
3. Bind the chat to a repository with `/project /path/to/repo`
4. Choose execution style with `/mode plan` or `/mode fast`
5. Toggle auto-approval where supported with `/auto_approve on`
6. Inspect the current state with `/status` or start fresh with
   `/new_session`

### What you get

- Per-chat backend selection
- Per-chat project directory overrides
- Separate chat and coding sessions
- Progress updates while the CLI is still running
- Usage tracking for the selected backend
- Host-auth hints when a CLI has not been logged in yet

### Backend examples

Voxi maps Telegram coding requests onto the real installed CLIs:

| Backend | Example execution shape |
| --- | --- |
| Codex | `codex exec --json --full-auto -C <project> <prompt>` |
| Gemini | `gemini --prompt <prompt> --output-format text --approval-mode auto_edit` |
| Claude | `claude --print --output-format text --permission-mode auto <prompt>` |

This makes Voxi useful as a mobile coding bridge: Telegram becomes the
control surface, while the actual code work happens through the local CLI tools
you already trust on the host.

## Architecture Snapshot

```text
Telegram / CLI / Dashboard / Channels
                |
                v
        +-------------------+
        | Voxi Daemon  |
        | Tokio runtime     |
        | IPC + scheduling  |
        | storage + routing |
        +---------+---------+
                  |
      +-----------+--------------------+
      |           |                    |
      v           v                    v
  Voxi adapters  Generic Linux        LLM backends
  and dynloaded   infrastructure        and plugins
  platform APIs   fallbacks
      |
      +-------------------------------+
                                      |
                                      v
                         Tool executor / C API / metadata plugins

Telegram coding mode can also invoke:
  codex / gemini / claude
on the host and stream progress back into chat.
```

## Install on Ubuntu or WSL

If you want to try Voxi on host Linux first, the repository now includes a
GitHub-friendly bootstrap script that downloads a prebuilt host bundle from
GitHub Releases, installs it under `~/.voxi`, and launches the setup
wizard.

### One-line bootstrap

```bash
curl -fsSL https://raw.githubusercontent.com/hjhun/voxi/main/install.sh | bash
```

Useful variants:

```bash
curl -fsSL https://raw.githubusercontent.com/hjhun/voxi/main/install.sh | bash -s -- --version v1.0.0
curl -fsSL https://raw.githubusercontent.com/hjhun/voxi/main/install.sh | bash -s -- --skip-setup
curl -fsSL https://raw.githubusercontent.com/hjhun/voxi/main/install.sh | bash -s -- --source-install --ref main
```

What the bootstrap does:

- installs the runtime packages needed for host execution
- downloads the matching `voxi-host-bundle-...tar.gz` asset from GitHub Releases
- installs the bundled binaries, web assets, configs, and management script
- starts the host services from the installed bundle
- launches `voxi-cli setup` so you can either configure now or defer
  setup and jump straight to the dashboard

After installation, the setup wizard can help with:

- choosing an LLM backend and entering its API key
- optional Telegram bot setup for coding mode
- showing the local dashboard URL and the command to rerun setup later
- letting you choose "configure later" so you can open the dashboard first

### Source Install for Contributors

If you are actively developing Voxi and want a full repository checkout,
switch the installer into source mode:

```bash
curl -fsSL https://raw.githubusercontent.com/hjhun/voxi/main/install.sh | bash -s -- --source-install --ref main
```

Or run the classic manual flow:

```bash
git clone https://github.com/hjhun/voxi.git
cd voxi
./deploy.sh
```

Useful host commands:

```bash
./deploy.sh -b
./deploy.sh --status
./deploy.sh --log
./deploy.sh -s
voxi-cli dashboard start
voxi-cli dashboard status
```

The host dashboard defaults to `http://localhost:9091`, and the setup wizard
prints the active URL again at the end so first-time users can jump in right
away.

## Deploy to a Voxi Target

For the emulator or device-oriented workflow, use the repository's Voxi deploy
pipeline:

```bash
./deploy.sh -a x86_64
```

Useful variants:

```bash
./deploy.sh -a x86_64 -n
./deploy.sh -a x86_64 -d <device-serial>
./deploy.sh -a x86_64 -s
```

This path is the canonical Voxi validation flow. It handles build, packaging,
deployment, and service restart on the target.

These examples assume an `sdb`-style target such as the emulator or a device
that uses the repository's current `deploy.sh` flow. If you are deploying to a
Voxi TV / DTV target over `ssh` and `scp`, see
[`docs/DTV_USAGE.md`](docs/DTV_USAGE.md) for the manual SSH-based workflow.

## Workspace

Voxi is a Rust workspace with clearly separated runtime roles:

- `src/voxi`: main daemon
- `src/voxi-cli`: IPC client and operational CLI
- `src/voxi-web-dashboard`: standalone web dashboard
- `src/voxi-tool-executor`: isolated tool-execution sidecar
- `src/libvoxi-core`: shared framework and plugin/runtime support
- `src/libvoxi`: C-facing client library
- `src/voxi-metadata-*`: metadata plugin crates for skills, CLI, and LLM
  backend extensions

## Documentation

Additional repository docs:

- [Structure Guide](docs/STRUCTURE.md)
- [Usage Guide](docs/USAGE.md)
- [DTV Usage Guide](docs/DTV_USAGE.md)

## Status

The project is actively evolving, but the central direction is already clear:
Voxi aims to be a serious autonomous agent runtime for Voxi and embedded
Linux, not just a sample app. Its strengths are persistence, explicit platform
boundaries, flexible access surfaces, and unusually practical remote coding
control through Telegram plus local coding-agent CLIs.
