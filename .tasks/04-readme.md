# 04. The README tells it

## What to build

The README documents the self-documenting CLI (there is deliberately no ADR
— grilling Q7 chose the README as the record). A section explains the
Guide: that the binary carries its own agent-facing usage instructions,
the command surface (`askance`, `askance guide`, `askance guide gates`),
the Topic idea and why it exists (token cost paid only when the task
arises), and the one-line installation — the whole integration is a single
sentence in a global CLAUDE.md, quoted verbatim.

Housekeeping the change obsoletes, in the same pass: the README's status
paragraph still says "what is left is the skills that drive the tool", and
the v1 roadmap's stage-06 note says the adoption work landed in
tobico-skills — both now superseded by the Guide living here. Update them to
tell the current truth.

## Acceptance criteria

- [ ] A README section documents the Guide, the Topic surface, and the
      verbatim CLAUDE.md installation line
- [ ] The README status text no longer points at tobico-skills as the
      missing piece
- [ ] The stage-06 note in the v1 roadmap reflects that the agent-facing
      docs moved into this repo
