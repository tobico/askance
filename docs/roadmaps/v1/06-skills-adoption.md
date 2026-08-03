# 06. Skills adoption

## Goal

Agents running the tobico-skills question-asking skills route their questions
through askance instead of chat: a `/grilling` session's questions arrive as
a push on the phone, get answered in the PWA, and the session continues from
the Response — with clean fallback to chat when the CLI is absent.

**This stage's work lands in the `tobico-skills` repo** (~/src/tobico-skills),
not here. It is tracked in this roadmap because it's the step that makes
askance useful.

## Decisions in force

- **Adoption is in scope for v1** (grilling session Q13) — the tool is only
  useful once skills route questions to it.
- **Chat fallback is mandatory** — skills must detect whether the `askance`
  CLI is available and fall back to asking in chat exactly as today when it
  isn't. Other machines/users of the skills must be unaffected.
- **The question grammar is canonical at the tobico-skills repo root**,
  synced into each skill via `bin/question-grammar.sh sync`; per-skill copies
  are generated and drift-checked. Any grammar amendment must go through that
  mechanism, not per-skill edits.
- **The wait is a background shell command** ([ADR-0001](../../adr/0001-blocking-cli-for-agent-integration.md))
  — in Claude Code, `askance ask` runs via Bash `run_in_background` so the
  harness wakes the agent on delivery; the instructions must say this
  explicitly so agents don't block a foreground tool call for hours.
- The grammar's semantics don't change — labels, ★, completeness rules are
  identical whether questions travel via chat or askance. Askance's schema
  was designed as a direct encoding precisely so this stage is a transport
  change, not a semantic one.

## Proposed tasks (provisional)

1. **Grammar amendment** — add a transport section to the canonical
   QUESTION-GRAMMAR.md (use askance when available: how to author the YAML
   set, run the CLI in background, interpret the YAML Response; chat
   otherwise) and sync to all skills.
   - `bin/question-grammar.sh check` passes
   - Reply-grammar section still valid for the chat fallback
2. **Skill-by-skill sweep** — grilling, confirm, to-tasks, to-roadmap,
   next-task, domain-modeling: any skill-specific question instructions
   updated to be transport-aware (e.g. pacing/batching guidance applies to
   sets, not turns).
   - A `/grilling` session with the CLI present asks zero questions in chat
3. **End-to-end validation** — run a real grilling session through the phone
   PWA; capture rough edges as issues in the askance repo.
   - One full ask→push→answer→continue cycle observed

## Re-verify at start

- The actual `askance ask` CLI contract as shipped (flags, env var name,
  exact YAML shapes) — the grammar text must quote it verbatim.
- Whether tobico-skills' skill set changed since planning (new
  question-asking skills to sweep).
- How harnesses other than Claude Code handle long-running background
  commands, if any others are in use by then.
