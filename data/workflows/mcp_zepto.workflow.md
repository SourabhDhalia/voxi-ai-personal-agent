---
id: mcp_zepto
name: Zepto Shopping Workflow
description: Executes Zepto groceries setup, selection, and product search
trigger: zepto
---

## Step 1: List saved addresses
- type: tool
- tool_name: mcp_zepto_list_saved_addresses
- args: {}
- output_var: address_list

## Step 2: Request address selection
- type: prompt
- instruction: |
    Analyze the list of saved addresses from the output of `mcp_zepto_list_saved_addresses`:
    1. If the list contains exactly 1 address:
       - Automatically select that address.
       - Immediately call `mcp_zepto_select_saved_address` with its `addressId` (e.g., {"addressId": "some-id"}).
       - Do not ask the user for confirmation; proceed in the background.
    2. If the list contains 2 or more addresses:
       - DO NOT automatically select one.
       - Present the saved addresses to the user in a numbered list (including names, addresses, landmarks if available).
       - Ask the user to select which address they would like to use by replying with the option number, or if they want to use a different address.
       - Wait for the user's selection. Once the user replies with the option/selection, call `mcp_zepto_select_saved_address` with the chosen `addressId`.
    3. If the list is empty:
       - Ask the user to add a new address or provide their delivery location.

## Step 3: Get location serviceability
- type: tool
- tool_name: mcp_zepto_get_location_serviceability
- args: {}
- output_var: serviceability_res

## Step 4: Select store
- type: tool
- tool_name: mcp_zepto_select_store
- args: {}
- output_var: store_res

## Step 5: Search products
- type: prompt
- instruction: |
    Call `mcp_zepto_search_products` with:
    - `queries`: An array of search query strings (e.g., ["apple"]).
    Once results are fetched, present them clearly to the user in the numbered format.
