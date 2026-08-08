# CI

Every push to `main` and every pull request runs the project's checks on
GitHub Actions, so the later, riskier stages of the public-release roadmap land
onto a guarded `main`. The repo has no `.github/` at all today; this stage
creates it.

Three jobs, one per task, all currently green locally: a **Rust job** (test,
clippy, rustfmt, and a guard on the ts-rs–generated TypeScript), a **viewer
job** (typecheck and vitest), and a **nix job** (`nix flake check`, the only
thing that boots the NixOS VM test) which runs on pushes to `main` only, so
branch feedback stays fast.

Roadmap stage: [02: CI](docs/roadmaps/public-release/02-ci.md)

## Verifying these tasks

The workflow triggers on pushes to `main` and on pull requests — so it does
**not** run on this branch until a PR exists. Task 01 therefore pushes the
branch and opens the draft PR itself, and every later task pushes to that same
PR to watch the run. A local `act`-style dry run is not a substitute: pinned
action versions and runner images are exactly what is being verified.

Because the PR is already open, the finish sequence in
`docs/agents/git-workflow.md` should **update** it (`gh pr edit`) rather than
run `gh pr create`, which fails when a PR exists.

## Tasks

- [x] 01: Rust job — [details](01-rust-job.md)
- [x] 02: Viewer job — [details](02-viewer-job.md)
- [x] 03: Nix job — [details](03-nix-job.md)
