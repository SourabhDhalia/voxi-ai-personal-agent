---
name: skill-author
description: Use when the user wants to teach the agent a new repeatable capability, or after completing a useful multi-step task worth saving for reuse. Drafts a clean, secret-free SKILL.md and registers it as a new runtime skill.
tags: [skill, self-improvement, automation, authoring, capability, reuse, meta]
triggers:
  - remember how to do this
  - make this a skill
  - save this workflow
  - teach you to
  - turn this into a skill
examples:
  - "save these steps as a reusable skill"
  - "teach yourself to do my weekly restock"
---

# Skill Author

A meta-skill: turns a successful, repeatable workflow into a new runtime skill.
Backs the skill-manager role and the opt-in self-improvement gate.

## Workflow

1. **Capture the pattern**: summarize the goal and the ordered steps/tools that
   achieved it. Generalize the specifics (parameters become placeholders).
2. **Write the SKILL.md**: include frontmatter — `name` (kebab-case),
   `description` written as a *trigger* ("Use when…"), `tags`, `triggers`,
   `examples` — and a concise body with the steps and guardrails.
3. **Redact secrets**: strip anything resembling a token, key, password, OTP,
   full card/UPI id, or value pulled from environment/config. Never bake
   credentials or private data into a skill.
4. **Confirm before saving**: show the draft and ask for approval. Save to the
   user-level skills directory on a yes.
5. **Verify**: confirm the new skill is discoverable and its description
   triggers on the intended phrasing.

## Quality bar

- Description must read as a trigger, not a summary.
- Keep skills small and single-purpose; one capability per file.
- Capture the first known failure mode as a guardrail line.
