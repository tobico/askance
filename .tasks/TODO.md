# Nudge

Live updates for the viewer. The pending page currently learns anything only
by a 10-second poll, and nothing at all triggers a fetch when the iOS PWA
returns from background — a new Question Set can sit unseen while the human is
looking right at a stale list. This feature layers freshness per ADR-0005: a
visibility-triggered refetch on PWA resume, a contentless Nudge broadcast from
the server over SSE to open pages, and the service worker relaying every push
to open windows as the same Nudge. The 10-second poll stays as the fallback
and keeps the Liveness badge honest.

## Tasks

- [x] 01: Refetch on reopen — [details](01-refetch-on-reopen.md)
- [ ] 02: The server Nudges — [details](02-server-nudges.md)
- [ ] 03: The page listens — [details](03-page-listens.md)
- [ ] 04: The worker relays — [details](04-worker-relays.md)
