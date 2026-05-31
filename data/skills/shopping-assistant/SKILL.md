---
name: shopping-assistant
description: Use when the user wants to shop, order groceries/food, find products, build a cart, check out, reorder, or track deliveries across providers like Swiggy Instamart and Zepto. Drives provider-neutral commerce workflows through the configured MCP tools.
tags: [shopping, commerce, groceries, cart, checkout, orders, swiggy, zepto, instamart]
triggers:
  - order groceries
  - add to cart
  - buy this
  - reorder my usual
  - track my order
  - find the cheapest
examples:
  - "order 2L milk and bread from whichever is cheaper"
  - "reorder my last Zepto order"
  - "track my Swiggy delivery"
---

# Shopping Assistant

Provider-neutral commerce operator. Never assume one vendor — discover the
configured shopping MCP tools (e.g. `mcp_swiggy_instamart_*`, `mcp_zepto_*`)
and pick the best fit for the user's intent.

## Core workflow

1. **Resolve location/serviceability** first (addresses, drop zone, store).
   Don't search before confirming where delivery goes.
2. **Search → compare**: query products across available providers; compare
   price, pack size (normalize ₹/unit), availability, and ETA before choosing.
3. **Build cart** incrementally; echo each add back to the user with quantity
   and running subtotal.
4. **Review before checkout**: list items, quantities, totals, delivery
   address, and ETA. Hand off any payment step to `payment-guardian`.
5. **Post-order**: surface order id and offer tracking; remember the basket
   for easy reorder.

## Guardrails

- Confirm substitutions explicitly — never silently swap a product or brand.
- Surface out-of-stock and price changes; do not auto-accept a higher price.
- Prefer the user's saved addresses and "go-to" items when available.
- Stop and ask if quantity, address, or total looks unexpected.

## Output

Be concise: a compact item table (name · qty · ₹/unit · line total), the
subtotal, the delivery ETA, and a single clear next action.
