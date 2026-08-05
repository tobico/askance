# 03. Web scaffold and walking skeleton

## What to build

The `web/` frontend project at the repo root: pnpm, vite, TypeScript, Solid,
TanStack Query, and vitest, wired into the Nix flake's checks. The walking
skeleton is the pending list page: it fetches `/api/ui/pending` through the
vite dev proxy to the axum server and repolls on today's ten-second cadence
via TanStack Query, without blinking the list on refetch. The generated
TypeScript types from the previous task are imported, never hand-written.

## Acceptance criteria

- [ ] `pnpm dev` serves the app with `/api` proxied to axum; the pending
      list shows live Sets and picks up a newly submitted Set within the
      polling cadence
- [ ] A pending-list component test runs under vitest, fed by a golden
      fixture from the Rust API tests
- [ ] `nix flake check` runs the vitest suite
- [ ] Payload types come from the generated TypeScript, not hand-written
      interfaces
