# Zepto MCP Flow Fix and Hardening

## Summary
- Your curl flow is correct for Zepto Streamable HTTP MCP: `initialize`, capture `Mcp-Session-Id`, send `notifications/initialized`, call `tools/list`, then `tools/call`.
- Current logs show Zepto transport is already connecting and calling tools, so the main bug is not “Zepto unavailable”; it is flow/routing:
  - Ollama sometimes calls non-canonical or widget-only tools.
  - The daemon may not force address/store setup before search.
  - Dashboard response races can pollute the next Zepto prompt.
  - MCP HTTP handshake should exactly match Zepto’s expected shape.

## Key Changes
- Match the Zepto HTTP flow exactly:
  - Send `initialize.params.capabilities = {"tools": {}}` instead of `{}`.
  - Send `notifications/initialized` with `"params": {}`.
  - Keep `Accept: application/json, text/event-stream`, `Authorization: Bearer ...`, `MCP-Protocol-Version`, and captured `Mcp-Session-Id`.
  - Add logs for `initialize`, captured session id, `tools/list`, and each `tools/call` canonical name.
- Add a daemon diagnostic command:
  - `/mcp status zepto` shows endpoint/auth/session/tool count.
  - `/mcp tools zepto` lists remote names and safe names.
  - `/mcp test zepto search "amul 500ml milk"` runs the safe flow and reports the failing step.
- Enforce Zepto CLI/Ollama flow:
  - For product search/order requests, use:
    1. `mcp_zepto_get_user_details`
    2. `mcp_zepto_list_saved_addresses`
    3. `mcp_zepto_select_saved_address` with a real address id
    4. `mcp_zepto_get_past_order_items`
    5. `mcp_zepto_search_products` or `mcp_zepto_search_multiple_products`
  - Do not use `mcp_zepto_zepto_shop` in local daemon/Ollama mode; it is widget-oriented.
  - If address/store context is missing, return a corrective tool result instead of looping.
- Fix alias/routing:
  - Resolve `search_products`, `list_saved_addresses`, etc. to Zepto safe MCP names before any local tool fallback.
  - If a tool name is ambiguous across providers, return safe-name choices and stop that round.
  - Risky cart/order/payment tools remain confirmation-gated by canonical safe MCP name.
- Apply the chat request-id race fix from the previous plan so old Zepto/Swiggy responses cannot appear in a later chat.

## Correct Zepto Usage Flow
- For “find 500ml Amul milk price”:
  - If user gives address id, call `select_saved_address` first.
  - Call `get_past_order_items`.
  - Search with normalized query, for example `Amul milk 500ml`.
  - Present product names, pack size, availability, and price from `search_products`.
- For order placement:
  - Search and select product.
  - `update_cart`.
  - `view_cart`.
  - `get_payment_methods`.
  - Ask explicit confirmation.
  - Then call the relevant order tool with `confirmOrder: true`.

## Tests and Verification
- Add tests for:
  - HTTP initialize payload includes `capabilities.tools`.
  - initialized notification includes empty `params`.
  - Original Zepto tool names resolve to `mcp_zepto_*`.
  - `zepto_shop` is hidden from non-widget Ollama tool declarations.
  - Search-before-address receives a corrective result.
  - `/mcp test zepto search ...` reports each step.
- Target validation only:
  - Ubuntu x86_64: `./deploy_host.sh --test`
  - TizenOS armv7l: `./deploy.sh`
- Manual check:
  - Run `/mcp status zepto`.
  - Run `/mcp tools zepto`.
  - Ask: `use zepto address-id <real-id>: find 500ml Amul milk price`.
  - Confirm logs show `select_saved_address`, `get_past_order_items`, then `search_products`.

## Assumptions
- Zepto token is valid because logs show `token=yes` and `zepto connected (23 tools)`.
- Direct HTTP and current bridge-wrapper modes must both remain supported.
- Local Ollama should use CLI/API Zepto tools, not ChatGPT widget tools.
