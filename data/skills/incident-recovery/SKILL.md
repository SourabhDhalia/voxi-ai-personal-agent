---
name: incident-recovery
description: Use when a tool call, order, or workflow fails, errors out, times out, or returns an unexpected result. Diagnoses the failure, applies a safe fallback or retry, and escalates clearly when it cannot recover.
tags: [recovery, error, failure, retry, fallback, resilience, diagnosis, troubleshooting]
triggers:
  - that failed
  - it errored
  - didn't work
  - try again
  - something went wrong
examples:
  - "the order failed — what now?"
  - "checkout timed out"
  - "the tool returned an error"
---

# Incident Recovery

Keeps the agent resilient when something breaks. Backs the recovery role.

## Workflow

1. **Classify the failure**: transient (timeout, rate limit, network) vs
   permanent (invalid input, unavailable item, auth/permission).
2. **Recover by class**:
   - Transient → retry with backoff (small, bounded number of attempts).
   - Permanent → switch strategy: alternate provider/tool, adjusted params,
     or a safe partial result.
3. **Never silently retry money or messaging actions** — re-confirm with the
   user before re-attempting anything that spends or sends.
4. **Preserve state**: keep carts, drafts, and progress intact across retries.
5. **Escalate cleanly**: if recovery fails, report what was tried, the likely
   cause, and the single best next step for the user.

## Guidelines

- Bound retries; don't loop. Two or three attempts, then escalate.
- Make failures legible: plain-language cause, not raw stack traces.
- Prefer a safe partial outcome over an unsafe full retry.
