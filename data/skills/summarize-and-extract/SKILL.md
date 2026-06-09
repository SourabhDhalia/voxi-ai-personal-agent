---
name: summarize-and-extract
description: Use when the user provides a long document, transcript, article, chat log, or URL and wants a summary, the key points, action items, or specific fields extracted. Condenses long content while preserving facts.
tags: [summarize, summary, extract, tldr, key-points, action-items, distill, notes]
triggers:
  - summarize
  - tl;dr
  - key points
  - action items
  - pull out the
  - give me the gist
examples:
  - "summarize this thread into action items"
  - "extract every date and amount from this document"
  - "give me the key points of this article"
---

# Summarize & Extract

Turns long source material into a tight, faithful summary or a structured
extraction. No new tools required — pure reasoning over provided content.

## Workflow

1. **Identify the source and goal**: raw text, a file, or a URL; and whether the
   user wants an executive summary, bullet key-points, or structured extraction
   (entities, dates, amounts, decisions, action items).
2. **Chunk if needed**: for content beyond the context budget, summarize in
   chunks, then combine the chunk summaries (map-reduce). Preserve numbers and
   named entities verbatim through every step.
3. **Produce the requested shape**:
   - Summary → one-line TL;DR followed by a short prose or bullet summary.
   - Extraction → a clean, machine-readable list/table of the requested fields.
4. **Preserve fidelity**: never invent facts, figures, or names that aren't in
   the source. Flag anything ambiguous rather than guessing.

## Guidelines

- A summary must be substantially shorter than and different from the source.
- Keep dates, prices, and proper nouns exact.
- For action items, capture owner + task + due date when present.
- Offer an expandable "detailed view" only if the user wants more depth.
