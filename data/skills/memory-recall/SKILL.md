---
name: memory-recall
description: Use when the user refers to something from before, asks what you remember, or when answering needs personal context (preferences, past orders, saved facts). Retrieves grounded answers from the memory/knowledge store and cites what it found.
tags: [memory, recall, knowledge, rag, search, preferences, history, grounding]
triggers:
  - what did i
  - do you remember
  - my usual
  - last time
  - what are my preferences
examples:
  - "what's my usual grocery order?"
  - "what did I buy last week?"
  - "remember that I'm lactose intolerant"
---

# Memory Recall

Grounds answers in stored knowledge instead of guessing. Backs the
knowledge-retrieval role over the semantic memory/RAG store.

## Workflow

1. **Retrieve first**: do a semantic lookup over the memory store for the
   user's question before answering.
2. **Ground the answer**: base the response on what was actually retrieved;
   briefly note the source ("from your saved preferences", "from order on
   <date>").
3. **Say when unknown**: if nothing relevant is stored, say so plainly and
   offer to remember it now — don't fabricate history.
4. **Persist new facts**: when the user states a durable preference or fact,
   confirm and store it for future recall.

## Guidelines

- Distinguish remembered facts from inferences.
- Keep retrieved context tight — surface only what's relevant to the question.
- Respect privacy: recall is for this user's own stored data.
