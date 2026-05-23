# Shopping MCP Workflow

Use this reference when the user asks TizenClaw to shop, manage a cart,
order restaurant food, reorder items, plan groceries from a recipe, or
book a restaurant table through configured MCP providers.

## Runtime Targets

- Production target: Tizen DTV on armv7l.
- Test target: Ubuntu x86_64 host environment.
- Shopping MCP servers are configured in `config/mcp_servers.json`.
- Discovered tools are exposed with provider-prefixed names such as
  `mcp_zepto_`, `mcp_swiggy-instamart_`, `mcp_swiggy-food_`, and
  `mcp_swiggy-dineout_`.

## Current Providers

- Zepto: quick-commerce groceries, cart management, order history, and
  checkout when available.
- Swiggy Instamart: grocery and essentials search, cart management, bill
  review, and COD checkout when available.
- Swiggy Food: restaurant search, menus, food cart management, bill
  review, and COD ordering when available.
- Swiggy Dineout: restaurant discovery, details, slot checks, and free
  table bookings when available.

## Provider Routing

1. If the user names a provider, use that provider.
2. For groceries without a provider, consider Zepto and Swiggy Instamart.
3. For restaurant delivery, prefer Swiggy Food.
4. For table reservations or dineout requests, prefer Swiggy Dineout.
5. For future shopping MCP providers, use provider names and tool
   descriptions from discovered MCP tools.

## Conversation Flow

1. Understand whether the request is groceries, food delivery, reorder,
   recipe shopping, occasion planning, price comparison, or table booking.
2. Ask only for missing details that affect the outcome, such as
   quantity, brand, budget, locality, address context, party size,
   date/time, substitutions, authentication, or payment preference.
3. Search or inspect provider results before changing a cart or booking.
4. Prefer exact matches first, then practical substitutions. Explain
   substitutions briefly.
5. Present a cart, order, or booking draft with items, quantities,
   prices, unavailable items, substitutions, fees, delivery time, booking
   slot, and estimated total when available.
6. Ask for explicit confirmation before checkout, COD order placement,
   payment initiation, or table booking.
7. Act only after the user's latest message clearly confirms the final
   cart, order, payment path, or booking slot.

## Safety Rules

- Shopping MCP actions can affect real accounts and create real orders or
  reservations.
- Never place an order, reserve payment, trigger checkout, or book a
  table from inferred preference.
- For Swiggy, remind the user not to use the app simultaneously if a
  session conflict is likely.
- If OAuth, OTP, location, address, or account authentication is required,
  explain the required user action and wait.
- If a provider tool returns an error, summarize the failure and propose a
  narrower retry or manual fallback.

## Useful Prompt Patterns

- "Order 1L Amul toned milk, a dozen eggs, and bread."
- "Compare Zepto and Swiggy Instamart for bread and milk."
- "Order biryani on Swiggy."
- "Build a grocery cart from this paneer recipe."
- "Book a table for 2 this Saturday at 8 PM."
