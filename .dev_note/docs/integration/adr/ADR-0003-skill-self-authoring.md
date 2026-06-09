# ADR-0003 — Skill self-authoring loop + `agentskills.io` compatibility

**Status:** Proposed
**Gap:** P3 — Voxi has skill *management* but no "solve → write reusable skill" loop
**Peer evidence:** Hermes Agent's signature: writes a reusable skill document
when it solves a hard problem; skills are searchable/shareable on the
`agentskills.io` open standard.

## Context
Voxi already has skill plumbing: `clawhub_client`, `skill_capability_manager`,
`auto_skill_agent`, `data/skills/*`, and a `skill_author` skill. What's missing
is the *self-improvement loop* — turning a successful multi-step solution into a
durable, retrievable SKILL.md — and conformance to a portable open format.

## Decision
1. After a task completes successfully via N tool steps, an optional
   `SkillDistiller` proposes a SKILL.md (name, trigger, steps) and stores it as
   `pending` (human/agent approval gate already exists via
   `/api/skills/approve`).
2. Adopt the `agentskills.io` SKILL.md schema so Voxi skills are interoperable
   with the wider ecosystem (import/export).
3. Retrieval: index skills (reuse the existing memory SQLite index) so the
   distilled skill is surfaced on similar future prompts.

## Consequences
- (+) Compounding capability (Hermes-class); ecosystem interop.
- (+) Reuses the existing approval + storage paths.
- (−) Distillation quality needs guardrails (don't memorize one-offs); gate on
  task success + step-count threshold.

## Validation
`tests/system/skill_self_author_runtime_contract.json`: simulate a solved task,
assert a `pending` skill is proposed and retrievable after approval.
