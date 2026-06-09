---
name: web-research
description: Use when the user asks to look something up online, research a topic, check a current fact, compare products or prices, or gather sources from the internet. Searches the web and synthesizes a cited answer.
tags: [web, research, internet, search, browse, fetch, sources, news, compare]
triggers:
  - look up
  - search the web
  - find online
  - what's the latest
  - research
  - compare prices
examples:
  - "look up the current price of a Raspberry Pi 5"
  - "research the best on-device embedding models in 2026"
  - "what's the latest news on X?"
---

# Web Research

Answers questions that need fresh, external information by searching the web
and grounding the answer in real sources.

## Workflow

1. **Classify the request**: a quick fact lookup, a multi-source comparison, or
   a deep research task. Pick the lightest tool path that satisfies it.
2. **Search**: call the `web_search` tool with a focused query. Prefer specific
   queries over broad ones; refine once if the first results are weak.
3. **Read sources**: when a result needs the full page, fetch it with the
   `web_fetch` tool and extract the relevant passage (don't rely on the snippet
   alone for anything important).
4. **Synthesize**: write a concise answer. Cite 2–5 sources inline by name/URL,
   and note the date/freshness of time-sensitive facts.
5. **Be honest about gaps**: if results conflict or are thin, say so and present
   the best-supported view rather than overstating certainty.

## Guidelines

- Lead with a one-line TL;DR, then key findings, then a short sources list.
- Never fabricate a URL or a quote — only cite what was actually retrieved.
- Respect the safety policy: do not fetch credential, payment, or login-gated
  pages, and decline obviously unsafe destinations.
- Keep retrieved context tight; quote sparingly and attribute every quote.
