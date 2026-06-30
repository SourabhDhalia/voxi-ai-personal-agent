# Project: Voxi Dashboard Redesign

## Architecture
- Single Page Application (SPA) served as static assets by Axum Rust web server.
- Interface with the Voxi Agent daemon via UNIX socket API endpoint `/api/metrics` for stats, and `/api/chat/stream` for chat streaming.
- CSS style variables in `:root` and user customizer class rules on `<html>` control colors, shadows, borders, backgrounds, fonts, and styles.
- Customizer settings persisted in browser `localStorage` as JSON (`voxi_ui_config`).

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Stage 1: Planning | Define scope, analyze requirements, establish bindings preservation map | None | DONE |
| 2 | Stage 2: Design | Draft the CSS layout architecture, Bento grid structure, frosted glass effects, navigation styles, and custom scrollbars | M1 | IN_PROGRESS |
| 3 | Stage 3: Development | Implement UI structural changes in `index.html`, style variables and elements in `style.css`, and chat streaming/metric stability in `app.js` | M2 | PLANNED |
| 4 | Stage 4: Build/Deploy | Execute GBS deployment via `./deploy.sh` to package and serve new web assets | M3 | PLANNED |
| 5 | Stage 5: Test/Review | Run `./deploy.sh --test` to verify front-end serving, metric polling integrity, and test coverage | M4 | PLANNED |
| 6 | Stage 6: Commit/Push | Save changes via version manager skill to repository with upstream-compliant commit message | M5 | PLANNED |

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
