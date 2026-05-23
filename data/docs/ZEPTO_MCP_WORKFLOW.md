# Zepto MCP Shopping Workflow

Use this reference when the user asks TizenClaw to shop with Zepto,
manage a quick-commerce cart, reorder items, plan groceries from a
recipe, or prepare an occasion-based shopping cart.

## Runtime Targets

- Production target: Tizen DTV on armv7l.
- Test target: Ubuntu x86_64 host environment.
- The Zepto MCP server is configured in `config/mcp_servers.json` as the
  `zepto` server. Its discovered tools are exposed with the
  `mcp_zepto_` prefix.

## Core Capabilities

- Search live Zepto products and availability.
- Manage the user's cart by adding, updating, and removing items.
- Retrieve previous order history for reorder flows.
- Place real Zepto orders after explicit user confirmation.
- Support payment choices such as cash on delivery, UPI, cards, Zepto
  Cash, and UPI reserve pay when exposed by the MCP.

## Conversation Flow

1. Understand the shopping goal: quick order, reorder, recipe-based
   shopping, photo/list interpretation, or occasion planning.
2. Ask only for missing details that affect the order, such as quantity,
   brand preference, budget, substitutions, delivery context, or payment
   preference.
3. Use Zepto MCP product search before adding items to the cart.
4. Prefer exact matches first, then practical substitutions. Explain
   substitutions briefly.
5. Build a cart draft and summarize items, quantities, prices, and any
   unavailable products.
6. Ask for explicit confirmation before checkout or payment.
7. Place the order only after the user's latest message clearly confirms
   the final cart and payment path.

## Safety Rules

- Zepto MCP actions can affect a real account and create real orders.
- Never place an order, reserve payment, or trigger checkout from an
  inferred preference.
- If OAuth, OTP, location, or account authentication is required, explain
  the required user action and wait.
- If the MCP tool returns an error, summarize the failure and propose a
  narrower retry or manual fallback.

## Useful Prompt Patterns

- "Order 1L Amul toned milk, a dozen eggs, and bread."
- "Reorder my last Zepto order."
- "I am making paneer butter masala for 4 people. Build a Zepto cart."
- "Plan snacks and drinks for a small party and keep it under my budget."
