# P2 — OpenRouter LLM Backend

**Status:** ✅ Implemented & test-verified
**Files:** `src/voxi/src/llm/openai.rs`, `src/voxi/src/llm/backend.rs`
**Skill lens:** rust-engineer, architecture-designer
**Gap closed:** parity with Hermes Agent (200+ models) / Kilo Code (500+ models)

## Problem
Voxi supported only `anthropic`, `openai`, `gemini`, `ollama`, and `plugin`
backends. The leading peer agents advertise access to hundreds of models via
OpenRouter. Voxi had no such aggregator backend.

## Insight
OpenRouter is **OpenAI-compatible** (`/chat/completions`, Bearer auth). Voxi's
`OpenAiBackend` already abstracts `endpoint` + `model` and already does base-URL
switching for `xai`/grok. So OpenRouter is a *configuration arm*, not a new
backend implementation.

## Change
- `OpenAiBackend::new()` — added arm:
  `"openrouter" => ("https://openrouter.ai/api/v1", "openai/gpt-4o")`.
  The default model is overridable via config `model`
  (e.g. `anthropic/claude-3.5-sonnet`, `google/gemini-pro-1.5`).
- `backend::create_backend()` — added `"openrouter"` to the
  `"openai" | "xai"` arm so the factory constructs an `OpenAiBackend`.
- `initialize()` already honored `api_key`/`model`/`endpoint` overrides, so no
  further wiring was required; no provider allowlist rejects the new name.

## Usage
```json
{ "provider": "openrouter", "api_key": "sk-or-…",
  "model": "anthropic/claude-3.5-sonnet" }
```

## Tests
- `backend::tests`: `openrouter_backend_is_created`,
  `openai_compatible_providers_resolve`, `unknown_provider_is_none`.
- `openai::tests`: `openrouter_uses_openrouter_endpoint`,
  `endpoint_overridable_via_config`.

## Verification
`./deploy.sh --test` → all 5 new tests pass; workspace 518 passing, 0 failing.

## Follow-ups
- Optional: send OpenRouter's `HTTP-Referer` / `X-Title` headers for dashboard
  attribution.
- Surface `openrouter` in any provider picker UI / `llm_config_store` examples.
