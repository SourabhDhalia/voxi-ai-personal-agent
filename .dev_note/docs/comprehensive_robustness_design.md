# Design: Comprehensive Shopping Agent Robustness Fixes

This document details the design specifications for Stage 2 (Design) of the comprehensive robustness task.

## 1. Thread-Safe ID Cache and Parameter Injection

To prevent missing required parameters (like `addressId` or `storeId`) from failing tool executions, we implement a caching and injection mechanism in `McpClientManager`.

### Cache Structure
Add fields to `McpClientManager` in `mcp_client.rs`:
```rust
pub struct McpClientManager {
    clients: Vec<McpClient>,
    last_swiggy_address_id: std::sync::Mutex<Option<String>>,
    last_zepto_address_id: std::sync::Mutex<Option<String>>,
    last_zepto_store_id: std::sync::Mutex<Option<String>>,
}
```

### JSON ID Extraction
Create a recursive JSON walker `find_first_id_by_key(val: &Value, target_keys: &[&str]) -> Option<String>` to find the first occurrence of string IDs like:
- Addresses: `["id", "addressId", "address_id"]`
- Stores: `["id", "storeId", "store_id"]`

### Injection Logic in `call_tool_resolved`
1. **Caching**: If a tool call successfully returns, check the tool name:
   - If it contains `get_addresses` or `list_saved_addresses`: extract the address ID and save it to the appropriate provider cache.
   - If it contains `select_store`: extract the store ID and save it to `last_zepto_store_id`.
2. **Injection**: Before sending arguments to the client, inspect the requested tool name and arguments:
   - If it is a Swiggy Instamart search or cart mutation tool, and it is missing `addressId` or `address_id`, check the cache. If `last_swiggy_address_id` is populated, inject it.
   - If it is a Zepto search or cart mutation tool, check if it is missing `addressId`/`address_id` or `storeId`/`store_id`. If so, inject the cached values.

## 2. Persistent Selection Context

To connect quantity confirmations and retries reliably to the previously selected product, we will store the choice index inside the `shopping_state` JSON file.

### Heuristic
In `process_prompt.rs` -> `shopping_selection_context`:
- If `resolve_selection_index` returns `Some(index)`, save `index` under `"selected_number"` inside the session's `shopping_state` JSON.
- If it returns `None`, load `"selected_number"` from `shopping_state`. If found, continue to load and inject that selection context. This ensures selection context remains active during quantity, confirmation, or retry turns.

## 3. Tool Pruning Refinement

In `process_prompt.rs` inside the `is_shopping` branch:
- Modify `is_shopping_intent` to take `session_id` and check if `shopping_state_path` exists.
- In `tools.retain` when `is_shopping` is true, always keep `is_checkout_tool` and `is_cart_mutation_tool` as `true` instead of checking current turn keywords.

## 4. Retries and Clean User Language

In `process_prompt.rs` inside the main loop:
- If a cart mutation fails, check if the error is recoverable. If so, update the state, record the failed operation, reject loop termination, inject a system retry instruction, and continue the loop.
- In `extract_final_text`, strip any lines starting with `Called tool` or containing `Called tool 'mcp_` or `Called tool 'action_` to ensure raw logs are never printed to the user.
