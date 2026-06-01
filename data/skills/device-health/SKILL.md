---
name: device-health
description: Use when the user asks about device status, battery, temperature, CPU, memory, storage, network, or overall system health. Produces a clear health report with thresholds and actionable recommendations, cross-platform (Linux and macOS).
tags: [device, health, battery, temperature, cpu, memory, storage, network, monitoring]
triggers:
  - how is my device
  - check system health
  - battery status
  - is it overheating
  - how much storage is left
examples:
  - "is the device healthy?"
  - "why is it running hot?"
  - "how much memory is free?"
---

# Device Health

Reports device health from available metrics and flags anything out of range.
Use cross-platform sources — never assume Linux-only files like `/proc`; fall
back gracefully where a metric is unavailable.

## Report structure

1. **Status**: overall verdict — ✅ Good / ⚠️ Warning / 🔴 Critical.
2. **Metrics**: key numbers with context (current vs normal).
3. **Alerts**: any value outside the normal range, most severe first.
4. **Recommendations**: concrete next steps.

## Normal ranges (defaults)

| Metric | Good | Warning | Critical |
|---|---|---|---|
| Battery | >20% | 10–20% | <10% |
| Temperature | <40°C | 40–50°C | >50°C |
| Memory used | <80% | 80–90% | >90% |
| Storage free | >10% | 5–10% | <5% |

## Guidelines

- Lead with the verdict; keep the body scannable.
- Only raise alerts that have a recommended action.
- If a metric isn't available on this platform, say so briefly rather than
  guessing.
