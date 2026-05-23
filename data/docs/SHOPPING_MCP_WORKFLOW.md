# Shopping MCP Workflow

Use this reference when the user asks TizenClaw to shop, manage a cart,
order restaurant food, reorder items, plan groceries from a recipe, or
book a restaurant table through configured MCP providers.

## Runtime Targets

- Production target: Tizen DTV on armv7l.
- Test target: Ubuntu x86_64 host environment.
- Shopping MCP servers are configured in `config/mcp_servers.json`.
- Discovered tools are exposed with sanitized provider-prefixed names
  such as `mcp_<provider>_<tool>`.
- Tool names, descriptions, and input schemas from `tools/list` are the
  source of truth for each provider's flow.

## Current Example Providers

- Zepto: quick-commerce groceries, cart management, order history, and
  checkout when available.
- Swiggy Instamart: grocery and essentials search, cart management, bill
  review, and COD checkout when available.
- Swiggy Food: restaurant search, menus, food cart management, bill
  review, and COD ordering when available.
- Swiggy Dineout: restaurant discovery, details, slot checks, and free
  table bookings when available.
- Future providers: any shopping MCP added to `mcp_servers.json` should
  work through the same metadata discovery path.

## Tool Discovery

1. If the relevant MCP tool is already in context, inspect its schema and
   use it directly.
2. Otherwise call `search_tools` with provider, capability, item, action,
   or approximate/misspelled terms.
3. Prefer the highest-scoring MCP tool whose provider, description, and
   schema match the user's intent.
4. If multiple providers match, compare or ask a concise choice question.
5. Do not assume a hardcoded provider flow; follow the selected MCP
   tool's own descriptions and schemas.

## Conversation Flow

1. Understand whether the request is groceries, food delivery, reorder,
   recipe shopping, occasion planning, price comparison, or table booking.
2. Ask only for missing details that affect the outcome, such as
   quantity, brand, budget, locality, address context, party size,
   date/time, substitutions, authentication, or payment preference.
3. Search or inspect provider results before changing a cart, checkout
   state, payment, order, reservation, or booking.
4. For misspellings, search likely corrected terms before asking the
   user to restate.
5. Prefer exact matches first, then practical substitutions. Explain
   substitutions briefly.
6. Present a cart, order, or booking draft with items, quantities,
   prices, unavailable items, substitutions, fees, delivery time, booking
   slot, and estimated total when available.
7. Ask for explicit confirmation before checkout, COD order placement,
   payment initiation, reservation, or table booking.
8. Act only after the user's latest message clearly confirms the final
   cart, order, payment path, or booking slot.

## Safety Rules

- Shopping MCP actions can affect real accounts and create real orders or
  reservations.
- The daemon blocks risky MCP tools until the latest user turn explicitly
  confirms the exact pending tool and arguments.
- Never place an order, reserve payment, trigger checkout, or book a
  table from inferred preference.
- For Swiggy, remind the user not to use the app simultaneously if a
  session conflict is likely.
- If OAuth, OTP, location, address, or account authentication is required,
  explain the required user action and wait.
- If a provider tool returns an error, summarize the failure and propose a
  narrower retry or manual fallback.
- If `requires_confirmation` is returned, show the final cart/order or
  booking and wait for a fresh user confirmation before retrying.

## Useful Prompt Patterns

- "Order 1L Amul toned milk, a dozen eggs, and bread."
- "Compare available grocery providers for bread and milk."
- "Order biryani on Swiggy."
- "Build a grocery cart from this paneer recipe."
- "Book a table for 2 this Saturday at 8 PM."
