# Project: Voxi OS Workspace Redesign

## Architecture
- Single Page Application (SPA) served as static assets by Axum Rust web server.
- Interface with the Voxi Agent daemon via UNIX socket API endpoint `/api/metrics` for stats, and `/api/chat/stream` for chat streaming.
- CSS style variables in `:root` and user customizer class rules on `<html>` control colors, shadows, borders, backgrounds, fonts, and styles.
- Customizer settings persisted in browser `localStorage` as JSON (`voxi_ui_config`).
- Unified workspace canvas (`#workspace-canvas`) replacing standard layouts with resizable, snappable window panels.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Stage 1: Planning | Define workspace redesign requirements, map DOM bindings to preserve, and outline panel structure | None | DONE |
| 2 | Stage 2: Design | Draft the workspace grid layout, window header layout, command palette, and context menu | M1 | DONE |
| 3 | Stage 3: Development | Implement `.nav-rail`, `#workspace-canvas`, panel snapping, Command Palette, and Custom Context Menu in html/css/js | M2 | DONE |
| 4 | Stage 4: Build/Deploy | Execute deployment via `./deploy.sh` to package and serve new OS workspace assets | M3 | DONE |
| 5 | Stage 5: Test/Review | Run `./deploy.sh --test` to verify telemetry data, chat streams, window controls, and run reviews/audits | M4 | DONE |
| 6 | Stage 6: Commit/Push | Save changes via version manager skill to repository with upstream-compliant commit message | M5 | DONE |

## Interface Contracts
### `app.js` ↔ `index.html` (DOM Bindings)
All elements listed in the Binding Preservation Matrix in the analysis report must maintain exact IDs to support JS metric updates, event delegation, and input capture.

### `app.js` ↔ `/api/metrics` (Data Formats)
The JSON structure parsed by `refreshMetrics()` is preserved exactly:
- `m.uptime.formatted` (uptime)
- `m.counters.llm_calls`, `m.counters.tool_calls`, `m.counters.errors` (stats)
- `m.tokens.prompt`, `m.tokens.completion`, `m.tokens.cache_read`, `m.tokens.cache_write` (token stats)
- `m.memory.vm_rss_kb` (memory RSS)
- `m.cpu.load_1m` (CPU load)
- `m.threads` (threads count)
- `m.pid` (PID)
