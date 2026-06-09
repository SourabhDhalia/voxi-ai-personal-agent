# F2 — SafetyGuard Hardening

**Status:** ✅ Implemented & test-verified
**File:** `src/voxi/src/core/safety_guard.rs`
**Skill lens:** secure-code-guardian, security-reviewer

## Problem
The original `SafetyGuard` enforced command safety with a 5-entry,
case-sensitive substring denylist (`args.contains("rm -rf /")`, …). Trivially
bypassed by:
- whitespace: `rm  -rf /`
- flag reorder: `rm -fr /`
- case: `RM -RF /`
- long flags: `rm --recursive --force /`
- alternatives: `find / -delete`, `curl … | bash`
- argument splitting across structured fields

Additionally, `check_prompt_injection` was defined but never called (dead
safety control).

## Change
1. **Normalization** — `normalize()` lowercases and collapses whitespace before
   matching, so trivial obfuscation collapses to one canonical form.
2. **Regex pattern set** — 13 curated dangerous-command patterns
   (`DANGEROUS_PATTERNS`) compiled once into a `regex::RegexSet`, resistant to
   flag reordering and matching `rm -rf`/`mkfs`/`dd if=`/raw-disk writes/
   shutdown/reboot/fork-bomb/recursive-chmod/`find -delete`/pipe-to-shell.
3. **Structured-arg scanning** — `scan_value()` collects all string leaves of a
   JSON arg object, scans each, then scans the concatenation so a command split
   across fields (`{"cmd":"rm","flags":"-rf","path":"/"}`) is still caught.
4. **Allowlist mode** — `allow_tool()`; once any tool is allow-listed the guard
   denies every tool not listed (least privilege).
5. **Config** — `load_config` now also reads `allowed_tools` and operator-supplied
   `dangerous_patterns` (invalid custom regexes are skipped, never panic).

Public API preserved (`check_tool`, `check_tool_call`, `check_prompt_injection`,
`block_tool`, `is_blocked`, `status_json`, `load_config`).

## Tests
10 new regression tests assert each bypass vector is now blocked and that clean
commands (`ls -la /tmp`, `git status`, `grep -r …`) still pass. All existing
tests retained.

## Verification
`./deploy.sh --test` → all 21 `safety_guard` tests pass; workspace green.

## Follow-ups
- Wire `check_prompt_injection` into the prompt-intake path (still unused).
- Add a standing red-team `tests/system/*.json` bypass suite.
- Longer term: route through the canonical `vclaw-*` `policy_engine`
  (capability tokens) — see ADR-0001.
