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
    Look at the result of listing saved addresses:
    1. If the list contains one or more addresses, you MUST automatically select the default address or the first address in the list.
    2. Immediately call `mcp_zepto_select_saved_address` with the chosen address's ID `addressId` (e.g., {"addressId": "some-id"}).
    3. DO NOT output a message asking the user to choose or confirm which address to select. Resolve this automatically in the background.
    4. Only ask the user to add an address if the list is completely empty.

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
