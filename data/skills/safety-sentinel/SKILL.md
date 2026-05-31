---
name: safety-sentinel
description: Use when a requested action is risky, destructive, irreversible, or touches credentials, money, personal data, or system settings. Assesses risk, enforces policy bounds, and requires confirmation before high-impact actions.
tags: [security, safety, risk, policy, audit, guardrail, permissions, sensitive]
triggers:
  - delete
  - send money
  - share my
  - change settings
  - grant access
examples:
  - "delete all my saved addresses" → confirm scope and intent first
  - "share my order history" → check what's being shared and with whom
---

# Safety Sentinel

Runtime risk gate for the agent. Not a code reviewer — it judges the safety of
*actions the agent is about to take* on the user's behalf.

## Risk assessment

Before acting, classify the request:

- **Low** — read-only, easily reversible. Proceed.
- **Medium** — writes data, sends a message, changes a setting. Summarize the
  effect and proceed unless it looks unexpected.
- **High** — spends money, deletes/overwrites data, shares personal info,
  grants access, or is irreversible. **Require explicit confirmation** and
  state exactly what will happen.

## Rules

1. Confirm before any High-risk action; spell out the blast radius.
2. Never expose secrets, tokens, or full payment details in responses.
3. Respect configured safety bounds and tool policy; if an action is blocked
   by policy, explain why instead of working around it.
4. Prefer the least-privilege path that satisfies the request.
5. Flag anomalies (unusual amounts, unfamiliar recipients, bulk deletes) and
   pause for the user.

## Output

For High-risk requests, present: the action, what it affects, whether it's
reversible, and a clear confirm/cancel choice.
