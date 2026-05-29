# Fix Zepto MCP Flow and Routing Reliability

## Summary
- Zepto itself is connected: logs show `zepto connected (23 tools)` and successful `tools/call`.
- The failure is orchestration: `/mcp swiggy-instamart` is not a supported command, dashboard race mixed an older answer into the Zepto chat, Ollama chose widget-only `zepto_shop`, and address/store selection happened too late.
- Fix by adding a deterministic Zepto MCP flow, hiding widget-only tools from Ollama/CLI, and enforcing chat/request routing from the previous stop/request-id plan.

## Key Changes
- Add explicit MCP status/help handling:
  - `/mcp <server>` becomes a supported alias for `/mcp status <server>`.
  - `/mcp status zepto` reports auth state, safe tool names, and the recommended CLI flow.
  - `/mcp tools zepto` lists safe names such as `mcp_zepto_list_saved_addresses`.
- Add a Zepto flow guard in `AgentCore`:
  - For Zepto shopping/search prompts, inject or enforce this order:
    1. `mcp_zepto_get_user_details`
    2. If unregistered, ask full name then `mcp_zepto_update_user_name`
    3. `mcp_zepto_list_saved_addresses`
    4. `mcp_zepto_select_saved_address` using a real address id
    5. `mcp_zepto_get_past_order_items`
    6. `mcp_zepto_search_products` or `mcp_zepto_search_multiple_products`
    7. `mcp_zepto_get_product_details` only with returned variant id
    8. Cart/order tools only after explicit latest-turn confirmation
- Treat ChatGPT widget tools as non-preferred for daemon/Ollama:
  - Do not expose `mcp_zepto_zepto_shop` to local Ollama unless widget mode is explicitly active.
  - Keep exposing CLI/API tools: addresses, store, search, cart, payment, order.
- Strengthen MCP alias dispatch:
  - Before local tool fallback, always resolve original names like `search_products` against connected MCP tools.
  - If an original name is ambiguous, fail closed with safe-name choices.
  - Log canonical routing: `search_products -> mcp_zepto_search_products`.
- Add Zepto-specific loop prevention:
  - If Ollama calls search before address/store selection, return a corrective tool result naming the required next safe tool.
  - If it repeats the wrong step twice, stop that round with a clear user-facing error.
  - If search returns empty/short results, do not call `zepto_shop`; retry once with normalized query or past-order exact name, then answer honestly.

## Chat Routing Dependency
- Implement the request-id stop/race fix first or in the same commit:
  - Every `/api/chat` request carries `session_id + request_id`.
  - Dashboard replaces the matching pending placeholder only.
  - Late responses from old chats never get appended to the currently open chat.
- This directly addresses the log where an old `/mcp swiggy-instamart` answer was pasted into the later Zepto milk request, polluting the next prompt.

## Tests and Verification
- Add resolver tests:
  - `search_products` routes to `mcp_zepto_search_products` when only Zepto exposes it.
  - Ambiguous original names return safe-name choices.
  - `mcp_zepto_zepto_shop` is hidden from non-widget Ollama tool declarations.
- Add flow tests:
  - Milk price prompt with address id selects address before search.
  - Search-before-address receives corrective result.
  - Risky cart/order tools still require confirmation.
  - `/mcp zepto`, `/mcp status zepto`, and `/mcp tools zepto` return deterministic status/help.
- Target validation only:
  - Ubuntu x86_64: `./deploy_host.sh --test`
  - VoxiOS armv7l: `./deploy.sh`
- Manual Zepto flow:
  - Pull latest code, start daemon, run `/mcp status zepto`.
  - Ask: `use zepto address-id <real-id>: find 500ml Amul milk price`.
  - Confirm logs show `select_saved_address`, `get_past_order_items`, then `search_products`.

## Assumptions
- Zepto token/session is valid because startup shows token present and Zepto connected.
- The tested log may be from before the latest alias hardening; verification must confirm commit `7593178f` or newer is deployed.
- For Ollama/local daemon mode, widget-only Zepto tools should be suppressed unless a web widget client explicitly asks for them.
