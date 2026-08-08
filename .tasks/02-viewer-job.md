# 02. Viewer job

## What to build

A second job in the same workflow, running the viewer's own checks under
`web/`: install with the lockfile frozen, then **typecheck and test**.

Typecheck is not decoration — a component that only compiles because vite
erases the types would pass vitest and fail nobody, and the generated
`web/src/api/types.ts` is only worth generating if something checks the viewer
against it. The nix check already runs both for the same reason.

Node and pnpm are pinned to what the flake gives (node 22, pnpm 10.28);
`web/pnpm-lock.yaml` is `lockfileVersion: 9.0`, so a pnpm the runner happens to
ship is not good enough. Cache the pnpm store. The suite is 222 tests across 14
files and takes about two seconds, so this job should be quick.

It runs independently of the Rust job — neither produces anything the other
consumes.

## Acceptance criteria

- [ ] The viewer job reports **green** on the PR, alongside the Rust job
- [ ] The install fails rather than silently updating if `pnpm-lock.yaml` is out
      of step with `package.json`
- [ ] A type error that vitest alone would not catch turns the job red
- [ ] A deliberately failing component test turns the job red
- [ ] Node and pnpm versions are pinned in the workflow, matching the flake
