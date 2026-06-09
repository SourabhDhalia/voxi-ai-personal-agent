# Voxi API Specifications (layered)

Three OpenAPI 3.1 documents, one per surface. All validated as well-formed
(parse-checked); lint with Redocly when network is available.

| Spec | Surface | Reality |
|------|---------|---------|
| [ipc-jsonrpc.openapi.yaml](ipc-jsonrpc.openapi.yaml) | Daemon control plane | JSON-RPC 2.0 over length-prefixed socket. Methods derived from `core/ipc_server.rs` + `tests/system/*.json`. OpenAPI is a documentation projection (`POST /rpc`). |
| [dashboard.openapi.yaml](dashboard.openapi.yaml) | Web dashboard | 21 real axum routes from `src/voxi-web-dashboard/src/main.rs`. Session-cookie auth. |
| [integration-api.openapi.yaml](integration-api.openapi.yaml) | **Proposed** external REST API | Greenfield design for models/skills/sandbox/memory. Bearer auth, RFC 7807, cursor pagination, `/v1`. |

## Conventions
- Errors use RFC 7807 `application/problem+json` with a stable `type` URI.
- Collections paginate with opaque cursors (`{ data, pagination:{ next_cursor, has_more } }`).
- The integration API is versioned in the path (`/v1`); breaking changes ship a new version with a deprecation window.

## Lint locally
```bash
npx @redocly/cli lint ipc-jsonrpc.openapi.yaml
npx @redocly/cli lint dashboard.openapi.yaml
npx @redocly/cli lint integration-api.openapi.yaml
# Mock a surface:
npx @stoplight/prism-cli mock integration-api.openapi.yaml
```
