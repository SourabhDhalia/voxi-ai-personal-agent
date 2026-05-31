---
name: payment-guardian
description: Use before any checkout, payment, order placement, or money-spending action. Enforces explicit user confirmation, verifies totals and items, and blocks silent or surprise spending. Pairs with the pre_tool hook ask-gate.
tags: [payment, checkout, safety, confirmation, money, spending, guardrail]
triggers:
  - place the order
  - pay now
  - checkout
  - confirm purchase
  - complete payment
examples:
  - "checkout my cart" → confirm the full summary and amount first
  - "pay with UPI" → verify total and method before proceeding
---

# Payment Guardian

A spending safety gate. Any action that moves money or places an order must
pass through an explicit, itemized confirmation before it executes.

## Hard rules

1. **Never auto-spend.** Always present an itemized summary and wait for a
   clear "yes" before placing an order or triggering payment.
2. **Show the exact total** the user will be charged, including delivery/fees,
   and the payment method.
3. **Re-verify on change.** If price, quantity, address, or payment method
   changed since the cart was reviewed, stop and re-confirm.
4. **Bound the amount.** If the total exceeds the user's usual range or a
   configured limit, require a second explicit confirmation.
5. **No partial guesses.** If any field (address, method, total) is missing or
   ambiguous, ask — do not assume a default.

## Confirmation template

```
Confirm purchase:
  Items:   <n> (<short list>)
  Total:   ₹<amount>  (incl. ₹<fees> fees/delivery)
  Pay via: <method>
  Deliver: <address> · ETA <eta>
Reply "confirm" to place this order, or tell me what to change.
```

## On denial / timeout

If the user declines or doesn't confirm, **do not place the order**. Report
what was held and leave the cart intact for later.
