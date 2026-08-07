# Public release roadmap

Turns Askance from a private checkout into an installable open-source
project: one binary per platform built by GitHub Actions, a flake that
installs the released binary, an adoption-first README, and an Update Notice
in the UI. The decisions were made in a grilling session on 2026-08-07 and
are recorded in [ADR-0004](../../adr/0004-single-binary-distribution.md) and
[CONTEXT.md](../../../CONTEXT.md); each brief carries the rest of that
session's rationale.

Each stage is one `/to-tasks` feature (one branch, one review unit). Start
the next one with `/next-stage` in a fresh session. Task chunkings inside
briefs are provisional — re-grounded against the codebase when the stage
starts.

Dependencies: 03 needs 01 (the asset shape assumes one binary), 04 needs 03
(a manifest and a real release to read), 06 comes last because it documents
everything before it. 02 and 05 are reorderable — 02 is placed early so every
later stage lands with checks; 05 only needs a repo with releases to poll.
**The repo flips public before stage 03** — release assets on a private repo
need auth to download, so the pipeline cannot be exercised for real until
then; the history audit (2026-08-07) cleared all of it for publication.

## Stages

- [ ] 01: One binary — [brief](01-one-binary.md) *(in progress — `one-binary`)*
- [ ] 02: CI — [brief](02-ci.md)
- [ ] 03: Release pipeline — [brief](03-release-pipeline.md)
- [ ] 04: Binary flake — [brief](04-binary-flake.md)
- [ ] 05: Update Notice — [brief](05-update-notice.md)
- [ ] 06: Adoption docs and go-live — [brief](06-adoption-docs.md)
