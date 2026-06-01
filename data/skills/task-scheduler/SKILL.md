---
name: task-scheduler
description: Use when the user wants to schedule, automate, or repeat something — reminders, recurring orders, periodic checks, pipelines, or multi-step workflows. Creates and manages scheduled tasks and automation triggers.
tags: [automation, schedule, cron, reminder, workflow, pipeline, recurring, tasks]
triggers:
  - remind me to
  - every morning
  - schedule a
  - automate
  - run this daily
examples:
  - "reorder milk every Monday at 8am"
  - "check device health every evening"
  - "remind me to reorder when stock is low"
---

# Task Scheduler

Turns recurring intent into durable scheduled tasks and workflows.

## Workflow

1. **Clarify the trigger**: time-based (cron/interval) or event-based
   (condition like low stock, low battery). Get the cadence and timezone.
2. **Define the action**: which tool/skill runs, with what parameters.
3. **Set guardrails**: tasks that spend money or send messages must route
   through confirmation (see `payment-guardian`) rather than fire silently.
4. **Confirm + register**: echo the schedule back ("every Mon 08:00 IST → …")
   and create it. Give it a clear name.
5. **Manage**: support list / pause / resume / delete; report next run time.

## Guidelines

- Always state the next execution time after creating a task.
- Prefer idempotent actions; for recurring purchases, require confirmation each
  run unless the user explicitly pre-authorizes a bounded amount.
- Keep recurring tasks visible — never create hidden automation.
