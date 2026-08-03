# Askance v1 roadmap

Realizes [docs/PLAN.md](../../PLAN.md): a single-user Leptos web service plus
blocking CLI through which coding agents put Question Sets to a human and wait
for the Response — answerable from any tailnet device as a PWA with push.
Decisions are recorded in the plan, [CONTEXT.md](../../../CONTEXT.md), and
[ADR-0001](../../adr/0001-blocking-cli-for-agent-integration.md).

Each stage is one `/to-tasks` feature (one branch, one review unit). Start the
next one with `/next-stage` in a fresh session. Task chunkings inside briefs
are provisional — re-grounded against the codebase when the stage starts.

Dependencies: 02 → 01, 03 → 02, 04 → 02. Stage 05 only needs 01 and is
reorderable to any point after it. Stage 06 needs the tool usable end-to-end
(after 04 for the full experience; workable after 02). Stages 03 and 04 could
swap if push notifications become urgent before conveniences.

## Stages

- [x] 01: API core + CLI — [brief](01-api-core-and-cli.md)
- [ ] 02: Answering web UI — [brief](02-answering-web-ui.md) *(in progress, branch `answering-web-ui`)*
- [ ] 03: Answering conveniences — [brief](03-answering-conveniences.md)
- [ ] 04: PWA + push — [brief](04-pwa-and-push.md)
- [ ] 05: Nix packaging + deployment — [brief](05-nix-packaging.md)
- [ ] 06: Skills adoption — [brief](06-skills-adoption.md)
