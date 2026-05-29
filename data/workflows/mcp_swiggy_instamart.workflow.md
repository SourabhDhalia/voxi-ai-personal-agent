---
id: mcp_swiggy_instamart
name: Swiggy Instamart Shopping Workflow
description: Executes Swiggy Instamart groceries setup and product search
trigger: swiggy
---

## Step 1: List saved addresses
- type: tool
- tool_name: mcp_swiggy_instamart_get_addresses
- args: {}
- output_var: address_list

## Step 2: Search products
- type: prompt
- instruction: |
    Analyze the list of saved addresses from the output of `mcp_swiggy_instamart_get_addresses`:
    1. If the list contains exactly 1 address:
       - Automatically select that address.
       - Immediately call `mcp_swiggy_instamart_search_products` with:
         - `query`: The user's search query (e.g. "apple").
         - `addressId`: The selected address's ID.
       - Do not ask the user for confirmation; proceed in the background.
    2. If the list contains 2 or more addresses:
       - DO NOT automatically select one.
       - Present the saved addresses to the user in a numbered list (including names, addresses, landmarks if available).
       - Ask the user to select which address they would like to use by replying with the option number, or if they want to use a different address.
       - Wait for the user's selection. Once the user replies with the option/selection, call `mcp_swiggy_instamart_search_products` with:
         - `query`: The user's search query (e.g. "apple").
         - `addressId`: The chosen address's ID.
    3. If the list is empty:
       - Ask the user to add a new address or provide their delivery location.
    Once results are fetched, present them clearly to the user in the numbered format.
