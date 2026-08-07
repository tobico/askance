# 05. Update Notice

## Goal

A server that has run for a day past a new release shows the human an Update
Notice — a banner above the pending list linking the updating instructions —
and an env var turns the whole check off. Nothing is installed automatically.

## Decisions in force

- **Update Notice** is a CONTEXT.md term: informs only, never installs.
- **The poll runs server-side, daily, against GitHub's latest-release API**
  — chosen over the browser asking GitHub directly (every device would
  phone home, and the tailnet UI should not depend on GitHub reachability)
  and over a startup-only check (stale on a long-running service). Held in
  memory; a failed poll costs nothing and is retried next cycle.
- **An opt-out was explicitly promised** — open-source users expect a
  switch on anything that phones home. An env var on the server disables
  the check entirely (name it at stage start, alongside the server's
  existing configuration style).
- The banner links to the README's updating section — stage 06 writes it;
  use the anchor agreed there (a stable `#updating` anchor on the repo
  README is the working assumption).
- Version comparison is the compiled-in crate version against the latest
  release tag; treat "newer" conservatively (a plain semver compare, no
  pre-release cleverness needed at 0.x).

## Proposed tasks (provisional)

1. **Poller and version state** — the server learns the latest release and
   exposes "an update exists" through the UI API the pending list already
   fetches. Accepts: with a mocked latest release above the crate version
   the API says so; with the env var set no request is ever made.
2. **The banner** — pending list shows the Notice when the API says update,
   links the instructions, disappears when current. Accepts: vitest
   component coverage both ways.

## Re-verify at start

- The `/api/ui/` response the pending list polls (shape and where a field
  lands in the TypeScript types via ts-rs).
- The repo is public — unauthenticated API polls; one request a day is far
  under any rate limit, but confirm the endpoint shape then.
- Stage 06's anchor for updating instructions, if it has landed; otherwise
  note the link target as provisional.
