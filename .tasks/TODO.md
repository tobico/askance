# Answering web UI

The human's interface to Askance: open the server in a browser, see the
pending Question Sets, open one, read its Preface, review the attached Diff,
answer the Questions, and submit — unblocking the waiting CLI. Replaces the
stage 01 curl round trip.

Built as Leptos with full cargo-leptos hydration from the start (decided at
planning: stages 03/04 need real client-side interactivity, so we set up the
canonical layout now rather than retrofit it). One binary serves both the
agent API routes and the UI. Responsive from the start — this UI becomes the
phone experience in stage 04.

Roadmap stage: [02: Answering web UI](docs/roadmaps/v1/02-answering-web-ui.md)

## Tasks

- [x] 01: Leptos SSR skeleton + pending list — [details](01-leptos-skeleton-pending-list.md)
- [ ] 02: Set view: render the ask — [details](02-set-view-render.md)
- [ ] 03: Answer state + submit with unanswered warning — [details](03-answer-state-submit.md)
- [ ] 04: Diff viewer — [details](04-diff-viewer.md)
