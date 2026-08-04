# 05. Grammar guidance

## What to build

The agent-side half, in the sibling repo `~/src/tobico-skills`, where the
question grammar is canonical at the repo root and synced into each skill.

Amend *Authoring the Set* with glance-ability guidance:

- Lead the Preface with the bottom line; the context that justifies it comes
  after.
- Keep the Preface short — decision-specific context belongs in the Question
  that needs it, so a Question can often be answered without reading the
  Preface at all.
- Prefer a mermaid fence over prose when describing relationships, flows, or
  state; keep diagrams small (roughly ten nodes) so they stay legible on a
  phone. Diagrams degrade to their source text, so they're safe to use even
  against an older viewer.
- Prefer a comparison table for multi-option trade-offs.
- Bold the load-bearing phrases so skimming works.

Amend *Confirmation gates* with the stronger push: include a structure
diagram at every gate unless the change is trivial (a few files, no new
relationships); diagram the delta rather than the whole system — the
components the change touches and their relationships, tagged with the
viewer's semantic classes (`new` / `modified` / `removed`); a before/after
pair only when the change reshapes existing structure; a sequence diagram
when the point is a new runtime flow.

Run the repo's sync script so every skill's copy picks the amendment up, and
follow that repo's own conventions for landing changes.

## Acceptance criteria

- [ ] Canonical grammar carries the authoring and gate guidance above
- [ ] `bin/question-grammar.sh check` passes (all synced copies current)
- [ ] Guidance wording is gentle for Sets, stronger for gates, matching the
      decisions on record
