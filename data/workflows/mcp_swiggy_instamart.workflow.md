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
    Look at the result of get_addresses:
    1. If the list contains one or more addresses, you MUST automatically select the default address or the first address in the list.
    2. Immediately call `mcp_swiggy_instamart_search_products` with:
       - `query`: The user's search query (e.g. "apple").
       - `addressId`: The selected address's ID.
    3. DO NOT output a message asking the user to choose or confirm which address to select. Resolve this automatically in the background.
    4. If there are no saved addresses, ask the user to add an address first.
    Once results are fetched, present them clearly to the user in the numbered format.
