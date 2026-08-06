# Self-documenting CLI

Askance stops depending on tobico-skills by shipping its own agent-facing
Guide in the binary: `askance guide` (and bare `askance`) print everything an
agent needs to ask well — authoring Sets, running the CLI, reading the
Response — with a `gates` Topic fetched only when an approval gate is at
hand. The question grammar's canonical home moves from tobico-skills into
this repo; tobico-skills loses every reference to askance, grammar,
transport, and pacing; and a single line in the global CLAUDE.md becomes the
whole installation. The chat fallback feature is removed — no chat topic, no
detection protocol, no reply grammar.

Decided in grilling Question Sets 179–184 (see the Archive). Landing is two
branches, one feature: this repo's `self-documenting-cli`, and a matching
feature branch in ~/src/tobico-skills, landed together with each repo's
finish sequence.

## Tasks

- [x] 01: The Guide takes the stage — [details](01-guide-command.md)
- [x] 02: The core Guide complete — [details](02-core-guide-content.md)
- [x] 03: The gates Topic — [details](03-gates-topic.md)
- [x] 04: The README tells it — [details](04-readme.md)
- [ ] 05: The switch-over — [details](05-switch-over.md)
