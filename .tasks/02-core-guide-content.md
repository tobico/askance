# 02. The core Guide complete

## What to build

The core Guide grows to everything any ask needs, moved at its current depth
from tobico-skills' canonical QUESTION-GRAMMAR.md — nothing silently
trimmed. On top of task 01's sections it gains:

- **Question labels** — `Qn` numbered monotonically across the session,
  sub-question letters, `.N` Options, the ★ Recommendation, two levels
  maximum.
- **Pacing** — complexity budgets per Set, batching independent questions,
  going sequential only for genuine dependencies, enumerate-vs-defer.
  Written Set-only: the per-chat-turn column and every chat-pacing remark
  are gone with the chat fallback.
- **Authoring the Set** — the YAML mapping from the labels, the Preface
  rules (lead with the bottom line, keep it short), block scalars for prose,
  never supplying `project`/`branch`/`diff`, and written-to-be-glanced:
  Diagrams for structure, comparison tables for multi-axis trade-offs,
  bolding the load-bearing phrases.
- **A "Required topic guides" contract section near the top** — the Topics
  are part of the Guide, split out only to save reading them before their
  task arises; before authoring a confirmation gate (any Set asking approval
  to proceed, commit, or ship) the agent MUST run `askance guide gates`
  first. Written so a Topic never reads as optional.
- **The failure note** — if the server is unreachable, report the failure to
  the human and wait for instructions. No structured chat protocol, no
  detection ritual, no reply grammar.

Same adaptation rules as task 01: "the human" voice, no skill names, Claude
Code as a named harness example, no chat fallback anywhere.

## Acceptance criteria

- [ ] Bare `askance` prints a core Guide covering labels, Set-only pacing,
      authoring, the contract, running, and reading — every area the plan
      assigned to core
- [ ] The "Required topic guides" section sits near the top and uses
      contract language (MUST), naming the gate trigger precisely
- [ ] No sentence in the Guide describes asking in chat, detecting the
      transport, or a chat reply grammar
- [ ] The Guide reads standalone: no "I"/"me" for the human, no
      tobico-skills skill names
